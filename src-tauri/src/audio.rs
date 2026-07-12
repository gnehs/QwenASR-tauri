use std::{
    ffi::OsString,
    fs,
    io::ErrorKind,
    ops::Range,
    path::{Path, PathBuf},
    process::Command,
};

use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::FfmpegStatus;

pub const ASR_SAMPLE_RATE: u32 = 16_000;
const MIN_ASR_SAMPLES: u32 = 320;
const TEMP_AUDIO_DIR: &str = "qwenasr-tauri";
const FFMPEG_COMMAND: &str = "ffmpeg";
const FFMPEG_INSTALL_HINT: &str =
    "未偵測到 FFmpeg。請在終端機執行 `brew install ffmpeg` 後重開 QwenASR Studio。";

#[cfg(target_os = "macos")]
const MACOS_FFMPEG_PATHS: &[&str] = &[
    "/opt/homebrew/bin/ffmpeg",
    "/usr/local/bin/ffmpeg",
    "/opt/local/bin/ffmpeg",
    "/usr/local/ffmpeg/bin/ffmpeg",
];

pub struct PreparedAudio {
    normalized_path: PathBuf,
}

impl PreparedAudio {
    fn new(normalized_path: PathBuf) -> Self {
        Self { normalized_path }
    }

    pub fn inference_path(&self) -> &Path {
        &self.normalized_path
    }
}

impl Drop for PreparedAudio {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.normalized_path);
    }
}

pub fn ffmpeg_status() -> FfmpegStatus {
    if let Some(ffmpeg) = resolve_ffmpeg() {
        return FfmpegStatus {
            available: true,
            version: Some(ffmpeg.version),
        };
    }

    FfmpegStatus {
        available: false,
        version: None,
    }
}

pub fn prepare_audio_for_asr(audio_path: &str) -> AppResult<PreparedAudio> {
    let original_path = PathBuf::from(audio_path);
    if !original_path.is_file() {
        return Err(AppError::Transcription(
            "Selected audio file could not be found.".into(),
        ));
    }

    match normalize_with_ffmpeg(&original_path) {
        Ok(normalized_path) => Ok(PreparedAudio::new(normalized_path)),
        Err(FfmpegError::Unavailable) => Err(AppError::Transcription(FFMPEG_INSTALL_HINT.into())),
        Err(FfmpegError::Failed(message)) => Err(AppError::Transcription(message)),
    }
}

pub fn read_normalized_i16(path: &Path) -> AppResult<Vec<i16>> {
    let reader = hound::WavReader::open(path).map_err(|error| {
        AppError::Transcription(format!("Prepared audio could not be read as WAV: {error}"))
    })?;
    validate_asr_wav(reader.spec(), reader.duration()).map_err(|error| match error {
        FfmpegError::Unavailable => AppError::Transcription("FFmpeg is unavailable.".into()),
        FfmpegError::Failed(message) => AppError::Transcription(message),
    })?;

    reader
        .into_samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            AppError::Transcription(format!(
                "Prepared audio contains invalid PCM samples: {error}"
            ))
        })
}

/// Reads only the requested mono sample range from a normalized WAV.
///
/// Keeping alignment inputs on disk avoids retaining every batch item's PCM
/// buffer in memory while the ASR model is active.
pub fn read_normalized_i16_range(path: &Path, range: Range<usize>) -> AppResult<Vec<i16>> {
    let mut reader = hound::WavReader::open(path).map_err(|error| {
        AppError::Transcription(format!("Prepared audio could not be read as WAV: {error}"))
    })?;
    let duration = reader.duration() as usize;
    validate_asr_wav(reader.spec(), reader.duration()).map_err(|error| match error {
        FfmpegError::Unavailable => AppError::Transcription("FFmpeg is unavailable.".into()),
        FfmpegError::Failed(message) => AppError::Transcription(message),
    })?;

    if range.start > range.end || range.end > duration {
        return Err(AppError::Transcription(format!(
            "Prepared audio sample range {}..{} exceeds its {}-sample duration.",
            range.start, range.end, duration
        )));
    }

    reader.seek(range.start as u32).map_err(|error| {
        AppError::Transcription(format!("Prepared audio sample seek failed: {error}"))
    })?;
    reader
        .samples::<i16>()
        .take(range.end - range.start)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            AppError::Transcription(format!(
                "Prepared audio contains invalid PCM samples: {error}"
            ))
        })
}

pub fn samples_to_ms(samples: usize) -> u64 {
    ((samples as f64 / ASR_SAMPLE_RATE as f64) * 1000.0).round() as u64
}

fn normalize_with_ffmpeg(input: &Path) -> Result<PathBuf, FfmpegError> {
    let output = normalized_audio_path().map_err(|error| {
        FfmpegError::Failed(format!(
            "Could not create a temporary audio file for normalization: {error}"
        ))
    })?;
    let ffmpeg = resolve_ffmpeg().ok_or(FfmpegError::Unavailable)?;

    let command_output = Command::new(&ffmpeg.program)
        .args(ffmpeg_normalize_args(input, &output))
        .output()
        .map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                FfmpegError::Unavailable
            } else {
                FfmpegError::Failed(format!("Could not run FFmpeg: {error}"))
            }
        })?;

    if !command_output.status.success() {
        let _ = fs::remove_file(&output);
        return Err(FfmpegError::Failed(format!(
            "FFmpeg failed to normalize the selected file: {}",
            summarize_ffmpeg_stderr(&command_output.stderr, input, &output)
        )));
    }

    if let Err(error) = validate_normalized_wav(&output) {
        let _ = fs::remove_file(&output);
        return Err(error);
    }

    Ok(output)
}

fn resolve_ffmpeg() -> Option<ResolvedFfmpeg> {
    resolve_ffmpeg_from(ffmpeg_candidates())
}

fn resolve_ffmpeg_from(candidates: Vec<PathBuf>) -> Option<ResolvedFfmpeg> {
    candidates.into_iter().find_map(|program| {
        ffmpeg_version(&program).map(|version| ResolvedFfmpeg { program, version })
    })
}

fn ffmpeg_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_unique_candidate(&mut candidates, PathBuf::from(FFMPEG_COMMAND));

    #[cfg(target_os = "macos")]
    for path in MACOS_FFMPEG_PATHS {
        push_unique_candidate(&mut candidates, PathBuf::from(path));
    }

    candidates
}

fn push_unique_candidate(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn ffmpeg_version(program: &Path) -> Option<String> {
    let output = Command::new(program).arg("-version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(stdout.lines().next().unwrap_or(FFMPEG_COMMAND).to_string())
}

struct ResolvedFfmpeg {
    program: PathBuf,
    version: String,
}

fn normalized_audio_path() -> std::io::Result<PathBuf> {
    let directory = std::env::temp_dir().join(TEMP_AUDIO_DIR);
    fs::create_dir_all(&directory)?;
    Ok(directory.join(format!("{}.wav", Uuid::new_v4())))
}

fn ffmpeg_normalize_args(input: &Path, output: &Path) -> Vec<OsString> {
    let audio_filter = format!("aresample={ASR_SAMPLE_RATE},apad=whole_len={MIN_ASR_SAMPLES}");
    let sample_rate = ASR_SAMPLE_RATE.to_string();
    let mut args = ["-hide_banner", "-loglevel", "error", "-y", "-i"]
        .into_iter()
        .map(OsString::from)
        .chain([input.as_os_str().to_os_string()])
        .collect::<Vec<_>>();
    args.extend([
        OsString::from("-map"),
        OsString::from("0:a:0"),
        OsString::from("-vn"),
        OsString::from("-af"),
        OsString::from(audio_filter),
        OsString::from("-ac"),
        OsString::from("1"),
        OsString::from("-ar"),
        OsString::from(sample_rate),
        OsString::from("-acodec"),
        OsString::from("pcm_s16le"),
        OsString::from("-f"),
        OsString::from("wav"),
        output.as_os_str().to_os_string(),
    ]);
    args
}

fn validate_normalized_wav(path: &Path) -> Result<(), FfmpegError> {
    let reader = hound::WavReader::open(path).map_err(|error| {
        FfmpegError::Failed(format!(
            "FFmpeg produced an audio file that could not be read as WAV: {error}"
        ))
    })?;
    validate_asr_wav(reader.spec(), reader.duration())
}

fn validate_asr_wav(spec: hound::WavSpec, duration: u32) -> Result<(), FfmpegError> {
    if spec.channels != 1
        || spec.sample_rate != ASR_SAMPLE_RATE
        || spec.sample_format != hound::SampleFormat::Int
        || spec.bits_per_sample != 16
    {
        return Err(FfmpegError::Failed(format!(
            "FFmpeg produced an unsupported WAV format: {} channel(s), {} Hz, {:?}, {} bit.",
            spec.channels, spec.sample_rate, spec.sample_format, spec.bits_per_sample
        )));
    }

    if duration < MIN_ASR_SAMPLES {
        return Err(FfmpegError::Failed(format!(
            "The selected file contains too little audio to transcribe safely after normalization: {} samples at {} Hz.",
            duration,
            ASR_SAMPLE_RATE
        )));
    }

    Ok(())
}

fn summarize_ffmpeg_stderr(stderr: &[u8], input: &Path, output: &Path) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let redacted = stderr
        .replace(&input.to_string_lossy().to_string(), "<input>")
        .replace(&output.to_string_lossy().to_string(), "<output>");
    let summary = redacted
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");

    if summary.is_empty() {
        "no FFmpeg error output was captured".into()
    } else {
        summary
    }
}

enum FfmpegError {
    Unavailable,
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn resolves_ffmpeg_from_absolute_candidate_path() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!("qwenasr-ffmpeg-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let fake_ffmpeg = directory.join(FFMPEG_COMMAND);
        fs::write(&fake_ffmpeg, "#!/bin/sh\necho 'ffmpeg version test'\n").unwrap();
        fs::set_permissions(&fake_ffmpeg, fs::Permissions::from_mode(0o755)).unwrap();

        let resolved =
            resolve_ffmpeg_from(vec![directory.join("missing-ffmpeg"), fake_ffmpeg.clone()])
                .unwrap();

        assert_eq!(resolved.program, fake_ffmpeg);
        assert_eq!(resolved.version, "ffmpeg version test");

        let _ = fs::remove_dir_all(directory);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn includes_common_macos_ffmpeg_install_paths() {
        let candidates = ffmpeg_candidates();

        assert!(candidates.contains(&PathBuf::from("/opt/homebrew/bin/ffmpeg")));
        assert!(candidates.contains(&PathBuf::from("/usr/local/bin/ffmpeg")));
    }

    #[test]
    fn builds_ffmpeg_args_for_qwen_asr_wav_input() {
        let input = Path::new("/tmp/input.m4a");
        let output = Path::new("/tmp/output.wav");
        let args = ffmpeg_normalize_args(input, output)
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "-hide_banner",
                "-loglevel",
                "error",
                "-y",
                "-i",
                "/tmp/input.m4a",
                "-map",
                "0:a:0",
                "-vn",
                "-af",
                "aresample=16000,apad=whole_len=320",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-acodec",
                "pcm_s16le",
                "-f",
                "wav",
                "/tmp/output.wav",
            ]
        );
    }

    #[test]
    fn redacts_input_and_output_paths_from_ffmpeg_errors() {
        let input = Path::new("/Users/example/Private Recording.m4a");
        let output = Path::new("/tmp/qwenasr-tauri/private.wav");
        let summary = summarize_ffmpeg_stderr(
            b"/Users/example/Private Recording.m4a: Invalid data\n/tmp/qwenasr-tauri/private.wav failed",
            input,
            output,
        );

        assert_eq!(summary, "<input>: Invalid data\n<output> failed");
    }

    #[test]
    fn validates_minimum_wav_samples_for_mlx_preprocessing() {
        let path = test_wav_path("short");
        write_test_wav(&path, MIN_ASR_SAMPLES - 1);

        let error = validate_normalized_wav(&path).unwrap_err();
        match error {
            FfmpegError::Failed(message) => {
                assert!(message.contains("too little audio"));
            }
            FfmpegError::Unavailable => panic!("expected validation error"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn accepts_wav_at_mlx_safe_minimum() {
        let path = test_wav_path("minimum");
        write_test_wav(&path, MIN_ASR_SAMPLES);

        assert!(validate_normalized_wav(&path).is_ok());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn reads_only_the_requested_normalized_sample_range() {
        let path = test_wav_path("range");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: ASR_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for sample in 0i16..MIN_ASR_SAMPLES as i16 {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();

        let samples = read_normalized_i16_range(&path, 100..125).unwrap();

        assert_eq!(samples, (100i16..125).collect::<Vec<_>>());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_normalized_sample_ranges_past_the_end() {
        let path = test_wav_path("range-out-of-bounds");
        write_test_wav(&path, MIN_ASR_SAMPLES);

        let error = read_normalized_i16_range(
            &path,
            MIN_ASR_SAMPLES as usize - 1..MIN_ASR_SAMPLES as usize + 1,
        )
        .unwrap_err();

        assert!(error.to_string().contains("exceeds"));
        let _ = fs::remove_file(path);
    }

    fn test_wav_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("qwenasr-{label}-{}.wav", Uuid::new_v4()))
    }

    fn write_test_wav(path: &Path, samples: u32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: ASR_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for _ in 0..samples {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();
    }
}
