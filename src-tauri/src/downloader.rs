use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use hf_hub::progress::{DownloadEvent, FileStatus, ProgressEvent, ProgressHandler};
use hf_hub::repository::RepoTreeEntry;
use hf_hub::{HFClientSync, HFRepositorySync, RepoTypeModel};
use tauri::{AppHandle, Emitter};
use tokenizers::{
    decoders::byte_level::ByteLevel as ByteLevelDecoder, models::bpe::BPE,
    pre_tokenizers::byte_level::ByteLevel, Tokenizer,
};

use crate::error::{AppError, AppResult};
use crate::models::{find_known_model, DownloadProgress, KnownModel};
use crate::paths::{model_dir, model_status};

const TOKENIZER_JSON: &str = "tokenizer.json";
const GENERATED_FILE_WEIGHT_BYTES: u64 = 1;

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
    let size_plan = download_size_plan(&repo, model, &dir)?;

    emit_progress(
        &app,
        DownloadProgress {
            model_id: model.id.to_string(),
            state: "starting".into(),
            current_file: None,
            file_index: 0,
            total_files,
            file_bytes_completed: 0,
            file_total_bytes: size_plan.total_bytes,
            speed_bytes_per_sec: 0.0,
            percent: 0.0,
            message: format!("Preparing {}", model.title),
        },
    );

    for (index, file) in model.files.iter().enumerate() {
        let destination = dir.join(file);
        let file_size = size_plan.file_size(index);
        if destination.exists() {
            emit_progress(
                &app,
                DownloadProgress {
                    model_id: model.id.to_string(),
                    state: "cached".into(),
                    current_file: Some((*file).to_string()),
                    file_index: index + 1,
                    total_files,
                    file_bytes_completed: file_size,
                    file_total_bytes: file_size,
                    speed_bytes_per_sec: 0.0,
                    percent: size_plan.percent(index, file_size, file_size),
                    message: format!("{file} already exists"),
                },
            );
            continue;
        }

        if *file == TOKENIZER_JSON {
            emit_progress(
                &app,
                DownloadProgress {
                    model_id: model.id.to_string(),
                    state: "generating".into(),
                    current_file: Some((*file).to_string()),
                    file_index: index + 1,
                    total_files,
                    file_bytes_completed: 0,
                    file_total_bytes: file_size,
                    speed_bytes_per_sec: 0.0,
                    percent: size_plan.percent(index, 0, file_size),
                    message: "Generating tokenizer.json".into(),
                },
            );
            generate_tokenizer_json(model, &dir)?;
            emit_progress(
                &app,
                DownloadProgress {
                    model_id: model.id.to_string(),
                    state: "fileComplete".into(),
                    current_file: Some((*file).to_string()),
                    file_index: index + 1,
                    total_files,
                    file_bytes_completed: file_size,
                    file_total_bytes: file_size,
                    speed_bytes_per_sec: 0.0,
                    percent: size_plan.percent(index, file_size, file_size),
                    message: "Generated tokenizer.json".into(),
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
            size_plan.completed_before(index),
            file_size,
            size_plan.total_bytes,
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
            file_bytes_completed: size_plan.total_bytes,
            file_total_bytes: size_plan.total_bytes,
            speed_bytes_per_sec: 0.0,
            percent: 100.0,
            message: format!("{} is ready", model.title),
        },
    );

    model_status(model.id)
}

#[derive(Debug, Clone)]
struct DownloadSizePlan {
    file_sizes: Vec<u64>,
    total_bytes: u64,
}

impl DownloadSizePlan {
    fn new(file_sizes: Vec<u64>) -> Self {
        let file_sizes = file_sizes
            .into_iter()
            .map(|size| size.max(1))
            .collect::<Vec<_>>();
        let total_bytes = file_sizes.iter().copied().sum::<u64>().max(1);

        Self {
            file_sizes,
            total_bytes,
        }
    }

    fn file_size(&self, index: usize) -> u64 {
        self.file_sizes
            .get(index)
            .copied()
            .unwrap_or(GENERATED_FILE_WEIGHT_BYTES)
            .max(1)
    }

    fn completed_before(&self, index: usize) -> u64 {
        self.file_sizes.iter().take(index).copied().sum()
    }

    fn percent(&self, index: usize, file_completed: u64, file_total: u64) -> f64 {
        planned_progress_percent(
            self.completed_before(index),
            self.file_size(index),
            self.total_bytes,
            file_completed,
            file_total,
        )
    }
}

fn download_size_plan(
    repo: &HFRepositorySync<RepoTypeModel>,
    model: KnownModel,
    dir: &Path,
) -> AppResult<DownloadSizePlan> {
    let remote_files = model
        .files
        .iter()
        .copied()
        .filter(|file| *file != TOKENIZER_JSON)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let remote_sizes = remote_file_sizes(repo, &remote_files)?;

    let file_sizes = model
        .files
        .iter()
        .map(|file| {
            let local_size = dir
                .join(file)
                .metadata()
                .map(|metadata| metadata.len())
                .ok();

            if *file == TOKENIZER_JSON {
                return Ok(local_size.unwrap_or(GENERATED_FILE_WEIGHT_BYTES));
            }

            remote_sizes
                .get(*file)
                .copied()
                .or(local_size)
                .ok_or_else(|| {
                    AppError::Download(format!("Failed to resolve remote size for {file}"))
                })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(DownloadSizePlan::new(file_sizes))
}

fn remote_file_sizes(
    repo: &HFRepositorySync<RepoTypeModel>,
    remote_files: &[String],
) -> AppResult<HashMap<String, u64>> {
    let mut sizes = HashMap::new();

    if !remote_files.is_empty() {
        if let Ok(entries) = repo.get_paths_info().paths(remote_files.to_vec()).send() {
            for entry in entries {
                if let RepoTreeEntry::File { path, size, .. } = entry {
                    if size > 0 {
                        sizes.insert(path, size);
                    }
                }
            }
        }
    }

    for file in remote_files {
        if sizes.get(file).is_some_and(|size| *size > 0) {
            continue;
        }

        let metadata = repo
            .get_file_metadata()
            .filepath(file.to_string())
            .send()
            .map_err(|error| AppError::Download(error.to_string()))?;
        if metadata.file_size == 0 {
            return Err(AppError::Download(format!(
                "Failed to resolve non-zero remote size for {file}"
            )));
        }
        sizes.insert(file.clone(), metadata.file_size);
    }

    Ok(sizes)
}

fn planned_progress_percent(
    completed_before_file: u64,
    planned_file_bytes: u64,
    planned_total_bytes: u64,
    file_completed: u64,
    file_total: u64,
) -> f64 {
    let file_ratio = if file_total > 0 {
        file_completed as f64 / file_total as f64
    } else if file_completed > 0 {
        1.0
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    let completed = completed_before_file as f64 + planned_file_bytes as f64 * file_ratio;

    if planned_total_bytes > 0 {
        (completed / planned_total_bytes as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    }
}

fn generate_tokenizer_json(model: KnownModel, dir: &std::path::Path) -> AppResult<()> {
    let vocab_path = dir.join("vocab.json");
    let merges_path = dir.join("merges.txt");
    let tokenizer_path = dir.join(TOKENIZER_JSON);

    if !vocab_path.exists() || !merges_path.exists() {
        return Err(AppError::Download(format!(
            "Cannot generate tokenizer.json for {} before vocab.json and merges.txt are downloaded.",
            model.title
        )));
    }

    let vocab = vocab_path.to_string_lossy();
    let merges = merges_path.to_string_lossy();
    let bpe = BPE::from_file(vocab.as_ref(), merges.as_ref())
        .build()
        .map_err(|error| AppError::Download(format!("Failed to build BPE tokenizer: {error}")))?;
    let mut tokenizer = Tokenizer::new(bpe);
    // Qwen3-ASR tokenizer_config.json specifies add_prefix_space=false.
    tokenizer.with_pre_tokenizer(Some(ByteLevel::new(false, true, true)));
    tokenizer.with_decoder(Some(ByteLevelDecoder::default()));

    tokenizer
        .save(&tokenizer_path, false)
        .map_err(|error| AppError::Download(format!("Failed to save tokenizer.json: {error}")))?;

    Ok(())
}

struct TauriProgressHandler {
    app: AppHandle,
    model_id: String,
    file_name: String,
    file_index: usize,
    total_files: usize,
    completed_before_file: u64,
    planned_file_bytes: u64,
    planned_total_bytes: u64,
    last_sample: Mutex<(Instant, u64)>,
}

impl TauriProgressHandler {
    fn new(
        app: AppHandle,
        model_id: &str,
        file_name: &str,
        file_index: usize,
        total_files: usize,
        completed_before_file: u64,
        planned_file_bytes: u64,
        planned_total_bytes: u64,
    ) -> Self {
        Self {
            app,
            model_id: model_id.to_string(),
            file_name: file_name.to_string(),
            file_index,
            total_files,
            completed_before_file,
            planned_file_bytes,
            planned_total_bytes,
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
        let percent = planned_progress_percent(
            self.completed_before_file,
            self.planned_file_bytes,
            self.planned_total_bytes,
            completed,
            total,
        );

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

#[cfg(test)]
mod tests {
    use super::{planned_progress_percent, DownloadSizePlan};

    fn assert_percent(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn size_plan_weights_large_files_by_bytes() {
        let plan = DownloadSizePlan::new(vec![10, 990]);

        assert_percent(plan.percent(0, 10, 10), 1.0);
        assert_percent(plan.percent(1, 495, 990), 50.5);
        assert_percent(plan.percent(1, 990, 990), 100.0);
    }

    #[test]
    fn planned_progress_clamps_current_file_ratio() {
        assert_percent(planned_progress_percent(10, 90, 100, 200, 100), 100.0);
        assert_percent(planned_progress_percent(10, 90, 100, 0, 100), 10.0);
    }

    #[test]
    fn size_plan_treats_zero_size_as_minimal_weight() {
        let plan = DownloadSizePlan::new(vec![0, 99]);

        assert_eq!(plan.file_size(0), 1);
        assert_eq!(plan.total_bytes, 100);
    }
}
