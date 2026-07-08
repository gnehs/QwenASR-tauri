use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use opencc_rs::{Config, OpenCC};
use qwen3_asr::{
    inference::{AsrInference, TranscribeResult as QwenTranscribeResult},
    tensor::Device,
};
use tauri::{AppHandle, Emitter};

use crate::audio;
use crate::error::{AppError, AppResult};
use crate::models::{
    TranscribeBatchRequest, TranscribeFileRequest, TranscribeOptions, TranscriptSegment,
    TranscriptionProgress, TranscriptionResult,
};
use crate::paths::{model_dir, model_status};
use crate::srt;
use crate::vad::{self, AudioRange};

const TRANSCRIPTION_PROGRESS_EVENT: &str = "transcription-progress";
const TARGET_SRT_CHARS: usize = 42;
const MAX_SRT_CHARS: usize = 72;
const MIN_SEGMENT_MS: u64 = 900;
const CHINESE_ASR_LANGUAGE: &str = "Chinese";
const TRADITIONAL_CHINESE_LANGUAGE: &str = "chinese";
const SIMPLIFIED_CHINESE_LANGUAGE: &str = "chinese (simplified)";
const VAD_PHASE_START: f64 = 0.12;
const VAD_PHASE_RATIO: f64 = 0.05;
const VAD_PHASE_END: f64 = VAD_PHASE_START + VAD_PHASE_RATIO;
const TRANSCRIBE_PHASE_START: f64 = VAD_PHASE_END;
const TRANSCRIBE_PHASE_END: f64 = 0.99;
const FINALIZE_PHASE_START: f64 = TRANSCRIBE_PHASE_END;
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

    let engine = load_engine(&model_path)?;
    let result = transcribe_with_context(
        app,
        started,
        &engine,
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

    let engine = load_engine(&model_path)?;
    let total = request.audio_paths.len();
    let mut results = Vec::with_capacity(total);

    for (index, audio_path) in request.audio_paths.iter().enumerate() {
        let file_index = index + 1;
        let range_start = 10.0 + (index as f64 / total as f64) * 88.0;
        let range_end = 10.0 + (file_index as f64 / total as f64) * 88.0;

        let result = match transcribe_with_context(
            &app,
            started,
            &engine,
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
    let eta_ms = match state {
        "complete" => Some(0),
        "error" => None,
        _ => eta_ms.or_else(|| estimate_eta(started, percent)),
    };
    let metrics = metrics.unwrap_or_default();

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
    phase: &str,
    message: &str,
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
        None,
        Some(ProgressMetrics {
            chunk_index: Some(chunk_index),
            total_chunks: Some(total_chunks),
            chunk_start_ms: Some(chunk.start_ms()),
            chunk_end_ms: Some(chunk.end_ms()),
            processed_audio_ms: Some(processed_audio_ms),
            total_speech_ms: Some(total_speech_ms),
            skipped_silence_ms: Some(skipped_silence_ms),
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

fn load_engine(model_path: &Path) -> AppResult<AsrInference> {
    AsrInference::load(model_path, default_device()).map_err(|error| {
        AppError::Model(format!(
            "Failed to load model from {}: {error}",
            model_path.display()
        ))
    })
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

    let prepared_audio = audio::prepare_audio_for_asr(audio_path)?;
    let normalized_samples = audio::read_normalized_i16(prepared_audio.inference_path())?;
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
    let vad_analysis = vad::analyze_with_progress(&normalized_samples, |vad_progress| {
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
    let chunks = vad_analysis.chunks;
    let total_chunk_audio_ms = vad_analysis.chunk_audio_ms.max(1);
    let skipped_silence_ms = vad_analysis.skipped_silence_ms;
    let mut processed_audio_ms = 0u64;
    let mut transcript_parts = Vec::with_capacity(chunks.len());
    let mut segments = Vec::new();
    let total_chunks = chunks.len();
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
        }),
    );

    let output_language = normalize_output_language(options.language.as_deref());
    let asr_language = normalize_asr_language(output_language.as_deref());
    let language_forced = asr_language.is_some();

    for (index, chunk) in chunks.iter().enumerate() {
        let chunk_index = index + 1;
        let before_ratio = processed_audio_ms as f64 / total_chunk_audio_ms as f64;
        let before_percent = progress_between(
            progress_between(range_start, range_end, TRANSCRIBE_PHASE_START),
            progress_between(range_start, range_end, TRANSCRIBE_PHASE_END),
            before_ratio,
        );
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
            "transcribingSegments",
            &format!("轉錄第 {chunk_index} / {total_chunks} 個有聲片段"),
        );

        let chunk_audio = write_chunk_audio(&normalized_samples, *chunk)?;
        let chunk_audio_path = chunk_audio.inference_path().to_str().ok_or_else(|| {
            AppError::Transcription("Chunk audio path contains unsupported characters.".into())
        })?;
        let raw_result = engine
            .transcribe(chunk_audio_path, asr_language.as_deref())
            .map_err(|error| {
                AppError::Transcription(format!(
                    "Qwen3-ASR failed to transcribe chunk {chunk_index}/{total_chunks}: {error}"
                ))
            })?;

        let chunk_text = normalize_asr_text(&raw_result, language_forced);
        let chunk_text = convert_text_for_output_language(chunk_text, output_language.as_deref())?;
        if !chunk_text.trim().is_empty() {
            segments.extend(build_approximate_segments_with_offset(
                &chunk_text,
                chunk.duration_ms(),
                chunk.start_ms(),
            ));
            transcript_parts.push(chunk_text);
        }

        processed_audio_ms = processed_audio_ms.saturating_add(chunk.duration_ms());
        let after_ratio = processed_audio_ms as f64 / total_chunk_audio_ms as f64;
        let after_percent = progress_between(
            progress_between(range_start, range_end, TRANSCRIBE_PHASE_START),
            progress_between(range_start, range_end, TRANSCRIBE_PHASE_END),
            after_ratio,
        );
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
            "transcribingSegments",
            &format!("完成第 {chunk_index} / {total_chunks} 個有聲片段"),
        );
    }

    let finalize_percent = progress_between(range_start, range_end, FINALIZE_PHASE_START);
    emit_progress_with_metrics(
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
        Some(ProgressMetrics {
            chunk_index: Some(total_chunks),
            total_chunks: Some(total_chunks),
            chunk_start_ms: None,
            chunk_end_ms: None,
            processed_audio_ms: Some(total_chunk_audio_ms),
            total_speech_ms: Some(total_chunk_audio_ms),
            skipped_silence_ms: Some(skipped_silence_ms),
        }),
    );

    let text = transcript_parts.join("\n");
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

fn write_chunk_audio(samples: &[i16], chunk: AudioRange) -> AppResult<audio::PreparedAudio> {
    audio::write_temp_asr_wav(&samples[chunk.start_sample..chunk.end_sample])
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
    build_approximate_segments_with_offset(text, audio_ms, 0)
}

fn build_approximate_segments_with_offset(
    text: &str,
    audio_ms: u64,
    offset_ms: u64,
) -> Vec<TranscriptSegment> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let audio_ms = audio_ms.max(1);
    let mut units = split_transcript_units(text);
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

        let should_split = is_hard_boundary(character)
            || (is_soft_boundary(character) && current_chars >= TARGET_SRT_CHARS)
            || current_chars >= MAX_SRT_CHARS;

        if should_split {
            push_segment_unit(&mut units, &mut current, &mut current_chars);
        }
    }

    push_segment_unit(&mut units, &mut current, &mut current_chars);
    units
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
        '。' | '！' | '？' | '!' | '?' | '；' | ';' | '\n'
    )
}

fn is_soft_boundary(character: char) -> bool {
    matches!(character, '，' | ',' | '、' | '：' | ':')
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
    fn allocates_vad_to_five_percent_and_gives_remaining_progress_to_asr() {
        assert_roughly_eq(VAD_PHASE_END - VAD_PHASE_START, 0.05);
        assert_roughly_eq(TRANSCRIBE_PHASE_START, VAD_PHASE_END);
        assert_roughly_eq(FINALIZE_PHASE_START, TRANSCRIBE_PHASE_END);
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
                "我們今天討論風險控管，還有資產配置。",
                "最後看實際案例。"
            ]
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
