use std::process::Command;

use crate::models::FfmpegStatus;

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
