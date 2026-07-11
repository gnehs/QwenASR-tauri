use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::Instant;

use opencc_rs::{Config, OpenCC};
use qwen3_asr::{
    inference::{AsrInference, InferenceTimings, TranscribeResult as QwenTranscribeResult},
    tensor::Device,
};
use tauri::{AppHandle, Emitter};

use crate::audio;
use crate::error::{AppError, AppResult};
use crate::forced_alignment::{tokenize_alignment_units, AlignedUnit, ForcedAlignerInference};
use crate::models::{
    TranscribeBatchRequest, TranscribeFileRequest, TranscribeOptions, TranscriptSegment,
    TranscriptionProgress, TranscriptionResult, TranscriptionTimings, FORCED_ALIGNER_MODEL_ID,
};
use crate::paths::{model_dir, model_status};
use crate::srt;
use crate::vad::{self, AudioRange};

const TRANSCRIPTION_PROGRESS_EVENT: &str = "transcription-progress";
const TARGET_SRT_CHARS: usize = 42;
const MIN_SEGMENT_MS: u64 = 900;
const CHINESE_ASR_LANGUAGE: &str = "Chinese";
const TRADITIONAL_CHINESE_LANGUAGE: &str = "chinese";
const SIMPLIFIED_CHINESE_LANGUAGE: &str = "chinese (simplified)";
const PREPARE_PROGRESS_PERCENT: f64 = 2.0;
const MODEL_LOAD_PROGRESS_PERCENT: f64 = 8.0;
const TRANSCRIPTION_WORK_START_PERCENT: f64 = MODEL_LOAD_PROGRESS_PERCENT;
const TRANSCRIPTION_WORK_END_PERCENT: f64 = 99.0;
const VAD_PHASE_START: f64 = 0.0;
const VAD_PHASE_RATIO: f64 = 0.05;
const VAD_PHASE_END: f64 = VAD_PHASE_START + VAD_PHASE_RATIO;
const TRANSCRIBE_PHASE_START: f64 = VAD_PHASE_END;
// Based on the current measurements: VAD is about 5%, ASR about 91%,
// alignment about 4% of the post-model processing time.
const TRANSCRIBE_PHASE_END_WITH_ALIGNMENT: f64 = 0.96;
const TRANSCRIBE_PHASE_END_WITHOUT_ALIGNMENT: f64 = 1.0;
const ALIGN_PHASE_START: f64 = TRANSCRIBE_PHASE_END_WITH_ALIGNMENT;
const ALIGN_PHASE_END: f64 = 1.0;
const ASR_CHUNK_OVERHEAD_MS: u64 = 500;
static S2TW_CONVERTER: OnceLock<Mutex<Option<OpenCC>>> = OnceLock::new();
const ASR_LANGUAGES: &[&str] = &[
    "Chinese",
    "English",
    "Cantonese (Hong Kong accent)",
    "Cantonese (Guangdong accent)",
    "Cantonese",
    "Arabic",
    "German",
    "French",
    "Spanish",
    "Portuguese",
    "Indonesian",
    "Italian",
    "Korean",
    "Russian",
    "Thai",
    "Vietnamese",
    "Japanese",
    "Turkish",
    "Hindi",
    "Malay",
    "Dutch",
    "Swedish",
    "Danish",
    "Finnish",
    "Polish",
    "Czech",
    "Filipino",
    "Persian",
    "Greek",
    "Hungarian",
    "Macedonian",
    "Romanian",
    "Anhui",
    "Dongbei",
    "Fujian",
    "Gansu",
    "Guizhou",
    "Hebei",
    "Henan",
    "Hubei",
    "Hunan",
    "Jiangxi",
    "Ningxia",
    "Shandong",
    "Shaanxi",
    "Shanxi",
    "Sichuan",
    "Tianjin",
    "Yunnan",
    "Zhejiang",
    "Wu language",
    "Minnan language",
];

pub type CancellationToken = Arc<AtomicBool>;

#[derive(Clone, Default)]
pub struct TranscriptionControl {
    active: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl TranscriptionControl {
    pub fn register(&self, task_id: &str) -> AppResult<CancellationToken> {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if active.contains_key(task_id) {
            return Err(AppError::Transcription(
                "A transcription task with this ID is already running.".into(),
            ));
        }

        let token = Arc::new(AtomicBool::new(false));
        active.insert(task_id.to_string(), Arc::clone(&token));
        Ok(token)
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        let active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(token) = active.get(task_id) else {
            return false;
        };

        token.store(true, Ordering::Release);
        true
    }

    pub fn remove(&self, task_id: &str) {
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        active.remove(task_id);
    }
}

fn check_cancelled(cancel: &CancellationToken) -> AppResult<()> {
    if cancel.load(Ordering::Acquire) {
        Err(AppError::Cancelled("轉錄已終止".into()))
    } else {
        Ok(())
    }
}

pub fn transcribe_file(
    app: AppHandle,
    request: TranscribeFileRequest,
    cancel: CancellationToken,
) -> AppResult<TranscriptionResult> {
    let started = Instant::now();
    let audio_path = request.audio_path.clone();

    let result = transcribe_file_inner(&app, started, request, &cancel);
    if let Err(error) = &result {
        if matches!(error, AppError::Cancelled(_)) {
            emit_progress(
                &app,
                started,
                "cancelled",
                "cancelled",
                "轉錄已終止",
                Some(&audio_path),
                Some(&audio_path),
                1,
                1,
                100.0,
                None,
            );
            return result;
        }

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
    cancel: &CancellationToken,
) -> AppResult<TranscriptionResult> {
    check_cancelled(cancel)?;
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
        PREPARE_PROGRESS_PERCENT,
        None,
    );

    let model_path = ensure_model(&request.options.model_id)?;
    check_cancelled(cancel)?;
    let use_forced_aligner = should_use_forced_aligner(&request.options);
    let aligner_path = use_forced_aligner
        .then(|| ensure_model(FORCED_ALIGNER_MODEL_ID))
        .transpose()?;
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
        MODEL_LOAD_PROGRESS_PERCENT,
        None,
    );

    let device = default_device();
    let engine = load_engine(&model_path, device)?;
    check_cancelled(cancel)?;
    let asr_range_end = if use_forced_aligner {
        progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            TRANSCRIPTION_WORK_END_PERCENT,
            TRANSCRIBE_PHASE_END_WITH_ALIGNMENT,
        )
    } else {
        progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            TRANSCRIPTION_WORK_END_PERCENT,
            TRANSCRIBE_PHASE_END_WITHOUT_ALIGNMENT,
        )
    };
    let mut pending = transcribe_with_context(
        app,
        started,
        &engine,
        &request.audio_path,
        &request.options,
        1,
        1,
        TRANSCRIPTION_WORK_START_PERCENT,
        asr_range_end,
        cancel,
    )?;
    pending.range_start = if use_forced_aligner {
        progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            TRANSCRIPTION_WORK_END_PERCENT,
            ALIGN_PHASE_START,
        )
    } else {
        asr_range_end
    };
    pending.range_end = progress_between(
        TRANSCRIPTION_WORK_START_PERCENT,
        TRANSCRIPTION_WORK_END_PERCENT,
        ALIGN_PHASE_END,
    );
    drop(engine);
    check_cancelled(cancel)?;
    let aligner = load_aligner_after_transcription(
        app,
        started,
        aligner_path.as_deref(),
        device,
        Some(&request.audio_path),
        1,
        1,
        progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            TRANSCRIPTION_WORK_END_PERCENT,
            ALIGN_PHASE_START,
        ),
    )?;
    check_cancelled(cancel)?;
    let mut result =
        finalize_transcription_with_context(app, started, aligner.as_ref(), pending, cancel)?;
    result.duration_ms = started.elapsed().as_millis();
    result.timings.total_ms = result.duration_ms;

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
    let cancel = Arc::new(AtomicBool::new(false));
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
        PREPARE_PROGRESS_PERCENT,
        None,
    );

    let model_path = ensure_model(&request.options.model_id)?;
    let use_forced_aligner = should_use_forced_aligner(&request.options);
    let aligner_path = use_forced_aligner
        .then(|| ensure_model(FORCED_ALIGNER_MODEL_ID))
        .transpose()?;
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
        MODEL_LOAD_PROGRESS_PERCENT,
        None,
    );

    let device = default_device();
    let engine = load_engine(&model_path, device)?;
    let total = request.audio_paths.len();
    let mut pending_results = Vec::with_capacity(total);

    for (index, audio_path) in request.audio_paths.iter().enumerate() {
        let file_index = index + 1;
        let asr_request_end = progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            TRANSCRIPTION_WORK_END_PERCENT,
            if use_forced_aligner {
                TRANSCRIBE_PHASE_END_WITH_ALIGNMENT
            } else {
                TRANSCRIBE_PHASE_END_WITHOUT_ALIGNMENT
            },
        );
        let range_start = progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            asr_request_end,
            index as f64 / total as f64,
        );
        let range_end = progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            asr_request_end,
            file_index as f64 / total as f64,
        );

        let pending = match transcribe_with_context(
            &app,
            started,
            &engine,
            audio_path,
            &request.options,
            file_index,
            total,
            range_start,
            range_end,
            &cancel,
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
        let mut pending = pending;
        if use_forced_aligner {
            let align_request_start = progress_between(
                TRANSCRIPTION_WORK_START_PERCENT,
                TRANSCRIPTION_WORK_END_PERCENT,
                ALIGN_PHASE_START,
            );
            let align_request_end = progress_between(
                TRANSCRIPTION_WORK_START_PERCENT,
                TRANSCRIPTION_WORK_END_PERCENT,
                ALIGN_PHASE_END,
            );
            pending.range_start = progress_between(
                align_request_start,
                align_request_end,
                index as f64 / total as f64,
            );
            pending.range_end = progress_between(
                align_request_start,
                align_request_end,
                file_index as f64 / total as f64,
            );
        } else {
            pending.range_start = range_end;
            pending.range_end = range_end;
        }
        pending_results.push(pending);
    }

    drop(engine);
    let aligner = load_aligner_after_transcription(
        &app,
        started,
        aligner_path.as_deref(),
        device,
        None,
        total,
        total,
        progress_between(
            TRANSCRIPTION_WORK_START_PERCENT,
            TRANSCRIPTION_WORK_END_PERCENT,
            ALIGN_PHASE_START,
        ),
    )?;
    let mut results = Vec::with_capacity(total);
    for pending in pending_results {
        let audio_path = pending.audio_path.clone();
        let file_index = pending.file_index;
        let range_end = pending.range_end;
        let result = match finalize_transcription_with_context(
            &app,
            started,
            aligner.as_ref(),
            pending,
            &cancel,
        ) {
            Ok(result) => result,
            Err(error) => {
                emit_progress(
                    &app,
                    started,
                    "error",
                    "error",
                    &format!("轉錄失敗：{error}"),
                    Some(&audio_path),
                    Some(&audio_path),
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
    emit_progress_with_metrics(
        app,
        started,
        state,
        phase,
        message,
        audio_path,
        current_file,
        file_index,
        total_files,
        percent,
        eta_ms,
        None,
    );
}

#[derive(Debug, Clone, Default)]
struct ProgressMetrics {
    chunk_index: Option<usize>,
    total_chunks: Option<usize>,
    chunk_start_ms: Option<u64>,
    chunk_end_ms: Option<u64>,
    processed_audio_ms: Option<u64>,
    total_speech_ms: Option<u64>,
    skipped_silence_ms: Option<u64>,
    partial_segments: Option<Vec<TranscriptSegment>>,
    timings: Option<TranscriptionTimings>,
}

#[allow(clippy::too_many_arguments)]
fn emit_progress_with_metrics(
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
    metrics: Option<ProgressMetrics>,
) {
    let percent = clamp_percent(percent);
    let metrics = metrics.unwrap_or_default();
    let eta_ms = match state {
        "complete" => Some(0),
        "error" => None,
        _ => eta_ms,
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
            chunk_index: metrics.chunk_index,
            total_chunks: metrics.total_chunks,
            chunk_start_ms: metrics.chunk_start_ms,
            chunk_end_ms: metrics.chunk_end_ms,
            processed_audio_ms: metrics.processed_audio_ms,
            total_speech_ms: metrics.total_speech_ms,
            skipped_silence_ms: metrics.skipped_silence_ms,
            partial_segments: metrics.partial_segments,
            timings: metrics.timings,
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_chunk_progress(
    app: &AppHandle,
    progress_started: Instant,
    audio_path: &str,
    file_index: usize,
    total_files: usize,
    chunk_index: usize,
    total_chunks: usize,
    chunk: AudioRange,
    processed_audio_ms: u64,
    total_speech_ms: u64,
    skipped_silence_ms: u64,
    percent: f64,
    eta_ms: Option<u128>,
    phase: &str,
    message: &str,
    partial_segments: Option<Vec<TranscriptSegment>>,
    timings: Option<TranscriptionTimings>,
) {
    emit_progress_with_metrics(
        app,
        progress_started,
        "running",
        phase,
        message,
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        percent,
        eta_ms,
        Some(ProgressMetrics {
            chunk_index: Some(chunk_index),
            total_chunks: Some(total_chunks),
            chunk_start_ms: Some(chunk.start_ms()),
            chunk_end_ms: Some(chunk.end_ms()),
            processed_audio_ms: Some(processed_audio_ms),
            total_speech_ms: Some(total_speech_ms),
            skipped_silence_ms: Some(skipped_silence_ms),
            partial_segments,
            timings,
        }),
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

fn estimate_remaining_ms(started: Instant, processed_ms: u64, total_ms: u64) -> Option<u128> {
    let elapsed_ms = started.elapsed().as_millis();
    if elapsed_ms < 800 {
        return None;
    }

    estimate_remaining_ms_from_elapsed(elapsed_ms, processed_ms, total_ms)
}

fn estimate_remaining_ms_from_elapsed(
    elapsed_ms: u128,
    processed_ms: u64,
    total_ms: u64,
) -> Option<u128> {
    if processed_ms == 0 || processed_ms >= total_ms {
        return None;
    }

    let elapsed = elapsed_ms as f64;
    let remaining_ms = total_ms.saturating_sub(processed_ms);
    let remaining = elapsed * (remaining_ms as f64 / processed_ms as f64);
    Some(remaining.max(0.0).round() as u128)
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

fn load_engine(model_path: &Path, device: Device) -> AppResult<AsrInference> {
    AsrInference::load(model_path, device).map_err(|error| {
        AppError::Model(format!(
            "Failed to load model from {}: {error}",
            model_path.display()
        ))
    })
}

fn load_forced_aligner(model_path: &Path, device: Device) -> AppResult<ForcedAlignerInference> {
    ForcedAlignerInference::load(model_path, device)
}

#[allow(clippy::too_many_arguments)]
fn load_aligner_after_transcription(
    app: &AppHandle,
    started: Instant,
    model_path: Option<&Path>,
    device: Device,
    audio_path: Option<&str>,
    file_index: usize,
    total_files: usize,
    percent: f64,
) -> AppResult<Option<ForcedAlignerInference>> {
    let Some(model_path) = model_path else {
        return Ok(None);
    };

    emit_progress(
        app,
        started,
        "running",
        "loadingModel",
        "ASR 轉錄完成，載入 ForcedAligner 模型",
        audio_path,
        audio_path,
        file_index,
        total_files,
        percent,
        None,
    );

    load_forced_aligner(model_path, device).map(Some)
}

struct PendingChunk {
    chunk_index: usize,
    range: AudioRange,
    samples: Vec<i16>,
    raw_text: String,
    output_text: String,
    alignment_language: Option<&'static str>,
}

struct PendingTranscription {
    audio_path: String,
    options: TranscribeOptions,
    file_index: usize,
    total_files: usize,
    range_start: f64,
    range_end: f64,
    total_chunks: usize,
    total_speech_ms: u64,
    skipped_silence_ms: u64,
    transcript_parts: Vec<String>,
    vad_estimated_segments: Vec<TranscriptSegment>,
    chunks: Vec<PendingChunk>,
    duration_ms: u128,
    timings: TranscriptionTimings,
}

fn default_device() -> Device {
    #[cfg(target_os = "macos")]
    {
        qwen3_asr::backend::mlx::stream::init_mlx(true);
        Device::Gpu(0)
    }

    #[cfg(not(target_os = "macos"))]
    {
        if tch::Cuda::is_available() {
            Device::Gpu(0)
        } else {
            Device::Cpu
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn transcribe_with_context(
    app: &AppHandle,
    progress_started: Instant,
    engine: &AsrInference,
    audio_path: &str,
    options: &TranscribeOptions,
    file_index: usize,
    total_files: usize,
    range_start: f64,
    range_end: f64,
    cancel: &CancellationToken,
) -> AppResult<PendingTranscription> {
    check_cancelled(cancel)?;
    let started = Instant::now();
    let mut timings = TranscriptionTimings::default();
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

    let audio_prepare_started = Instant::now();
    let prepared_audio = audio::prepare_audio_for_asr(audio_path)?;
    let normalized_samples = audio::read_normalized_i16(prepared_audio.inference_path())?;
    check_cancelled(cancel)?;
    timings.audio_prepare_ms = audio_prepare_started.elapsed().as_millis();
    let analysis_percent = progress_between(range_start, range_end, VAD_PHASE_START);
    emit_progress(
        app,
        progress_started,
        "running",
        "analyzingAudio",
        "偵測語音與靜音片段",
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        analysis_percent,
        None,
    );

    let mut last_vad_percent = analysis_percent;
    let mut last_vad_message = "";
    let vad_started = Instant::now();
    let cancel_for_vad = Arc::clone(cancel);
    let vad_analysis = vad::analyze_with_progress(&normalized_samples, |vad_progress| {
        if cancel_for_vad.load(Ordering::Relaxed) {
            return;
        }

        let file_ratio = progress_between(VAD_PHASE_START, VAD_PHASE_END, vad_progress.ratio);
        let percent = progress_between(range_start, range_end, file_ratio);
        let message_changed = vad_progress.message != last_vad_message;
        let moved_enough = percent - last_vad_percent >= 0.5;
        if message_changed || moved_enough || vad_progress.ratio >= 1.0 {
            last_vad_percent = percent;
            last_vad_message = vad_progress.message;
            emit_progress(
                app,
                progress_started,
                "running",
                "analyzingAudio",
                vad_progress.message,
                Some(audio_path),
                Some(audio_path),
                file_index,
                total_files,
                percent,
                None,
            );
        }
    })?;
    check_cancelled(cancel)?;
    timings.vad_ms = vad_started.elapsed().as_millis();
    let chunks = vad_analysis.chunks;
    let total_chunk_audio_ms = vad_analysis.chunk_audio_ms.max(1);
    let skipped_silence_ms = vad_analysis.skipped_silence_ms;
    let mut processed_audio_ms = 0u64;
    let mut processed_asr_work_ms = 0u64;
    let mut transcript_parts = Vec::with_capacity(chunks.len());
    let mut pending_chunks = Vec::with_capacity(chunks.len());
    let mut vad_estimated_segments = Vec::new();
    let total_chunks = chunks.len();
    let total_asr_work_ms = total_chunk_work_ms(&chunks);
    let vad_message = if skipped_silence_ms > 0 {
        format!(
            "找到 {total_chunks} 個有聲片段，將跳過 {} 靜音",
            format_duration_short(skipped_silence_ms)
        )
    } else {
        format!("找到 {total_chunks} 個有聲片段，沒有可跳過的長靜音")
    };
    emit_progress_with_metrics(
        app,
        progress_started,
        "running",
        "analyzingAudio",
        &vad_message,
        Some(audio_path),
        Some(audio_path),
        file_index,
        total_files,
        progress_between(range_start, range_end, VAD_PHASE_END),
        None,
        Some(ProgressMetrics {
            chunk_index: None,
            total_chunks: Some(total_chunks),
            chunk_start_ms: None,
            chunk_end_ms: None,
            processed_audio_ms: Some(0),
            total_speech_ms: Some(total_chunk_audio_ms),
            skipped_silence_ms: Some(skipped_silence_ms),
            partial_segments: None,
            timings: Some(timings.clone()),
        }),
    );

    let output_language = normalize_output_language(options.language.as_deref());
    let asr_language = normalize_asr_language(output_language.as_deref());
    let language_forced = asr_language.is_some();
    let asr_started = Instant::now();

    for (index, chunk) in chunks.iter().enumerate() {
        check_cancelled(cancel)?;
        let chunk_index = index + 1;
        let before_ratio = processed_asr_work_ms as f64 / total_asr_work_ms as f64;
        let before_percent = progress_between(
            progress_between(range_start, range_end, TRANSCRIBE_PHASE_START),
            range_end,
            before_ratio,
        );
        let before_eta_ms =
            estimate_remaining_ms(asr_started, processed_asr_work_ms, total_asr_work_ms);
        emit_chunk_progress(
            app,
            progress_started,
            audio_path,
            file_index,
            total_files,
            chunk_index,
            total_chunks,
            *chunk,
            processed_audio_ms,
            total_chunk_audio_ms,
            skipped_silence_ms,
            before_percent,
            before_eta_ms,
            "transcribingSegments",
            &format!("轉錄第 {chunk_index} / {total_chunks} 個有聲片段"),
            Some(vad_estimated_segments.clone()),
            Some(timings.clone()),
        );

        let asr_chunk_started = Instant::now();
        let chunk_samples = normalized_samples[chunk.start_sample..chunk.end_sample].to_vec();
        let chunk_samples_f32 = normalized_i16_to_f32(&chunk_samples);
        let raw_result = engine
            .transcribe_samples(&chunk_samples_f32, asr_language.as_deref())
            .map_err(|error| {
                AppError::Transcription(format!(
                    "Qwen3-ASR failed to transcribe chunk {chunk_index}/{total_chunks}: {error}"
                ))
            })?;
        check_cancelled(cancel)?;
        let asr_chunk_elapsed_ms = asr_chunk_started.elapsed().as_millis();
        timings.asr_ms = timings.asr_ms.saturating_add(asr_chunk_elapsed_ms);
        add_inference_timings(&mut timings, &raw_result.timings);
        timings.asr_chunk_count = timings.asr_chunk_count.saturating_add(1);

        let raw_chunk_text = normalize_asr_text(&raw_result, language_forced);
        let chunk_text =
            convert_text_for_output_language(raw_chunk_text.clone(), output_language.as_deref())?;
        if !raw_chunk_text.trim().is_empty() {
            let detected_language = parse_auto_asr_language(&raw_result.raw_output)
                .unwrap_or(raw_result.language.as_str());
            let alignment_language = alignment_language(asr_language.as_deref(), detected_language);
            transcript_parts.push(chunk_text.clone());
            vad_estimated_segments.extend(build_approximate_segments_with_offset(
                &chunk_text,
                chunk.duration_ms(),
                chunk.start_ms(),
                options.segment_by_punctuation,
            ));
            pending_chunks.push(PendingChunk {
                chunk_index,
                range: *chunk,
                samples: chunk_samples,
                raw_text: raw_chunk_text,
                output_text: chunk_text,
                alignment_language,
            });
        }

        processed_audio_ms = processed_audio_ms.saturating_add(chunk.duration_ms());
        processed_asr_work_ms = processed_asr_work_ms.saturating_add(chunk_work_ms(*chunk));
        let after_ratio = processed_asr_work_ms as f64 / total_asr_work_ms as f64;
        let after_percent = progress_between(
            progress_between(range_start, range_end, TRANSCRIBE_PHASE_START),
            range_end,
            after_ratio,
        );
        let after_eta_ms =
            estimate_remaining_ms(asr_started, processed_asr_work_ms, total_asr_work_ms);
        emit_chunk_progress(
            app,
            progress_started,
            audio_path,
            file_index,
            total_files,
            chunk_index,
            total_chunks,
            *chunk,
            processed_audio_ms.min(total_chunk_audio_ms),
            total_chunk_audio_ms,
            skipped_silence_ms,
            after_percent,
            after_eta_ms,
            "transcribingSegments",
            &format!("完成第 {chunk_index} / {total_chunks} 個有聲片段"),
            Some(vad_estimated_segments.clone()),
            Some(timings.clone()),
        );
    }

    let duration_ms = started.elapsed().as_millis();
    timings.total_ms = duration_ms;

    Ok(PendingTranscription {
        audio_path: audio_path.to_string(),
        options: options.clone(),
        file_index,
        total_files,
        range_start: range_end,
        range_end,
        total_chunks,
        total_speech_ms: total_chunk_audio_ms,
        skipped_silence_ms,
        transcript_parts,
        vad_estimated_segments,
        chunks: pending_chunks,
        duration_ms,
        timings,
    })
}

fn finalize_transcription_with_context(
    app: &AppHandle,
    progress_started: Instant,
    aligner: Option<&ForcedAlignerInference>,
    mut pending: PendingTranscription,
    cancel: &CancellationToken,
) -> AppResult<TranscriptionResult> {
    check_cancelled(cancel)?;
    let alignment_started = Instant::now();
    let vad_estimated_segments = pending.vad_estimated_segments.clone();
    let output_language = normalize_output_language(pending.options.language.as_deref());
    let total_alignment_ms = pending
        .chunks
        .iter()
        .map(|chunk| chunk.range.duration_ms())
        .sum::<u64>()
        .max(1);
    let mut processed_alignment_ms = 0u64;
    let mut segments = Vec::new();

    for chunk in pending.chunks {
        check_cancelled(cancel)?;
        let before_ratio = processed_alignment_ms as f64 / total_alignment_ms as f64;
        let before_percent = progress_between(pending.range_start, pending.range_end, before_ratio);
        let before_eta_ms = estimate_remaining_ms(
            alignment_started,
            processed_alignment_ms,
            total_alignment_ms,
        );
        let aligned_segments =
            if let (Some(aligner), Some(language)) = (aligner, chunk.alignment_language) {
                emit_chunk_progress(
                    app,
                    progress_started,
                    &pending.audio_path,
                    pending.file_index,
                    pending.total_files,
                    chunk.chunk_index,
                    pending.total_chunks,
                    chunk.range,
                    processed_alignment_ms,
                    pending.total_speech_ms,
                    pending.skipped_silence_ms,
                    before_percent,
                    before_eta_ms,
                    "aligningTimestamps",
                    &format!(
                        "對齊第 {} / {} 個片段的時間戳",
                        chunk.chunk_index, pending.total_chunks
                    ),
                    Some(vad_estimated_segments.clone()),
                    Some(pending.timings.clone()),
                );
                let chunk_samples_f32 = normalized_i16_to_f32(&chunk.samples);
                let aligned_units = aligner
                    .align_samples(&chunk_samples_f32, &chunk.raw_text, language)
                    .map_err(|error| {
                        AppError::Transcription(format!(
                            "Qwen3 ForcedAligner failed on chunk {}/{}: {error}",
                            chunk.chunk_index, pending.total_chunks
                        ))
                    })?;
                check_cancelled(cancel)?;
                build_aligned_segments_with_offset(
                    &chunk.raw_text,
                    &aligned_units,
                    language,
                    output_language.as_deref(),
                    chunk.range.start_ms(),
                    chunk.range.duration_ms(),
                    pending.options.segment_by_punctuation,
                )?
            } else {
                None
            };

        segments.extend(aligned_segments.unwrap_or_else(|| {
            build_approximate_segments_with_offset(
                &chunk.output_text,
                chunk.range.duration_ms(),
                chunk.range.start_ms(),
                pending.options.segment_by_punctuation,
            )
        }));
        processed_alignment_ms = processed_alignment_ms.saturating_add(chunk.range.duration_ms());
    }

    pending.timings.alignment_ms = alignment_started.elapsed().as_millis();

    emit_progress_with_metrics(
        app,
        progress_started,
        "running",
        if pending.options.write_srt {
            "writingSrt"
        } else {
            "finalizing"
        },
        if pending.options.write_srt {
            "寫入 SRT 字幕"
        } else {
            "整理轉錄結果"
        },
        Some(&pending.audio_path),
        Some(&pending.audio_path),
        pending.file_index,
        pending.total_files,
        pending.range_end,
        Some(0),
        Some(ProgressMetrics {
            chunk_index: Some(pending.total_chunks),
            total_chunks: Some(pending.total_chunks),
            chunk_start_ms: None,
            chunk_end_ms: None,
            processed_audio_ms: Some(pending.total_speech_ms),
            total_speech_ms: Some(pending.total_speech_ms),
            skipped_silence_ms: Some(pending.skipped_silence_ms),
            partial_segments: Some(vad_estimated_segments),
            timings: Some(pending.timings.clone()),
        }),
    );

    let finalize_started = Instant::now();
    let text = pending.transcript_parts.join("\n");
    check_cancelled(cancel)?;
    let srt_path = if pending.options.write_srt {
        Some(write_srt(&pending.audio_path, &pending.options, &segments)?)
    } else {
        None
    };
    pending.timings.finalize_ms = finalize_started.elapsed().as_millis();
    pending.timings.total_ms = pending
        .duration_ms
        .saturating_add(pending.timings.alignment_ms)
        .saturating_add(pending.timings.finalize_ms);

    Ok(TranscriptionResult {
        audio_path: pending.audio_path,
        text,
        segments,
        srt_path,
        duration_ms: pending.timings.total_ms,
        timings: pending.timings,
    })
}

fn total_chunk_work_ms(chunks: &[AudioRange]) -> u64 {
    chunks
        .iter()
        .map(|chunk| chunk_work_ms(*chunk))
        .sum::<u64>()
        .max(1)
}

fn chunk_work_ms(chunk: AudioRange) -> u64 {
    chunk.duration_ms().saturating_add(ASR_CHUNK_OVERHEAD_MS)
}

fn add_inference_timings(target: &mut TranscriptionTimings, source: &InferenceTimings) {
    target.asr_mel_ms = target.asr_mel_ms.saturating_add(source.mel_ms);
    target.asr_encoder_ms = target
        .asr_encoder_ms
        .saturating_add(source.audio_encoder_ms);
    target.asr_prompt_ms = target.asr_prompt_ms.saturating_add(source.prompt_ms);
    target.asr_prefill_ms = target.asr_prefill_ms.saturating_add(source.prefill_ms);
    target.asr_decode_ms = target.asr_decode_ms.saturating_add(source.decode_ms);
    target.asr_postprocess_ms = target
        .asr_postprocess_ms
        .saturating_add(source.postprocess_ms);
    target.asr_generated_tokens = target
        .asr_generated_tokens
        .saturating_add(source.generated_tokens);
}

fn normalized_i16_to_f32(samples: &[i16]) -> Vec<f32> {
    const I16_SCALE: f32 = 32_768.0;
    samples
        .iter()
        .map(|&sample| sample as f32 / I16_SCALE)
        .collect()
}

fn normalize_output_language(language: Option<&str>) -> Option<String> {
    language
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto"))
        .map(str::to_string)
}

fn normalize_asr_language(output_language: Option<&str>) -> Option<String> {
    output_language
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("auto"))
        .map(|value| {
            if is_simplified_chinese_output_language(value) {
                CHINESE_ASR_LANGUAGE.to_string()
            } else {
                value.to_string()
            }
        })
}

fn should_use_forced_aligner(options: &TranscribeOptions) -> bool {
    if !options.write_srt {
        return false;
    }

    let output_language = normalize_output_language(options.language.as_deref());
    let asr_language = normalize_asr_language(output_language.as_deref());
    match asr_language.as_deref() {
        None => true,
        Some(language) => alignment_language(Some(language), "").is_some(),
    }
}

fn normalize_asr_text(result: &QwenTranscribeResult, language_forced: bool) -> String {
    let text = if language_forced {
        result.text.trim()
    } else {
        parse_auto_asr_text(&result.raw_output).unwrap_or_else(|| result.text.trim())
    };

    collapse_spaced_acronyms(text)
}

fn convert_text_for_output_language(text: String, language: Option<&str>) -> AppResult<String> {
    if should_convert_to_traditional_chinese(language) {
        convert_simplified_to_traditional_chinese(&text)
    } else {
        Ok(text)
    }
}

fn should_convert_to_traditional_chinese(language: Option<&str>) -> bool {
    language.is_some_and(|value| {
        value
            .trim()
            .eq_ignore_ascii_case(TRADITIONAL_CHINESE_LANGUAGE)
    })
}

fn is_simplified_chinese_output_language(language: &str) -> bool {
    language
        .trim()
        .eq_ignore_ascii_case(SIMPLIFIED_CHINESE_LANGUAGE)
}

fn convert_simplified_to_traditional_chinese(text: &str) -> AppResult<String> {
    if text.is_empty() {
        return Ok(String::new());
    }

    if text.contains('\0') {
        return Err(AppError::Transcription(
            "OpenCC cannot convert transcript text containing NUL bytes.".into(),
        ));
    }

    let mut converter = S2TW_CONVERTER
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| AppError::Transcription("OpenCC converter lock poisoned.".into()))?;

    if converter.is_none() {
        *converter = Some(OpenCC::new([Config::S2TW]).map_err(|error| {
            AppError::Transcription(format!("OpenCC converter initialization failed: {error}"))
        })?);
    }

    converter
        .as_ref()
        .expect("OpenCC converter should be initialized")
        .convert(text)
        .map_err(|error| AppError::Transcription(format!("OpenCC conversion failed: {error}")))
}

fn parse_auto_asr_text(raw: &str) -> Option<&str> {
    let rest = raw.trim().strip_prefix("language ")?;
    if let Some((_, text)) = rest.split_once("<asr_text>") {
        return Some(text.trim());
    }

    for language in ASR_LANGUAGES {
        if let Some(text) = rest.strip_prefix(language) {
            let text = text.trim_start();
            return Some(text.trim());
        }
    }
    None
}

fn parse_auto_asr_language(raw: &str) -> Option<&'static str> {
    let rest = raw.trim().strip_prefix("language ")?;
    ASR_LANGUAGES
        .iter()
        .copied()
        .find(|language| rest.starts_with(language))
}

fn collapse_spaced_acronyms(text: &str) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;

    while index < chars.len() {
        if chars[index].is_ascii_uppercase() {
            let mut end = index + 1;
            while end + 1 < chars.len() && chars[end] == ' ' && chars[end + 1].is_ascii_uppercase()
            {
                end += 2;
            }

            if end > index + 1 {
                let mut letter_index = index;
                while letter_index < end {
                    output.push(chars[letter_index]);
                    letter_index += 2;
                }
                index = end;
                continue;
            }
        }

        output.push(chars[index]);
        index += 1;
    }

    output
}

#[cfg(test)]
fn build_approximate_segments(text: &str, audio_ms: u64) -> Vec<TranscriptSegment> {
    build_approximate_segments_with_offset(text, audio_ms, 0, true)
}

fn build_approximate_segments_with_offset(
    text: &str,
    audio_ms: u64,
    offset_ms: u64,
    segment_by_punctuation: bool,
) -> Vec<TranscriptSegment> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let audio_ms = audio_ms.max(1);
    let mut units = transcript_units(text, segment_by_punctuation);
    if units.is_empty() {
        return Vec::new();
    }
    units = limit_segment_count(units, audio_ms);

    let total_weight = units
        .iter()
        .map(|unit| weighted_char_count(unit))
        .sum::<usize>()
        .max(1);
    let mut cursor_ms = 0u64;
    let mut segments = Vec::with_capacity(units.len());

    for (index, unit) in units.iter().enumerate() {
        let remaining_segments = units.len() - index;
        let remaining_ms = audio_ms.saturating_sub(cursor_ms).max(1);
        let end_ms = if remaining_segments == 1 {
            audio_ms.max(cursor_ms + 1)
        } else {
            let dynamic_min = (remaining_ms / remaining_segments as u64).clamp(1, MIN_SEGMENT_MS);
            let reserved_ms = dynamic_min.saturating_mul((remaining_segments - 1) as u64);
            let max_duration = remaining_ms.saturating_sub(reserved_ms).max(1);
            let proportional = ((audio_ms as f64) * (weighted_char_count(unit) as f64)
                / (total_weight as f64))
                .round() as u64;
            cursor_ms + proportional.max(dynamic_min).min(max_duration)
        };

        segments.push(TranscriptSegment {
            start_ms: offset_ms.saturating_add(cursor_ms),
            end_ms: offset_ms.saturating_add(end_ms),
            text: unit.clone(),
        });
        cursor_ms = end_ms;
    }

    segments
}

fn alignment_language(forced: Option<&str>, detected: &str) -> Option<&'static str> {
    let language = forced.unwrap_or(detected).trim();
    if language.eq_ignore_ascii_case("Chinese") {
        Some("Chinese")
    } else if language.eq_ignore_ascii_case("English") {
        Some("English")
    } else if language.eq_ignore_ascii_case("Cantonese")
        || language.to_ascii_lowercase().starts_with("cantonese (")
    {
        Some("Cantonese")
    } else if language.eq_ignore_ascii_case("French") {
        Some("French")
    } else if language.eq_ignore_ascii_case("German") {
        Some("German")
    } else if language.eq_ignore_ascii_case("Italian") {
        Some("Italian")
    } else if language.eq_ignore_ascii_case("Japanese") {
        Some("Japanese")
    } else if language.eq_ignore_ascii_case("Korean") {
        Some("Korean")
    } else if language.eq_ignore_ascii_case("Portuguese") {
        Some("Portuguese")
    } else if language.eq_ignore_ascii_case("Russian") {
        Some("Russian")
    } else if language.eq_ignore_ascii_case("Spanish") {
        Some("Spanish")
    } else {
        None
    }
}

fn build_aligned_segments_with_offset(
    text: &str,
    aligned_units: &[AlignedUnit],
    language: &str,
    output_language: Option<&str>,
    offset_ms: u64,
    audio_ms: u64,
    segment_by_punctuation: bool,
) -> AppResult<Option<Vec<TranscriptSegment>>> {
    let text_units = tokenize_alignment_units(text, language);
    if text_units.is_empty() || aligned_units.len() != text_units.len() {
        return Ok(None);
    }

    let phrase_texts = transcript_units(text, segment_by_punctuation);
    if phrase_texts.is_empty() {
        return Ok(None);
    }

    let mut aligned_cursor = 0usize;
    let mut segments = Vec::with_capacity(phrase_texts.len());
    for phrase in phrase_texts {
        let phrase_unit_count = tokenize_alignment_units(&phrase, language).len();
        if phrase_unit_count == 0 {
            continue;
        }
        let aligned_end = aligned_cursor.saturating_add(phrase_unit_count);
        if aligned_end > aligned_units.len() {
            return Ok(None);
        }

        let first = &aligned_units[aligned_cursor];
        let last = &aligned_units[aligned_end - 1];
        let local_start = first.start_ms.min(audio_ms.saturating_sub(1));
        let local_end = last
            .end_ms
            .max(local_start.saturating_add(1))
            .min(audio_ms.max(1));
        let phrase = convert_text_for_output_language(phrase, output_language)?;
        segments.push(TranscriptSegment {
            start_ms: offset_ms.saturating_add(local_start),
            end_ms: offset_ms.saturating_add(local_end),
            text: phrase,
        });
        aligned_cursor = aligned_end;
    }

    if aligned_cursor != aligned_units.len() || segments.is_empty() {
        return Ok(None);
    }

    for index in 1..segments.len() {
        let previous_end = segments[index - 1].end_ms;
        if segments[index].start_ms < previous_end {
            segments[index].start_ms = previous_end;
        }
        if segments[index].end_ms <= segments[index].start_ms {
            segments[index].end_ms = segments[index].start_ms.saturating_add(1);
        }
    }

    Ok(Some(segments))
}

fn format_duration_short(ms: u64) -> String {
    let total_seconds = (ms as f64 / 1000.0).round() as u64;
    if total_seconds < 60 {
        return format!("{total_seconds} 秒");
    }

    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes} 分 {seconds:02} 秒")
}

fn split_transcript_units(text: &str) -> Vec<String> {
    let mut units = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;

    for character in text.chars() {
        if matches!(character, '\n' | '\r') {
            push_segment_unit(&mut units, &mut current, &mut current_chars);
            continue;
        }

        if character.is_whitespace() {
            if !current.ends_with(' ') && !current.is_empty() {
                current.push(' ');
            }
            continue;
        }

        current.push(character);
        current_chars += 1;

        if is_hard_boundary(character)
            || (is_soft_boundary(character) && current_chars >= TARGET_SRT_CHARS)
        {
            push_segment_unit(&mut units, &mut current, &mut current_chars);
        }
    }

    push_segment_unit(&mut units, &mut current, &mut current_chars);
    units
}

fn transcript_units(text: &str, segment_by_punctuation: bool) -> Vec<String> {
    if segment_by_punctuation {
        split_transcript_units(text)
    } else {
        let text = text.trim();
        (!text.is_empty())
            .then(|| text.to_string())
            .into_iter()
            .collect()
    }
}

fn limit_segment_count(units: Vec<String>, audio_ms: u64) -> Vec<String> {
    let max_segments = usize::try_from(audio_ms).unwrap_or(usize::MAX).max(1);
    if units.len() <= max_segments {
        return units;
    }

    let total_units = units.len();
    let mut merged = vec![String::new(); max_segments];
    for (index, unit) in units.into_iter().enumerate() {
        let bucket = (index * max_segments) / total_units;
        merged[bucket].push_str(&unit);
    }

    merged
        .into_iter()
        .filter(|unit| !unit.trim().is_empty())
        .collect()
}

fn push_segment_unit(units: &mut Vec<String>, current: &mut String, current_chars: &mut usize) {
    let unit = current.trim();
    if !unit.is_empty() {
        units.push(unit.to_string());
    }
    current.clear();
    *current_chars = 0;
}

fn is_hard_boundary(character: char) -> bool {
    matches!(
        character,
        '。' | '，' | '！' | '？' | '!' | '?' | '；' | ';' | '\n'
    )
}

fn is_soft_boundary(character: char) -> bool {
    matches!(character, ',' | '.' | '、' | '：' | ':')
}

fn weighted_char_count(text: &str) -> usize {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .count()
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_roughly_eq(actual: f64, expected: f64) {
        let difference = (actual - expected).abs();
        assert!(
            difference < 0.000_001,
            "expected {actual} to be roughly {expected}, difference {difference}"
        );
    }

    #[test]
    fn orders_vad_asr_alignment_and_finalization_progress() {
        assert_roughly_eq(
            TRANSCRIPTION_WORK_START_PERCENT,
            MODEL_LOAD_PROGRESS_PERCENT,
        );
        assert_roughly_eq(VAD_PHASE_START, 0.0);
        assert_roughly_eq(VAD_PHASE_END - VAD_PHASE_START, 0.05);
        assert_roughly_eq(TRANSCRIBE_PHASE_START, VAD_PHASE_END);
        let asr_end = progress_between(0.0, 1.0, TRANSCRIBE_PHASE_END_WITH_ALIGNMENT);
        let alignment_start = progress_between(0.0, 1.0, ALIGN_PHASE_START);
        let alignment_end = progress_between(0.0, 1.0, ALIGN_PHASE_END);
        let no_alignment_end = progress_between(0.0, 1.0, TRANSCRIBE_PHASE_END_WITHOUT_ALIGNMENT);
        assert!(asr_end <= alignment_start);
        assert!(alignment_start <= alignment_end);
        assert!(alignment_end <= no_alignment_end);
    }

    #[test]
    fn weights_asr_progress_by_audio_duration_and_chunk_overhead() {
        let one_second = audio::ASR_SAMPLE_RATE as usize;
        let chunks = vec![
            AudioRange::new(0, one_second),
            AudioRange::new(one_second, one_second * 3),
        ];

        assert_eq!(chunk_work_ms(chunks[0]), 1_000 + ASR_CHUNK_OVERHEAD_MS);
        assert_eq!(chunk_work_ms(chunks[1]), 2_000 + ASR_CHUNK_OVERHEAD_MS);
        assert_eq!(
            total_chunk_work_ms(&chunks),
            3_000 + ASR_CHUNK_OVERHEAD_MS * 2
        );
    }

    #[test]
    fn estimates_eta_from_observed_asr_work() {
        assert_eq!(
            estimate_remaining_ms_from_elapsed(10_000, 25_000, 100_000),
            Some(30_000)
        );
        assert_eq!(estimate_remaining_ms_from_elapsed(10_000, 0, 100_000), None);
        assert_eq!(
            estimate_remaining_ms_from_elapsed(10_000, 100_000, 100_000),
            None
        );
    }

    #[test]
    fn parses_auto_language_without_eating_text_prefix() {
        let raw = "language ChineseAI幫你股票賺了很多錢嗎？";

        assert_eq!(parse_auto_asr_text(raw), Some("AI幫你股票賺了很多錢嗎？"));
    }

    #[test]
    fn parses_auto_language_with_asr_text_marker() {
        let raw = "language Chinese<asr_text>AI幫你股票賺了很多錢嗎？";

        assert_eq!(parse_auto_asr_text(raw), Some("AI幫你股票賺了很多錢嗎？"));
    }

    #[test]
    fn tracks_all_qwen_asr_languages_and_dialects() {
        assert_eq!(ASR_LANGUAGES.len(), 52);
    }

    #[test]
    fn parses_auto_dialect_with_asr_text_marker() {
        let raw = "language Cantonese (Hong Kong accent)<asr_text>今日天氣很好。";

        assert_eq!(parse_auto_asr_text(raw), Some("今日天氣很好。"));
    }

    #[test]
    fn parses_every_auto_language_without_marker() {
        for language in ASR_LANGUAGES {
            let raw = format!("language {language}轉錄文字");

            assert_eq!(
                parse_auto_asr_text(&raw),
                Some("轉錄文字"),
                "failed to parse {language}"
            );
        }
    }

    #[test]
    fn collapses_spaced_acronyms() {
        assert_eq!(
            collapse_spaced_acronyms("A I客服和S A P系統"),
            "AI客服和SAP系統"
        );
    }

    #[test]
    fn detects_chinese_language_for_traditional_conversion() {
        assert!(should_convert_to_traditional_chinese(Some("Chinese")));
        assert!(should_convert_to_traditional_chinese(Some(" chinese ")));
        assert!(!should_convert_to_traditional_chinese(Some(
            "Chinese (Simplified)"
        )));
        assert!(!should_convert_to_traditional_chinese(Some("auto")));
        assert!(!should_convert_to_traditional_chinese(Some("English")));
        assert!(!should_convert_to_traditional_chinese(None));
    }

    #[test]
    fn preserves_official_language_hint_casing() {
        assert_eq!(
            normalize_output_language(Some(" Cantonese (Hong Kong accent) ")).as_deref(),
            Some("Cantonese (Hong Kong accent)")
        );
    }

    #[test]
    fn maps_simplified_chinese_output_to_official_asr_language() {
        assert_eq!(
            normalize_asr_language(Some("Chinese (Simplified)")).as_deref(),
            Some("Chinese")
        );
        assert_eq!(
            normalize_asr_language(Some(" Cantonese (Hong Kong accent) ")).as_deref(),
            Some("Cantonese (Hong Kong accent)")
        );
        assert_eq!(normalize_asr_language(None), None);
    }

    #[test]
    fn converts_simplified_chinese_to_taiwan_traditional() {
        assert_eq!(
            convert_text_for_output_language("汉语转换".into(), Some("Chinese")).unwrap(),
            "漢語轉換"
        );
    }

    #[test]
    fn skips_conversion_for_non_chinese_output_language() {
        assert_eq!(
            convert_text_for_output_language("汉语转换".into(), Some("English")).unwrap(),
            "汉语转换"
        );
    }

    #[test]
    fn skips_conversion_for_simplified_chinese_output_language() {
        assert_eq!(
            convert_text_for_output_language("汉语转换".into(), Some("Chinese (Simplified)"))
                .unwrap(),
            "汉语转换"
        );
    }

    #[test]
    fn normalizes_auto_result_from_raw_output() {
        let result = QwenTranscribeResult {
            text: "I幫你股票賺了很多錢嗎？".into(),
            language: "ChineseA".into(),
            raw_output: "language ChineseA I幫你股票賺了很多錢嗎？".into(),
            duration_seconds: 1.0,
            timings: InferenceTimings::default(),
        };

        assert_eq!(
            normalize_asr_text(&result, false),
            "AI幫你股票賺了很多錢嗎？"
        );
    }

    #[test]
    fn splits_transcript_text_into_readable_units() {
        let units = split_transcript_units(
            "AI幫你股票賺了很多錢嗎？我們今天討論風險控管，還有資產配置。最後看實際案例。",
        );

        assert_eq!(
            units,
            vec![
                "AI幫你股票賺了很多錢嗎？",
                "我們今天討論風險控管，",
                "還有資產配置。",
                "最後看實際案例。"
            ]
        );
    }

    #[test]
    fn splits_english_only_at_punctuation_boundaries() {
        let text = "Interview Mr. Swallows. Give Mr. Swallows your full attention. Whatever Mr. Swallows says is good, and you're going to go along with it.";
        let units = split_transcript_units(text);

        assert!(units.len() > 1);
        assert!(units
            .iter()
            .take(units.len() - 1)
            .all(|unit| unit.chars().last().is_some_and(
                |character| is_hard_boundary(character) || is_soft_boundary(character)
            )));
        assert_eq!(
            units
                .iter()
                .flat_map(|unit| tokenize_alignment_units(unit, "English"))
                .collect::<Vec<_>>(),
            tokenize_alignment_units(text, "English")
        );
    }

    #[test]
    fn keeps_long_unpunctuated_text_in_one_unit() {
        let text = "This deliberately long transcript has no punctuation and must remain a single subtitle unit even after passing the former seventy two character limit";

        assert_eq!(split_transcript_units(text), vec![text]);
    }

    #[test]
    fn keeps_punctuation_in_one_unit_when_segmentation_is_disabled() {
        let text = "第一句，第二句。Third sentence!";

        assert_eq!(transcript_units(text, false), vec![text]);
    }

    #[test]
    fn preserves_complete_words_for_forced_alignment() {
        let text = "No, I completely understand. I've been trying to get this interview for nearly six months now, so I'm gonna make sure I use this one hour to the full. Make sure you only film Mr. Swallow's left side, not his right.";
        let text_units = tokenize_alignment_units(text, "English");
        let aligned = text_units
            .iter()
            .enumerate()
            .map(|(index, word)| AlignedUnit {
                text: word.clone(),
                start_ms: index as u64 * 100,
                end_ms: index as u64 * 100 + 80,
            })
            .collect::<Vec<_>>();

        let segments = build_aligned_segments_with_offset(
            text,
            &aligned,
            "English",
            Some("English"),
            0,
            aligned.len() as u64 * 100,
            true,
        )
        .unwrap()
        .expect("punctuation-based splitting should preserve alignment units");

        assert_eq!(
            segments
                .iter()
                .flat_map(|segment| tokenize_alignment_units(&segment.text, "English"))
                .collect::<Vec<_>>(),
            text_units
        );
    }

    #[test]
    fn approximate_segments_cover_audio_with_monotonic_ranges() {
        let text = "AI幫你股票賺了很多錢嗎？我們今天討論風險控管，還有資產配置。最後看實際案例。";
        let segments = build_approximate_segments(text, 10_000);

        assert!(segments.len() > 1);
        assert_eq!(segments.first().map(|segment| segment.start_ms), Some(0));
        assert_eq!(segments.last().map(|segment| segment.end_ms), Some(10_000));

        for pair in segments.windows(2) {
            assert!(pair[0].end_ms > pair[0].start_ms);
            assert_eq!(pair[0].end_ms, pair[1].start_ms);
        }

        let rebuilt = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<String>();
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn approximate_segments_use_one_cue_when_segmentation_is_disabled() {
        let text = "第一句，第二句。Third sentence!";
        let segments = build_approximate_segments_with_offset(text, 10_000, 2_000, false);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_ms, 2_000);
        assert_eq!(segments[0].end_ms, 12_000);
        assert_eq!(segments[0].text, text);
    }

    #[test]
    fn approximate_segments_keep_vad_chunk_offsets() {
        let mut segments = build_approximate_segments_with_offset("第一段。", 1_000, 0, true);
        segments.extend(build_approximate_segments_with_offset(
            "第二段。",
            1_500,
            2_500,
            true,
        ));

        assert_eq!(segments.first().map(|segment| segment.start_ms), Some(0));
        assert_eq!(segments.first().map(|segment| segment.end_ms), Some(1_000));
        assert_eq!(segments.last().map(|segment| segment.start_ms), Some(2_500));
        assert_eq!(segments.last().map(|segment| segment.end_ms), Some(4_000));
    }

    #[test]
    fn approximate_segments_keep_short_audio_ranges_valid() {
        let segments =
            build_approximate_segments("第一句。第二句。第三句。第四句。第五句。", 1_000);

        assert_eq!(segments.first().map(|segment| segment.start_ms), Some(0));
        assert_eq!(segments.last().map(|segment| segment.end_ms), Some(1_000));
        assert!(segments
            .iter()
            .all(|segment| segment.end_ms > segment.start_ms));
    }

    #[test]
    fn approximate_segments_merge_when_audio_is_too_short_for_unit_count() {
        let segments = build_approximate_segments("第一句。第二句。第三句。第四句。第五句。", 3);

        assert_eq!(segments.len(), 3);
        assert_eq!(segments.first().map(|segment| segment.start_ms), Some(0));
        assert_eq!(segments.last().map(|segment| segment.end_ms), Some(3));
        assert!(segments
            .iter()
            .all(|segment| segment.end_ms > segment.start_ms));
    }

    #[test]
    fn groups_forced_alignment_units_into_readable_subtitle_cues() {
        let aligned = vec![
            AlignedUnit {
                text: "第".into(),
                start_ms: 80,
                end_ms: 160,
            },
            AlignedUnit {
                text: "一".into(),
                start_ms: 160,
                end_ms: 240,
            },
            AlignedUnit {
                text: "句".into(),
                start_ms: 240,
                end_ms: 400,
            },
            AlignedUnit {
                text: "第".into(),
                start_ms: 560,
                end_ms: 640,
            },
            AlignedUnit {
                text: "二".into(),
                start_ms: 640,
                end_ms: 720,
            },
            AlignedUnit {
                text: "句".into(),
                start_ms: 720,
                end_ms: 880,
            },
        ];

        let segments = build_aligned_segments_with_offset(
            "第一句，第二句。",
            &aligned,
            "Chinese",
            Some("Chinese"),
            1_000,
            1_000,
            true,
        )
        .unwrap()
        .unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "第一句，");
        assert_eq!(segments[0].start_ms, 1_080);
        assert_eq!(segments[0].end_ms, 1_400);
        assert_eq!(segments[1].text, "第二句。");
        assert_eq!(segments[1].start_ms, 1_560);
        assert_eq!(segments[1].end_ms, 1_880);
    }

    #[test]
    fn forced_alignment_uses_one_cue_when_segmentation_is_disabled() {
        let text = "第一句，第二句。";
        let aligned = tokenize_alignment_units(text, "Chinese")
            .into_iter()
            .enumerate()
            .map(|(index, unit)| AlignedUnit {
                text: unit,
                start_ms: index as u64 * 100,
                end_ms: index as u64 * 100 + 80,
            })
            .collect::<Vec<_>>();

        let segments = build_aligned_segments_with_offset(
            text,
            &aligned,
            "Chinese",
            Some("Chinese"),
            1_000,
            1_000,
            false,
        )
        .unwrap()
        .unwrap();

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, text);
        assert_eq!(segments[0].start_ms, 1_000);
        assert_eq!(segments[0].end_ms, 1_580);
    }

    #[test]
    fn maps_only_officially_supported_forced_alignment_languages() {
        assert_eq!(
            alignment_language(Some("Chinese"), "forced"),
            Some("Chinese")
        );
        assert_eq!(
            alignment_language(Some("Cantonese (Hong Kong accent)"), "forced"),
            Some("Cantonese")
        );
        assert_eq!(alignment_language(None, "English"), Some("English"));
        assert_eq!(alignment_language(Some("Arabic"), "forced"), None);
    }

    #[test]
    fn resolves_auto_detected_language_before_transcript_prefix() {
        assert_eq!(
            parse_auto_asr_language("language ChineseAI幫你股票賺錢"),
            Some("Chinese")
        );
        assert_eq!(
            parse_auto_asr_language("language Cantonese (Hong Kong accent)<asr_text>今日天氣很好"),
            Some("Cantonese (Hong Kong accent)")
        );
    }

    #[test]
    fn loads_forced_aligner_only_for_supported_srt_languages() {
        let options = |language: &str, write_srt: bool| TranscribeOptions {
            model_id: "qwen3-asr-0.6b".into(),
            language: Some(language.into()),
            write_srt,
            segment_by_punctuation: true,
            output_dir: None,
        };

        assert!(should_use_forced_aligner(&options("auto", true)));
        assert!(should_use_forced_aligner(&options("Chinese", true)));
        assert!(!should_use_forced_aligner(&options("Arabic", true)));
        assert!(!should_use_forced_aligner(&options("Chinese", false)));
    }

    #[test]
    fn cancellation_control_targets_only_the_registered_task() {
        let control = TranscriptionControl::default();
        let first = control.register("first").unwrap();
        let second = control.register("second").unwrap();

        assert!(control.cancel("first"));
        assert!(first.load(Ordering::Acquire));
        assert!(!second.load(Ordering::Acquire));
        assert!(!control.cancel("missing"));

        control.remove("first");
        assert!(!control.cancel("first"));
    }

    #[test]
    fn cancellation_control_rejects_duplicate_task_ids() {
        let control = TranscriptionControl::default();
        control.register("same-task").unwrap();

        assert!(control.register("same-task").is_err());
    }
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
