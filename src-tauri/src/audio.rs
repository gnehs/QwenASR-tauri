use std::path::Path;
use std::process::Command;

use crate::error::{AppError, AppResult};
use crate::models::FfmpegStatus;

const FFMPEG_AUDIO_ARGS: &[&str] = &[
    "-loglevel",
    "error",
    "-i",
    "",
    "-ar",
    "16000",
    "-ac",
    "1",
    "-f",
    "s16le",
    "pipe:1",
];

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

pub fn load_audio_samples(path: &str, allow_ffmpeg: bool) -> AppResult<Vec<f32>> {
    let path_ref = Path::new(path);
    let is_wav = path_ref
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("wav"))
        .unwrap_or(false);

    if is_wav {
        if let Some(samples) = qwen_asr::audio::load_wav(path) {
            return Ok(samples);
        }

        if !allow_ffmpeg {
            return Err(AppError::Transcription(
                "WAV could not be loaded. Enable FFmpeg conversion to normalize it.".into(),
            ));
        }
    }

    if !allow_ffmpeg {
        return Err(AppError::Ffmpeg(
            "This file is not a directly supported 16 kHz mono WAV. Enable FFmpeg conversion."
                .into(),
        ));
    }

    load_with_ffmpeg(path)
}

fn load_with_ffmpeg(path: &str) -> AppResult<Vec<f32>> {
    let mut args = FFMPEG_AUDIO_ARGS.to_vec();
    args[3] = path;

    let output = Command::new("ffmpeg")
        .args(args)
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::Ffmpeg(
                    "FFmpeg was not found. Install it with Homebrew: brew install ffmpeg".into(),
                )
            } else {
                AppError::Ffmpeg(error.to_string())
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Ffmpeg(if stderr.is_empty() {
            "FFmpeg failed to convert the audio.".into()
        } else {
            stderr
        }));
    }

    if output.stdout.len() % 2 != 0 {
        return Err(AppError::Ffmpeg(
            "FFmpeg returned an invalid PCM byte stream.".into(),
        ));
    }

    Ok(output
        .stdout
        .chunks_exact(2)
        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / 32768.0)
        .collect())
}
