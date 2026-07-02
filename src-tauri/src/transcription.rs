use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use qwen_asr::{config::SAMPLE_RATE, context::QwenCtx, kernels, transcribe};
use tauri::{AppHandle, Emitter};

use crate::audio::load_audio_samples;
use crate::error::{AppError, AppResult};
use crate::models::{
    TranscribeBatchRequest, TranscribeFileRequest, TranscribeOptions, TranscriptSegment,
    TranscriptionProgress, TranscriptionResult,
};
use crate::paths::{model_dir, model_status};
use crate::srt;

const TRANSCRIPTION_PROGRESS_EVENT: &str = "transcription-progress";

pub fn transcribe_file(
    app: AppHandle,
    request: TranscribeFileRequest,
) -> AppResult<TranscriptionResult> {
    let started = Instant::now();
    let audio_path = request.audio_path.clone();

    let result = transcribe_file_inner(&app, started, request);
    if let Err(error) = &result {
        emit_progress(
            &app,
            started,
            "error",
            "error",
            &format!("轉錄失敗：{error}"),
            Some(&audio_path),
            Some(&audio_path),
            1,
            1,
            100.0,
            None,
        );
    }

    result
}

fn transcribe_file_inner(
    app: &AppHandle,
    started: Instant,
    request: TranscribeFileRequest,
) -> AppResult<TranscriptionResult> {
    emit_progress(
        app,
        started,
        "running",
        "preparing",
        "確認模型狀態",
        Some(&request.audio_path),
        Some(&request.audio_path),
        1,
        1,
        2.0,
        None,
    );

    let model_path = ensure_model(&request.options.model_id)?;
    emit_progress(
        app,
        started,
        "running",
        "loadingModel",
        "載入 QwenASR 模型",
        Some(&request.audio_path),
        Some(&request.audio_path),
        1,
        1,
        8.0,
        None,
    );

    let mut ctx = build_context(&model_path, &request.options)?;
    let result = transcribe_with_context(
        app,
        started,
        &mut ctx,
        &request.audio_path,
        &request.options,
        1,
        1,
        12.0,
        98.0,
    )?;

    emit_progress(
        app,
        started,
        "complete",
        "complete",
        "轉錄完成",
        Some(&request.audio_path),
        Some(&request.audio_path),
        1,
        1,
        100.0,
        Some(0),
    );

    Ok(result)
}

pub fn transcribe_batch(
    app: AppHandle,
    request: TranscribeBatchRequest,
) -> AppResult<Vec<TranscriptionResult>> {
    let started = Instant::now();
    if request.audio_paths.is_empty() {
        emit_progress(
            &app,
            started,
            "error",
            "error",
            "沒有選取音訊檔案",
            None,
            None,
            0,
            0,
            100.0,
            None,
        );
        return Err(AppError::Transcription("No audio files selected.".into()));
    }

    emit_progress(
        &app,
        started,
        "running",
        "preparing",
        "確認模型與批次佇列",
        None,
        None,
        0,
        request.audio_paths.len(),
        2.0,
        None,
    );

    let model_path = ensure_model(&request.options.model_id)?;
    emit_progress(
        &app,
        started,
        "running",
        "loadingModel",
        "載入 QwenASR 模型",
        None,
        None,
        0,
        request.audio_paths.len(),
        8.0,
        None,
    );

    let mut ctx = build_context(&model_path, &request.options)?;
    let total = request.audio_paths.len();
    let mut results = Vec::with_capacity(total);

    for (index, audio_path) in request.audio_paths.iter().enumerate() {
        let file_index = index + 1;
        let range_start = 10.0 + (index as f64 / total as f64) * 88.0;
        let range_end = 10.0 + (file_index as f64 / total as f64) * 88.0;

        let result = match transcribe_with_context(
            &app,
            started,
            &mut ctx,
            audio_path,
            &request.options,
            file_index,
            total,
            range_start,
            range_end,
        ) {
            Ok(result) => result,
            Err(error) => {
                emit_progress(
                    &app,
                    started,
                    "error",
                    "error",
                    &format!("轉錄失敗：{error}"),
                    Some(audio_path),
                    Some(audio_path),
                    file_index,
                    total,
                    range_end,
                    None,
                );
                return Err(error);
            }
        };
        results.push(result);
    }

    emit_progress(
        &app,
        started,
        "complete",
        "complete",
        "批次轉錄完成",
        None,
        None,
        total,
        total,
        100.0,
        Some(0),
    );

    Ok(results)
}

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    app: &AppHandle,
    started: Instant,
    state: &str,
    phase: &str,
    message: &str,
    audio_path: Option<&str>,
    current_file: Option<&str>,
    file_index: usize,
    total_files: usize,
    percent: f64,
    eta_ms: Option<u128>,
) {
    let percent = clamp_percent(percent);
    let eta_ms = match state {
        "complete" => Some(0),
        "error" => None,
        _ => eta_ms.or_else(|| estimate_eta(started, percent)),
    };

    let _ = app.emit(
        TRANSCRIPTION_PROGRESS_EVENT,
        TranscriptionProgress {
            state: state.to_string(),
            phase: phase.to_string(),
            message: message.to_string(),
            audio_path: audio_path.map(str::to_string),
            current_file: current_file.map(str::to_string),
            file_index,
            total_files,
            percent,
            elapsed_ms: started.elapsed().as_millis(),
            eta_ms,
        },
    );
}

fn clamp_percent(percent: f64) -> f64 {
    if percent.is_finite() {
        percent.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

fn progress_between(start: f64, end: f64, ratio: f64) -> f64 {
    start + (end - start).max(0.0) * ratio.clamp(0.0, 1.0)
}

fn estimate_eta(started: Instant, percent: f64) -> Option<u128> {
    if !(1.0..99.0).contains(&percent) {
        return None;
    }

    let elapsed_ms = started.elapsed().as_millis();
    if elapsed_ms < 800 {
        return None;
    }

    let elapsed = elapsed_ms as f64;
    let remaining = elapsed * ((100.0 - percent) / percent);
    Some(remaining.max(0.0).round() as u128)
}

fn estimate_eta_from_audio(started: Instant, percent: f64, audio_ms: u128) -> Option<u128> {
    let progress_eta = estimate_eta(started, percent);
    if audio_ms < 1_000 {
        return progress_eta;
    }

    let elapsed_ms = started.elapsed().as_millis();
    let audio_floor = (audio_ms / 3).saturating_sub(elapsed_ms);
    if audio_floor == 0 {
        return progress_eta;
    }

    Some(progress_eta.map_or(audio_floor, |eta| eta.max(audio_floor)))
}

fn ensure_model(model_id: &str) -> AppResult<PathBuf> {
    let status = model_status(model_id)?;
    if !status.installed {
        return Err(AppError::Model(format!(
            "{} is not installed. Missing: {}",
            status.title,
            status.missing_files.join(", ")
        )));
    }
    model_dir(model_id)
}

fn build_context(model_path: &Path, options: &TranscribeOptions) -> AppResult<QwenCtx> {
    let threads = options
        .threads
        .filter(|threads| *threads > 0)
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|count| count.get().min(8))
                .unwrap_or(4)
        });
    kernels::set_threads(threads);
    kernels::set_verbose(0);

    let model_path_string = model_path.to_string_lossy().to_string();
    let mut ctx = QwenCtx::load(&model_path_string).ok_or_else(|| {
        AppError::Model(format!(
            "Failed to load model from {}",
            model_path.display()
        ))
    })?;

    ctx.segment_sec = options.segment_seconds.max(1.0);
    ctx.search_sec = options.search_seconds.max(0.25);
    ctx.skip_silence = options.skip_silence;
    ctx.past_text_conditioning = options.past_text;

    if let Some(prompt) = options
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        ctx.set_prompt(prompt)
            .map_err(|_| AppError::Model("Failed to set the prompt.".into()))?;
    }

    if let Some(language) = options
        .language
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "auto")
    {
        ctx.set_force_language(language)
            .map_err(|_| AppError::Model(format!("Unsupported language: {language}")))?;
    }

    Ok(ctx)
}

fn transcribe_with_context(
    app: &AppHandle,
    progress_started: Instant,
    ctx: &mut QwenCtx,
    audio_path: &str,
    options: &TranscribeOptions,
    file_index: usize,
    total_files: usize,
    range_start: f64,
    range_end: f64,
) -> AppResult<TranscriptionResult> {
    let started = Instant::now();
    emit_progress(
        app,
        progress_started,
        "running",
        "loadingAudio",
        "讀取與正規化音訊",
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        range_start,
        None,
    );

    let samples = load_audio_samples(audio_path, options.convert_with_ffmpeg)?;
    let audio_ms = (samples.len() as f64 / SAMPLE_RATE as f64 * 1000.0).round() as u128;
    let inference_percent = progress_between(range_start, range_end, 0.35);
    emit_progress(
        app,
        progress_started,
        "running",
        "transcribing",
        "模型推論中",
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        inference_percent,
        estimate_eta_from_audio(progress_started, inference_percent, audio_ms),
    );

    let raw_segments = transcribe::transcribe_segmented(ctx, &samples)
        .ok_or_else(|| AppError::Transcription("QwenASR failed to transcribe this file.".into()))?;

    let finalize_percent = progress_between(range_start, range_end, 0.88);
    emit_progress(
        app,
        progress_started,
        "running",
        if options.write_srt {
            "writingSrt"
        } else {
            "finalizing"
        },
        if options.write_srt {
            "寫入 SRT 字幕"
        } else {
            "整理轉錄結果"
        },
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        finalize_percent,
        None,
    );

    let segments = raw_segments
        .into_iter()
        .map(|segment| TranscriptSegment {
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            text: segment.text,
        })
        .collect::<Vec<_>>();
    let text = segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let srt_path = if options.write_srt {
        Some(write_srt(audio_path, options, &segments)?)
    } else {
        None
    };

    emit_progress(
        app,
        progress_started,
        "running",
        "finalizing",
        "檔案處理完成",
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        range_end,
        None,
    );

    Ok(TranscriptionResult {
        audio_path: audio_path.to_string(),
        text,
        segments,
        srt_path,
        duration_ms: started.elapsed().as_millis(),
    })
}

fn write_srt(
    audio_path: &str,
    options: &TranscribeOptions,
    segments: &[TranscriptSegment],
) -> AppResult<String> {
    let audio = Path::new(audio_path);
    let stem = audio
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("transcript");
    let directory = options
        .output_dir
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| audio.parent().map(Path::to_path_buf))
        .ok_or_else(|| AppError::Io("Could not resolve output directory.".into()))?;
    fs::create_dir_all(&directory)?;

    let path = directory.join(format!("{stem}.srt"));
    fs::write(&path, srt::render(segments))?;
    Ok(path.to_string_lossy().to_string())
}
