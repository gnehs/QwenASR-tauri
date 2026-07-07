use std::{
    ffi::OsString,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::FfmpegStatus;

pub const ASR_SAMPLE_RATE: u32 = 16_000;
const MIN_ASR_SAMPLES: u32 = 320;
const TEMP_AUDIO_DIR: &str = "qwenasr-tauri";

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
    match Command::new("ffmpeg").arg("-version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version = stdout.lines().next().map(|line| line.to_string());
            FfmpegStatus {
                available: true,
                version,
            }
        }
        _ => FfmpegStatus {
            available: false,
            version: None,
        },
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
        Err(FfmpegError::Unavailable) => Err(AppError::Transcription(
            "FFmpeg is required to normalize audio before transcription. Install FFmpeg and try again.".into(),
        )),
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

pub fn write_temp_asr_wav(samples: &[i16]) -> AppResult<PreparedAudio> {
    let output = normalized_audio_path().map_err(|error| {
        AppError::Transcription(format!(
            "Could not create a temporary chunk audio file: {error}"
        ))
    })?;
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: ASR_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&output, spec).map_err(|error| {
        AppError::Transcription(format!("Could not create a chunk WAV file: {error}"))
    })?;

    let min_samples = MIN_ASR_SAMPLES as usize;
    for sample in samples {
        writer.write_sample(*sample).map_err(|error| {
            AppError::Transcription(format!("Could not write chunk audio samples: {error}"))
        })?;
    }
    for _ in samples.len()..min_samples {
        writer.write_sample(0i16).map_err(|error| {
            AppError::Transcription(format!("Could not pad chunk audio samples: {error}"))
        })?;
    }

    writer.finalize().map_err(|error| {
        AppError::Transcription(format!("Could not finalize chunk WAV: {error}"))
    })?;

    Ok(PreparedAudio::new(output))
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

    let command_output = Command::new("ffmpeg")
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
