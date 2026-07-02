use std::fs;
use std::sync::Mutex;
use std::time::Instant;

use hf_hub::progress::{DownloadEvent, FileStatus, ProgressEvent, ProgressHandler};
use hf_hub::HFClientSync;
use tauri::{AppHandle, Emitter};

use crate::error::{AppError, AppResult};
use crate::models::{find_known_model, DownloadProgress};
use crate::paths::{model_dir, model_status};

pub fn download_model(app: AppHandle, model_id: String) -> AppResult<crate::models::ModelStatus> {
    let model = find_known_model(&model_id)
        .ok_or_else(|| AppError::Model(format!("Unknown model: {model_id}")))?;
    let dir = model_dir(model.id)?;
    fs::create_dir_all(&dir)?;

    let (owner, name) = model
        .repo
        .split_once('/')
        .ok_or_else(|| AppError::Model(format!("Invalid Hugging Face repo: {}", model.repo)))?;
    let client = HFClientSync::new().map_err(|error| AppError::Download(error.to_string()))?;
    let repo = client.model(owner, name);
    let total_files = model.files.len();

    emit_progress(
        &app,
        DownloadProgress {
            model_id: model.id.to_string(),
            state: "starting".into(),
            current_file: None,
            file_index: 0,
            total_files,
            file_bytes_completed: 0,
            file_total_bytes: 0,
            speed_bytes_per_sec: 0.0,
            percent: 0.0,
            message: format!("Preparing {}", model.title),
        },
    );

    for (index, file) in model.files.iter().enumerate() {
        let destination = dir.join(file);
        if destination.exists() {
            emit_progress(
                &app,
                DownloadProgress {
                    model_id: model.id.to_string(),
                    state: "cached".into(),
                    current_file: Some((*file).to_string()),
                    file_index: index + 1,
                    total_files,
                    file_bytes_completed: 1,
                    file_total_bytes: 1,
                    speed_bytes_per_sec: 0.0,
                    percent: 100.0,
                    message: format!("{file} already exists"),
                },
            );
            continue;
        }

        let progress = hf_hub::progress::Progress::new(TauriProgressHandler::new(
            app.clone(),
            model.id,
            file,
            index + 1,
            total_files,
        ));

        repo.download_file()
            .filename((*file).to_string())
            .local_dir(dir.clone())
            .progress(progress)
            .send()
            .map_err(|error| AppError::Download(error.to_string()))?;
    }

    emit_progress(
        &app,
        DownloadProgress {
            model_id: model.id.to_string(),
            state: "complete".into(),
            current_file: None,
            file_index: total_files,
            total_files,
            file_bytes_completed: 1,
            file_total_bytes: 1,
            speed_bytes_per_sec: 0.0,
            percent: 100.0,
            message: format!("{} is ready", model.title),
        },
    );

    model_status(model.id)
}

struct TauriProgressHandler {
    app: AppHandle,
    model_id: String,
    file_name: String,
    file_index: usize,
    total_files: usize,
    last_sample: Mutex<(Instant, u64)>,
}

impl TauriProgressHandler {
    fn new(
        app: AppHandle,
        model_id: &str,
        file_name: &str,
        file_index: usize,
        total_files: usize,
    ) -> Self {
        Self {
            app,
            model_id: model_id.to_string(),
            file_name: file_name.to_string(),
            file_index,
            total_files,
            last_sample: Mutex::new((Instant::now(), 0)),
        }
    }

    fn emit(
        &self,
        state: &str,
        completed: u64,
        total: u64,
        speed_bytes_per_sec: f64,
        message: String,
    ) {
        let percent = if total > 0 {
            (completed as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        emit_progress(
            &self.app,
            DownloadProgress {
                model_id: self.model_id.clone(),
                state: state.into(),
                current_file: Some(self.file_name.clone()),
                file_index: self.file_index,
                total_files: self.total_files,
                file_bytes_completed: completed,
                file_total_bytes: total,
                speed_bytes_per_sec,
                percent,
                message,
            },
        );
    }
}

impl ProgressHandler for TauriProgressHandler {
    fn on_progress(&self, event: &ProgressEvent) {
        match event {
            ProgressEvent::Download(DownloadEvent::Start { total_bytes, .. }) => {
                self.emit(
                    "downloading",
                    0,
                    *total_bytes,
                    0.0,
                    format!("Downloading {}", self.file_name),
                );
            }
            ProgressEvent::Download(DownloadEvent::Progress { files }) => {
                for file in files {
                    let mut last = match self.last_sample.lock() {
                        Ok(guard) => guard,
                        Err(_) => return,
                    };
                    let now = Instant::now();
                    let elapsed = now.duration_since(last.0).as_secs_f64();
                    let delta = file.bytes_completed.saturating_sub(last.1);
                    let speed = if elapsed > 0.0 {
                        delta as f64 / elapsed
                    } else {
                        0.0
                    };
                    *last = (now, file.bytes_completed);
                    drop(last);

                    let state = match file.status {
                        FileStatus::Started | FileStatus::InProgress => "downloading",
                        FileStatus::Complete => "fileComplete",
                    };
                    self.emit(
                        state,
                        file.bytes_completed,
                        file.total_bytes,
                        speed,
                        format!("Downloading {}", file.filename),
                    );
                }
            }
            ProgressEvent::Download(DownloadEvent::AggregateProgress {
                bytes_completed,
                total_bytes,
                bytes_per_sec,
            }) => {
                self.emit(
                    "downloading",
                    *bytes_completed,
                    *total_bytes,
                    bytes_per_sec.unwrap_or(0.0),
                    format!("Downloading {}", self.file_name),
                );
            }
            ProgressEvent::Download(DownloadEvent::Complete) => {
                self.emit(
                    "fileComplete",
                    1,
                    1,
                    0.0,
                    format!("Finished {}", self.file_name),
                );
            }
            _ => {}
        }
    }
}

fn emit_progress(app: &AppHandle, progress: DownloadProgress) {
    let _ = app.emit("model-download-progress", progress);
}
