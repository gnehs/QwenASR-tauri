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

#[cfg(desktop)]
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    Emitter, Listener, Manager, PhysicalPosition,
};

#[cfg(target_os = "macos")]
use tauri::menu::PredefinedMenuItem;

#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, TitleBarStyle};

#[cfg(desktop)]
use tauri_plugin_opener::OpenerExt;

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

#[cfg(desktop)]
#[derive(serde::Deserialize)]
struct NativeMenuLabels {
    about: String,
    settings: String,
    file: String,
    new_task: String,
    edit: String,
    window: String,
    help: String,
    github: String,
}

#[cfg(desktop)]
const GITHUB_URL: &str = "https://github.com/gnehs/QwenASR-tauri";

#[cfg(desktop)]
fn open_about_window(app: &AppHandle) -> tauri::Result<()> {
    let about_window = if let Some(window) = app.get_webview_window("about") {
        window
    } else {
        WebviewWindowBuilder::new(
            app,
            "about",
            WebviewUrl::App("index.html?window=about".into()),
        )
        .title("關於 QwenASR Studio")
        .inner_size(540.0, 400.0)
        .resizable(false)
        .visible(false)
        .build()?
    };

    if let Some(main_window) = app.get_webview_window("main") {
        if let (Ok(main_position), Ok(main_size), Ok(about_size)) = (
            main_window.outer_position(),
            main_window.outer_size(),
            about_window.outer_size(),
        ) {
            let x = main_position.x + (main_size.width as i32 - about_size.width as i32) / 2;
            let y = main_position.y + (main_size.height as i32 - about_size.height as i32) / 2;
            about_window.set_position(PhysicalPosition::new(x, y))?;
        }
    }

    about_window.show()?;
    about_window.set_focus()?;

    Ok(())
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

            #[cfg(desktop)]
            {
                let about_item =
                    MenuItemBuilder::with_id("about", "關於 QwenASR Studio").build(app)?;
                let settings_item = MenuItemBuilder::with_id("settings", "設定")
                    .accelerator("CommandOrControl+,")
                    .build(app)?;
                let new_task_item = MenuItemBuilder::with_id("new-task", "新增任務")
                    .accelerator("CommandOrControl+N")
                    .build(app)?;
                let mut app_menu_builder = SubmenuBuilder::new(app, "QwenASR Studio")
                    .item(&about_item)
                    .separator()
                    .item(&settings_item);

                #[cfg(target_os = "macos")]
                {
                    let services = PredefinedMenuItem::services(app, None)?;
                    let hide = PredefinedMenuItem::hide(app, None)?;
                    let hide_others = PredefinedMenuItem::hide_others(app, None)?;
                    let show_all = PredefinedMenuItem::show_all(app, None)?;
                    app_menu_builder = app_menu_builder
                        .separator()
                        .item(&services)
                        .separator()
                        .item(&hide)
                        .item(&hide_others)
                        .item(&show_all);
                }

                let app_menu = app_menu_builder.separator().quit().build()?;
                let file_menu = SubmenuBuilder::new(app, "檔案")
                    .item(&new_task_item)
                    .build()?;
                let github_item = MenuItemBuilder::with_id("github", "GitHub").build(app)?;
                let help_menu = SubmenuBuilder::new(app, "說明")
                    .item(&github_item)
                    .build()?;

                #[cfg(target_os = "macos")]
                let (edit_menu, window_menu) = {
                    let undo = PredefinedMenuItem::undo(app, None)?;
                    let redo = PredefinedMenuItem::redo(app, None)?;
                    let cut = PredefinedMenuItem::cut(app, None)?;
                    let copy = PredefinedMenuItem::copy(app, None)?;
                    let paste = PredefinedMenuItem::paste(app, None)?;
                    let select_all = PredefinedMenuItem::select_all(app, None)?;
                    let edit_menu = SubmenuBuilder::new(app, "編輯")
                        .item(&undo)
                        .item(&redo)
                        .separator()
                        .item(&cut)
                        .item(&copy)
                        .item(&paste)
                        .item(&select_all)
                        .build()?;

                    let minimize = PredefinedMenuItem::minimize(app, None)?;
                    let maximize = PredefinedMenuItem::maximize(app, None)?;
                    let fullscreen = PredefinedMenuItem::fullscreen(app, None)?;
                    let close_window = PredefinedMenuItem::close_window(app, None)?;
                    let bring_all_to_front = PredefinedMenuItem::bring_all_to_front(app, None)?;
                    let window_menu = SubmenuBuilder::new(app, "視窗")
                        .item(&minimize)
                        .item(&maximize)
                        .item(&fullscreen)
                        .item(&close_window)
                        .separator()
                        .item(&bring_all_to_front)
                        .build()?;

                    (edit_menu, window_menu)
                };

                #[cfg(target_os = "macos")]
                let menu = MenuBuilder::new(app)
                    .items(&[&app_menu, &file_menu, &edit_menu, &window_menu, &help_menu])
                    .build()?;
                #[cfg(not(target_os = "macos"))]
                let menu = MenuBuilder::new(app)
                    .items(&[&app_menu, &file_menu, &help_menu])
                    .build()?;
                app.set_menu(menu)?;

                let about_item_for_locale = about_item.clone();
                let settings_item_for_locale = settings_item.clone();
                let file_menu_for_locale = file_menu.clone();
                let new_task_item_for_locale = new_task_item.clone();
                let help_menu_for_locale = help_menu.clone();
                let github_item_for_locale = github_item.clone();
                #[cfg(target_os = "macos")]
                let edit_menu_for_locale = edit_menu.clone();
                #[cfg(target_os = "macos")]
                let window_menu_for_locale = window_menu.clone();
                app.listen(
                    "native-menu-labels",
                    move |event| match serde_json::from_str::<NativeMenuLabels>(event.payload()) {
                        Ok(labels) => {
                            if let Err(error) = about_item_for_locale.set_text(&labels.about) {
                                eprintln!("failed to update about menu text: {error}");
                            }
                            if let Err(error) = settings_item_for_locale.set_text(&labels.settings)
                            {
                                eprintln!("failed to update settings menu text: {error}");
                            }
                            if let Err(error) = file_menu_for_locale.set_text(&labels.file) {
                                eprintln!("failed to update file menu text: {error}");
                            }
                            if let Err(error) = new_task_item_for_locale.set_text(&labels.new_task)
                            {
                                eprintln!("failed to update new task menu text: {error}");
                            }
                            if let Err(error) = help_menu_for_locale.set_text(&labels.help) {
                                eprintln!("failed to update help menu text: {error}");
                            }
                            if let Err(error) = github_item_for_locale.set_text(&labels.github) {
                                eprintln!("failed to update GitHub menu text: {error}");
                            }
                            #[cfg(target_os = "macos")]
                            {
                                if let Err(error) = edit_menu_for_locale.set_text(&labels.edit) {
                                    eprintln!("failed to update edit menu text: {error}");
                                }
                                if let Err(error) = window_menu_for_locale.set_text(&labels.window)
                                {
                                    eprintln!("failed to update window menu text: {error}");
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("failed to parse about menu text: {error}");
                        }
                    },
                );

                app.on_menu_event(|app, event| match event.id().0.as_str() {
                    "about" => {
                        if let Err(error) = open_about_window(app) {
                            eprintln!("failed to open about window: {error}");
                        }
                    }
                    "settings" | "new-task" => {
                        if let Err(error) =
                            app.emit_to("main", "native-menu-action", event.id().0.as_str())
                        {
                            eprintln!("failed to emit native menu action: {error}");
                        }
                    }
                    "github" => {
                        if let Err(error) = app.opener().open_url(GITHUB_URL, None::<&str>) {
                            eprintln!("failed to open GitHub URL: {error}");
                        }
                    }
                    _ => {}
                });
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
