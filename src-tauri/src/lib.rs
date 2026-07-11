mod audio;
mod downloader;
mod error;
mod forced_alignment;
mod models;
mod paths;
mod srt;
mod transcription;
mod vad;

use error::AppResult;
use models::{
    FfmpegStatus, ModelStatus, TranscribeBatchRequest, TranscribeFileRequest, TranscriptionResult,
};
use tauri::{ipc::Channel, AppHandle, State, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, TitleBarStyle};

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
    control: State<'_, transcription::TranscriptionControl>,
    request: TranscribeFileRequest,
    on_progress: Channel<models::TranscriptionProgress>,
) -> AppResult<TranscriptionResult> {
    let task_id = request.task_id.clone();
    let cancel = control.register(&task_id)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        transcription::transcribe_file(on_progress, request, cancel)
    })
    .await
    .map_err(|error| error::AppError::Transcription(error.to_string()));
    control.remove(&task_id);
    result?
}

#[tauri::command]
fn cancel_transcription(
    control: State<'_, transcription::TranscriptionControl>,
    task_id: String,
) -> bool {
    control.cancel(&task_id)
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
        .manage(transcription::TranscriptionControl::default())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let window_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("QwenASR Studio")
                .inner_size(1180.0, 760.0)
                .min_inner_size(980.0, 680.0);

            #[cfg(target_os = "macos")]
            let window_builder = window_builder
                .title_bar_style(TitleBarStyle::Overlay)
                .hidden_title(true)
                .traffic_light_position(LogicalPosition::new(14.0, 25.0));

            let window = window_builder.build()?;

            #[cfg(target_os = "macos")]
            {
                use objc2_app_kit::{NSColor, NSWindow};

                let ns_window: &NSWindow = unsafe { &*window.ns_window()?.cast() };
                let bg_color = NSColor::colorWithRed_green_blue_alpha(
                    250.0 / 255.0,
                    250.0 / 255.0,
                    250.0 / 255.0,
                    1.0,
                );
                ns_window.setBackgroundColor(Some(&bg_color));
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_available_models,
            get_ffmpeg_status,
            download_model,
            delete_model,
            transcribe_file,
            cancel_transcription,
            transcribe_batch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
