mod audio;
mod downloader;
mod error;
mod models;
mod paths;
mod srt;
mod transcription;
mod vad;

use error::AppResult;
use models::{
    FfmpegStatus, ModelStatus, TranscribeBatchRequest, TranscribeFileRequest, TranscriptionResult,
};
use tauri::AppHandle;

#[tauri::command]
fn list_available_models() -> AppResult<Vec<ModelStatus>> {
    models::KNOWN_MODELS
        .iter()
        .map(|model| paths::model_status(model.id))
        .collect()
}

#[tauri::command]
fn get_ffmpeg_status() -> FfmpegStatus {
    audio::ffmpeg_status()
}

#[tauri::command]
async fn download_model(app: AppHandle, model_id: String) -> AppResult<ModelStatus> {
    tauri::async_runtime::spawn_blocking(move || downloader::download_model(app, model_id))
        .await
        .map_err(|error| error::AppError::Download(error.to_string()))?
}

#[tauri::command]
async fn delete_model(model_id: String) -> AppResult<ModelStatus> {
    tauri::async_runtime::spawn_blocking(move || paths::delete_model(&model_id))
        .await
        .map_err(|error| error::AppError::Model(error.to_string()))?
}

#[tauri::command]
async fn transcribe_file(
    app: AppHandle,
    request: TranscribeFileRequest,
) -> AppResult<TranscriptionResult> {
    tauri::async_runtime::spawn_blocking(move || transcription::transcribe_file(app, request))
        .await
        .map_err(|error| error::AppError::Transcription(error.to_string()))?
}

#[tauri::command]
async fn transcribe_batch(
    app: AppHandle,
    request: TranscribeBatchRequest,
) -> AppResult<Vec<TranscriptionResult>> {
    tauri::async_runtime::spawn_blocking(move || transcription::transcribe_batch(app, request))
        .await
        .map_err(|error| error::AppError::Transcription(error.to_string()))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_available_models,
            get_ffmpeg_status,
            download_model,
            delete_model,
            transcribe_file,
            transcribe_batch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
