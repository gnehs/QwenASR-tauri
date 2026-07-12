use std::{
    io::Cursor,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, OnceLock,
    },
    thread,
};

use rustfft::{num_complex::Complex32, Fft, FftPlanner};
use serde::Deserialize;
use tract_onnx::prelude::*;

use crate::audio;
use crate::error::{AppError, AppResult};

type VadOnnxModel = TypedRunnableModel<TypedModel>;

static VAD_ONNX_MODEL: OnceLock<Result<Arc<VadOnnxModel>, String>> = OnceLock::new();
static VAD_CMVN: OnceLock<Result<Cmvn, String>> = OnceLock::new();
static VAD_MEL_FILTERBANK: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
static VAD_HANNING_WINDOW: OnceLock<Vec<f32>> = OnceLock::new();
static VAD_FFT: OnceLock<Arc<dyn Fft<f32>>> = OnceLock::new();

const MODEL_ONNX: &[u8] = include_bytes!("../resources/firered-vad-onnx/model.onnx");
const CMVN_JSON: &str = include_str!("../resources/firered-vad-onnx/cmvn.json");

const FRAME_LENGTH_MS: u64 = 25;
const FRAME_SHIFT_MS: u64 = 10;
const FRAME_LENGTH_S: f32 = 0.025;
const FRAME_SHIFT_S: f32 = 0.010;
const NUM_MEL_BINS: usize = 80;
const N_FFT: usize = 512;
const PRE_EMPHASIS: f32 = 0.97;
const ONNX_WINDOW_FRAMES: usize = 1_500;
const ONNX_OVERLAP_FRAMES: usize = 100;
const MAX_ONNX_INFERENCE_WORKERS: usize = 4;

const SPEECH_THRESHOLD: f32 = 0.4;
const SMOOTH_WINDOW_SIZE: usize = 5;
const MIN_SPEECH_FRAMES: usize = 20;
const MAX_SPEECH_FRAMES: usize = 2_000;
const MIN_SILENCE_FRAMES: usize = 20;
const MERGE_SILENCE_FRAMES: usize = 0;
const EXTEND_SPEECH_FRAMES: usize = 0;

const BOUNDARY_PAD_MS: u64 = 100;
const SPLIT_AFTER_SILENCE_MS: u64 = 800;
const MAX_CHUNK_MS: u64 = 240_000;
const MIN_CHUNK_MS: u64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioRange {
    pub start_sample: usize,
    pub end_sample: usize,
}

impl AudioRange {
    pub fn new(start_sample: usize, end_sample: usize) -> Self {
        Self {
            start_sample,
            end_sample: end_sample.max(start_sample),
        }
    }

    pub fn duration_samples(self) -> usize {
        self.end_sample.saturating_sub(self.start_sample)
    }

    pub fn start_ms(self) -> u64 {
        audio::samples_to_ms(self.start_sample)
    }

    pub fn end_ms(self) -> u64 {
        audio::samples_to_ms(self.end_sample)
    }

    pub fn duration_ms(self) -> u64 {
        audio::samples_to_ms(self.duration_samples())
    }
}

#[derive(Debug, Clone)]
pub struct VadAnalysis {
    pub chunks: Vec<AudioRange>,
    pub chunk_audio_ms: u64,
    pub skipped_silence_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct VadProgress {
    pub ratio: f64,
    pub message: &'static str,
}

#[derive(Debug, Deserialize)]
struct Cmvn {
    means: Vec<f32>,
    inverse_std_variances: Vec<f32>,
}

pub fn analyze_with_progress<F>(samples: &[i16], mut progress: F) -> AppResult<VadAnalysis>
where
    F: FnMut(VadProgress),
{
    emit_vad_progress(&mut progress, 0.0, "準備 FireRedVAD 分析");

    if samples.is_empty() {
        emit_vad_progress(&mut progress, 1.0, "語音片段分析完成");
        return Ok(VadAnalysis {
            chunks: Vec::new(),
            chunk_audio_ms: 0,
            skipped_silence_ms: 0,
        });
    }

    let audio_ms = audio::samples_to_ms(samples.len());
    let speech_segments = detect_speech_segments(samples, &mut progress)?;
    emit_vad_progress(&mut progress, 0.97, "整理語音片段");

    let chunks = if speech_segments.is_empty() {
        split_long_range(AudioRange::new(0, samples.len()))
    } else {
        pack_chunks(&speech_segments, samples.len())
    };
    let chunk_audio_ms = chunks.iter().map(|chunk| chunk.duration_ms()).sum::<u64>();
    emit_vad_progress(&mut progress, 1.0, "語音片段分析完成");

    Ok(VadAnalysis {
        chunks,
        chunk_audio_ms,
        skipped_silence_ms: audio_ms.saturating_sub(chunk_audio_ms),
    })
}

fn detect_speech_segments<F>(samples: &[i16], progress: &mut F) -> AppResult<Vec<AudioRange>>
where
    F: FnMut(VadProgress),
{
    emit_vad_progress(progress, 0.03, "讀取 FireRedVAD 正規化參數");
    validate_cached_cmvn()?;
    let frame_count = fbank_frame_count(samples.len());
    if frame_count == 0 {
        return Ok(Vec::new());
    }

    let probs = run_onnx(samples, frame_count, progress)?;
    emit_vad_progress(progress, 0.95, "整理 FireRedVAD 輸出");

    let decisions = VadPostprocessor::default().process(&probs);
    let segments = decisions_to_segments(&decisions, samples.len())
        .into_iter()
        .map(|range| pad_range(range, samples.len()))
        .collect::<Vec<_>>();

    Ok(merge_overlapping_ranges(segments))
}

fn fbank_frame_count(sample_count: usize) -> usize {
    let frame_len = ms_to_samples(FRAME_LENGTH_MS);
    if sample_count < frame_len {
        return 0;
    }

    1 + (sample_count - frame_len) / ms_to_samples(FRAME_SHIFT_MS)
}

fn validate_cached_cmvn() -> AppResult<()> {
    let cmvn = cached_cmvn()?;
    if cmvn.means.len() != NUM_MEL_BINS || cmvn.inverse_std_variances.len() != NUM_MEL_BINS {
        return Err(AppError::Transcription(
            "FireRedVAD CMVN parameters have an unexpected dimension.".into(),
        ));
    }
    Ok(())
}

fn extract_cmvn_fbank_window(
    samples: &[i16],
    frame_start: usize,
    frame_end: usize,
) -> AppResult<Vec<f32>> {
    let cmvn = cached_cmvn()?;
    debug_assert!(frame_start <= frame_end);
    let frame_len = ms_to_samples(FRAME_LENGTH_MS);
    let frame_shift = ms_to_samples(FRAME_SHIFT_MS);
    let mel_filterbank = cached_mel_filterbank();
    let window = cached_hanning_window(frame_len);
    let fft = cached_fft();
    let mut features = Vec::with_capacity((frame_end - frame_start) * NUM_MEL_BINS);
    let mut spectrum = vec![Complex32::new(0.0, 0.0); N_FFT];
    let mut power = vec![0.0f32; N_FFT / 2 + 1];

    for frame_index in frame_start..frame_end {
        let start = frame_index * frame_shift;
        debug_assert!(start + frame_len <= samples.len());
        spectrum.fill(Complex32::new(0.0, 0.0));
        for offset in 0..frame_len {
            spectrum[offset].re = preemphasized_sample(samples, start + offset) * window[offset];
        }

        fft.process(&mut spectrum);
        for (power_bin, spectrum_bin) in power.iter_mut().zip(spectrum.iter()) {
            *power_bin = spectrum_bin.re * spectrum_bin.re + spectrum_bin.im * spectrum_bin.im;
        }

        for mel_index in 0..NUM_MEL_BINS {
            let mel_energy = mel_filterbank[mel_index]
                .iter()
                .zip(power.iter())
                .map(|(weight, power)| weight * power)
                .sum::<f32>()
                .max(1.0);
            let log_mel = mel_energy.ln();
            features
                .push((log_mel - cmvn.means[mel_index]) * cmvn.inverse_std_variances[mel_index]);
        }
    }

    Ok(features)
}

fn run_onnx<F>(samples: &[i16], frame_count: usize, progress: &mut F) -> AppResult<Vec<f32>>
where
    F: FnMut(VadProgress),
{
    emit_vad_progress(progress, 0.08, "載入 FireRedVAD ONNX");

    let model = cached_onnx_model()?;

    emit_vad_progress(progress, 0.10, "分窗萃取特徵並執行 FireRedVAD");

    let windows = onnx_windows(frame_count);
    let worker_count = onnx_worker_count(windows.len());

    let probabilities = if worker_count == 1 {
        run_onnx_sequential(model.as_ref(), samples, frame_count, &windows, progress)?
    } else {
        run_onnx_parallel(
            model,
            samples,
            frame_count,
            &windows,
            worker_count,
            progress,
        )?
    };

    emit_vad_progress(progress, 0.93, "解讀 FireRedVAD 推論結果");
    Ok(probabilities)
}

fn load_onnx_model() -> AppResult<VadOnnxModel> {
    let mut model_bytes = Cursor::new(MODEL_ONNX);
    tract_onnx::onnx()
        .model_for_read(&mut model_bytes)
        .map_err(|error| AppError::Transcription(format!("FireRedVAD ONNX load failed: {error}")))?
        .into_optimized()
        .map_err(|error| {
            AppError::Transcription(format!("FireRedVAD ONNX optimization failed: {error}"))
        })?
        .into_runnable()
        .map_err(|error| {
            AppError::Transcription(format!("FireRedVAD ONNX compilation failed: {error}"))
        })
}

fn cached_onnx_model() -> AppResult<Arc<VadOnnxModel>> {
    match VAD_ONNX_MODEL.get_or_init(|| {
        load_onnx_model()
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }) {
        Ok(model) => Ok(Arc::clone(model)),
        Err(error) => Err(AppError::Transcription(error.clone())),
    }
}

fn run_onnx_sequential<F>(
    model: &VadOnnxModel,
    samples: &[i16],
    frame_count: usize,
    windows: &[OnnxWindow],
    progress: &mut F,
) -> AppResult<Vec<f32>>
where
    F: FnMut(VadProgress),
{
    let mut probabilities = vec![0.0f32; frame_count];
    for window in windows.iter().copied() {
        let inference = infer_onnx_window(model, samples, window)?;
        copy_window_inference(&mut probabilities, inference);

        let inference_ratio = window.output_end as f64 / frame_count as f64;
        emit_vad_progress(
            progress,
            0.10 + 0.83 * inference_ratio,
            "分窗萃取特徵並執行 FireRedVAD",
        );
    }

    Ok(probabilities)
}

fn run_onnx_parallel<F>(
    model: Arc<VadOnnxModel>,
    samples: &[i16],
    frame_count: usize,
    windows: &[OnnxWindow],
    worker_count: usize,
    progress: &mut F,
) -> AppResult<Vec<f32>>
where
    F: FnMut(VadProgress),
{
    let (sender, receiver) = mpsc::sync_channel::<AppResult<WindowInference>>(worker_count.max(1));
    let stop_requested = Arc::new(AtomicBool::new(false));

    thread::scope(|scope| {
        for worker_index in 0..worker_count {
            let sender = sender.clone();
            let model = Arc::clone(&model);
            let stop_requested = Arc::clone(&stop_requested);

            scope.spawn(move || {
                for window in assigned_onnx_windows(windows, worker_index, worker_count) {
                    if stop_requested.load(Ordering::Relaxed) {
                        break;
                    }

                    let result = infer_onnx_window(model.as_ref(), samples, window);
                    let should_stop = result.is_err();
                    if should_stop {
                        stop_requested.store(true, Ordering::Relaxed);
                    }

                    if sender.send(result).is_err() || should_stop {
                        break;
                    }
                }
            });
        }
        drop(sender);

        let mut probabilities = vec![0.0f32; frame_count];
        let mut completed_output_frames = 0usize;
        let mut first_error = None;

        for result in receiver {
            match result {
                Ok(inference) => {
                    completed_output_frames +=
                        inference.output_end.saturating_sub(inference.output_start);
                    copy_window_inference(&mut probabilities, inference);

                    let inference_ratio = completed_output_frames as f64 / frame_count as f64;
                    emit_vad_progress(
                        progress,
                        0.10 + 0.83 * inference_ratio,
                        "分窗萃取特徵並執行 FireRedVAD",
                    );
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(probabilities)
        }
    })
}

struct WindowInference {
    output_start: usize,
    output_end: usize,
    probabilities: Vec<f32>,
}

fn infer_onnx_window(
    model: &VadOnnxModel,
    samples: &[i16],
    window: OnnxWindow,
) -> AppResult<WindowInference> {
    let window_frames = window.input_end - window.input_start;
    let features = extract_cmvn_fbank_window(samples, window.input_start, window.input_end)?;
    let input = tract_ndarray::Array3::from_shape_vec((1, window_frames, NUM_MEL_BINS), features)
        .map_err(|error| {
        AppError::Transcription(format!("FireRedVAD input shape failed: {error}"))
    })?;
    let outputs = model
        .run(tvec!(input.into_tensor().into()))
        .map_err(|error| {
            AppError::Transcription(format!("FireRedVAD ONNX inference failed: {error}"))
        })?;
    let output = outputs[0].to_array_view::<f32>().map_err(|error| {
        AppError::Transcription(format!("FireRedVAD output decode failed: {error}"))
    })?;
    let window_probabilities = output.iter().copied().collect::<Vec<_>>();
    if window_probabilities.len() != window_frames {
        return Err(AppError::Transcription(format!(
            "FireRedVAD returned {} frames for a {}-frame input window.",
            window_probabilities.len(),
            window_frames
        )));
    }

    let probabilities = if window.output_start < window.output_end {
        let source_start = window.output_start - window.input_start;
        let source_end = window.output_end - window.input_start;
        window_probabilities[source_start..source_end].to_vec()
    } else {
        Vec::new()
    };

    Ok(WindowInference {
        output_start: window.output_start,
        output_end: window.output_end,
        probabilities,
    })
}

fn copy_window_inference(probabilities: &mut [f32], inference: WindowInference) {
    if inference.output_start < inference.output_end {
        debug_assert_eq!(
            inference.probabilities.len(),
            inference.output_end - inference.output_start
        );
        probabilities[inference.output_start..inference.output_end]
            .copy_from_slice(&inference.probabilities);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OnnxWindow {
    input_start: usize,
    input_end: usize,
    output_start: usize,
    output_end: usize,
}

fn onnx_windows(frame_count: usize) -> Vec<OnnxWindow> {
    if frame_count == 0 {
        return Vec::new();
    }

    let stride = ONNX_WINDOW_FRAMES
        .saturating_sub(ONNX_OVERLAP_FRAMES)
        .max(1);
    let trim = if frame_count <= ONNX_WINDOW_FRAMES {
        0
    } else {
        ONNX_OVERLAP_FRAMES / 2
    };
    let mut windows = Vec::new();
    let mut input_start = 0usize;

    while input_start < frame_count {
        let input_end = (input_start + ONNX_WINDOW_FRAMES).min(frame_count);
        let output_start = if input_start == 0 {
            input_start
        } else {
            (input_start + trim).min(input_end)
        };
        let output_end = if input_end == frame_count {
            input_end
        } else {
            input_end.saturating_sub(trim).max(output_start)
        };

        windows.push(OnnxWindow {
            input_start,
            input_end,
            output_start,
            output_end,
        });

        if input_end == frame_count {
            break;
        }
        input_start += stride;
    }

    windows
}

fn onnx_worker_count(window_count: usize) -> usize {
    if window_count <= 1 {
        return 1;
    }

    thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(MAX_ONNX_INFERENCE_WORKERS)
        .min(window_count)
        .max(1)
}

fn assigned_onnx_windows(
    windows: &[OnnxWindow],
    worker_index: usize,
    worker_count: usize,
) -> impl Iterator<Item = OnnxWindow> + '_ {
    windows
        .iter()
        .copied()
        .skip(worker_index)
        .step_by(worker_count.max(1))
}

fn emit_vad_progress<F>(progress: &mut F, ratio: f64, message: &'static str)
where
    F: FnMut(VadProgress),
{
    progress(VadProgress {
        ratio: ratio.clamp(0.0, 1.0),
        message,
    });
}

fn parse_cmvn() -> AppResult<Cmvn> {
    serde_json::from_str(CMVN_JSON)
        .map_err(|error| AppError::Transcription(format!("FireRedVAD CMVN parse failed: {error}")))
}

fn cached_cmvn() -> AppResult<&'static Cmvn> {
    match VAD_CMVN.get_or_init(|| parse_cmvn().map_err(|error| error.to_string())) {
        Ok(cmvn) => Ok(cmvn),
        Err(error) => Err(AppError::Transcription(error.clone())),
    }
}

fn cached_mel_filterbank() -> &'static [Vec<f32>] {
    VAD_MEL_FILTERBANK.get_or_init(build_mel_filterbank)
}

fn cached_hanning_window(frame_len: usize) -> &'static [f32] {
    debug_assert_eq!(frame_len, ms_to_samples(FRAME_LENGTH_MS));
    VAD_HANNING_WINDOW.get_or_init(|| hanning_window(frame_len))
}

fn cached_fft() -> Arc<dyn Fft<f32>> {
    Arc::clone(VAD_FFT.get_or_init(|| {
        let mut planner = FftPlanner::<f32>::new();
        planner.plan_fft_forward(N_FFT)
    }))
}

#[derive(Debug, Clone)]
struct VadPostprocessor {
    smooth_window_size: usize,
    prob_threshold: f32,
    min_speech_frame: usize,
    max_speech_frame: usize,
    min_silence_frame: usize,
    merge_silence_frame: usize,
    extend_speech_frame: usize,
}

impl Default for VadPostprocessor {
    fn default() -> Self {
        Self {
            smooth_window_size: SMOOTH_WINDOW_SIZE,
            prob_threshold: SPEECH_THRESHOLD,
            min_speech_frame: MIN_SPEECH_FRAMES,
            max_speech_frame: MAX_SPEECH_FRAMES,
            min_silence_frame: MIN_SILENCE_FRAMES,
            merge_silence_frame: MERGE_SILENCE_FRAMES,
            extend_speech_frame: EXTEND_SPEECH_FRAMES,
        }
    }
}

impl VadPostprocessor {
    fn process(&self, raw_probs: &[f32]) -> Vec<u8> {
        if raw_probs.is_empty() {
            return Vec::new();
        }

        let smoothed = self.smooth(raw_probs);
        let binary = smoothed
            .iter()
            .map(|prob| *prob >= self.prob_threshold)
            .collect::<Vec<_>>();
        let decisions = self.state_machine(&binary);
        let decisions = self.fix_start(decisions);
        let decisions = self.merge_silence(decisions);
        let decisions = self.extend_speech(decisions);
        self.split_long(decisions, raw_probs)
    }

    fn smooth(&self, probs: &[f32]) -> Vec<f32> {
        if self.smooth_window_size <= 1 {
            return probs.to_vec();
        }

        (0..probs.len())
            .map(|index| {
                let start = index.saturating_sub(self.smooth_window_size - 1);
                let values = &probs[start..=index];
                values.iter().sum::<f32>() / values.len() as f32
            })
            .collect()
    }

    fn state_machine(&self, binary: &[bool]) -> Vec<u8> {
        const SILENCE: u8 = 0;
        const POSSIBLE_SPEECH: u8 = 1;
        const SPEECH: u8 = 2;
        const POSSIBLE_SILENCE: u8 = 3;

        let mut decisions = vec![0u8; binary.len()];
        let mut state = SILENCE;
        let mut speech_start: Option<usize> = None;
        let mut silence_start: Option<usize> = None;

        for (frame, is_speech) in binary.iter().enumerate() {
            match state {
                SILENCE => {
                    if *is_speech {
                        state = POSSIBLE_SPEECH;
                        speech_start = Some(frame);
                    }
                }
                POSSIBLE_SPEECH => {
                    if *is_speech {
                        if let Some(start) = speech_start {
                            if frame.saturating_sub(start) >= self.min_speech_frame {
                                state = SPEECH;
                                decisions[start..frame].fill(1);
                            }
                        }
                    } else {
                        state = SILENCE;
                        speech_start = None;
                    }
                }
                SPEECH => {
                    if !*is_speech {
                        state = POSSIBLE_SILENCE;
                        silence_start = Some(frame);
                    }
                }
                POSSIBLE_SILENCE => {
                    if !*is_speech {
                        if let Some(start) = silence_start {
                            if frame.saturating_sub(start) >= self.min_silence_frame {
                                state = SILENCE;
                                speech_start = None;
                            }
                        }
                    } else {
                        state = SPEECH;
                        silence_start = None;
                    }
                }
                _ => {}
            }

            decisions[frame] = u8::from(matches!(state, SPEECH | POSSIBLE_SILENCE));
        }

        decisions
    }

    fn fix_start(&self, decisions: Vec<u8>) -> Vec<u8> {
        let mut fixed = decisions.clone();
        for frame in 1..decisions.len() {
            if decisions[frame - 1] == 0 && decisions[frame] == 1 {
                let start = frame.saturating_sub(self.smooth_window_size);
                fixed[start..frame].fill(1);
            }
        }
        fixed
    }

    fn merge_silence(&self, decisions: Vec<u8>) -> Vec<u8> {
        if self.merge_silence_frame == 0 {
            return decisions;
        }

        let mut merged = decisions.clone();
        let mut silence_start = None;
        for frame in 1..decisions.len() {
            if decisions[frame - 1] == 1 && decisions[frame] == 0 && silence_start.is_none() {
                silence_start = Some(frame);
            } else if decisions[frame - 1] == 0 && decisions[frame] == 1 {
                if let Some(start) = silence_start.take() {
                    if frame.saturating_sub(start) < self.merge_silence_frame {
                        merged[start..frame].fill(1);
                    }
                }
            }
        }
        merged
    }

    fn extend_speech(&self, decisions: Vec<u8>) -> Vec<u8> {
        if self.extend_speech_frame == 0 {
            return decisions;
        }

        let mut extended = decisions.clone();
        for (frame, decision) in decisions.iter().enumerate() {
            if *decision == 1 {
                let start = frame.saturating_sub(self.extend_speech_frame);
                let end = (frame + self.extend_speech_frame + 1).min(decisions.len());
                extended[start..end].fill(1);
            }
        }
        extended
    }

    fn split_long(&self, decisions: Vec<u8>, probs: &[f32]) -> Vec<u8> {
        let mut split = decisions.clone();
        for (start, end) in decision_frame_ranges(&decisions) {
            if end.saturating_sub(start) <= self.max_speech_frame {
                continue;
            }

            let mut cursor = start;
            while end.saturating_sub(cursor) > self.max_speech_frame {
                let window_start = cursor + self.max_speech_frame / 2;
                let window_end = (cursor + self.max_speech_frame).min(end);
                if window_start >= window_end {
                    break;
                }

                let split_offset = probs[window_start..window_end]
                    .iter()
                    .enumerate()
                    .min_by(|(_, left), (_, right)| left.total_cmp(right))
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                let split_frame = window_start + split_offset;
                split[split_frame] = 0;
                cursor = split_frame + 1;
            }
        }
        split
    }
}

fn decisions_to_segments(decisions: &[u8], total_samples: usize) -> Vec<AudioRange> {
    decision_frame_ranges(decisions)
        .into_iter()
        .map(|(start, end)| {
            let start_sample = seconds_to_samples(start as f32 * FRAME_SHIFT_S);
            let mut end_seconds = end as f32 * FRAME_SHIFT_S;
            if end == decisions.len() {
                end_seconds += FRAME_LENGTH_S;
            }
            let end_sample = seconds_to_samples(end_seconds).min(total_samples);
            AudioRange::new(start_sample.min(total_samples), end_sample)
        })
        .filter(|range| range.duration_samples() > 0)
        .collect()
}

fn decision_frame_ranges(decisions: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = None;

    for (frame, decision) in decisions.iter().enumerate() {
        match (start, *decision) {
            (None, 1) => start = Some(frame),
            (Some(range_start), 0) => {
                ranges.push((range_start, frame));
                start = None;
            }
            _ => {}
        }
    }

    if let Some(range_start) = start {
        ranges.push((range_start, decisions.len()));
    }

    ranges
}

fn build_mel_filterbank() -> Vec<Vec<f32>> {
    let n_freqs = N_FFT / 2 + 1;
    let freq_bins = (0..n_freqs)
        .map(|index| index as f32 * (audio::ASR_SAMPLE_RATE as f32 / 2.0) / (n_freqs - 1) as f32)
        .collect::<Vec<_>>();
    let mel_min = hz_to_mel(0.0);
    let mel_max = hz_to_mel(audio::ASR_SAMPLE_RATE as f32 / 2.0);
    let mel_points = (0..NUM_MEL_BINS + 2)
        .map(|index| mel_min + (mel_max - mel_min) * index as f32 / (NUM_MEL_BINS + 1) as f32)
        .map(mel_to_hz)
        .collect::<Vec<_>>();
    let mut filters = vec![vec![0.0f32; n_freqs]; NUM_MEL_BINS];

    for mel_index in 0..NUM_MEL_BINS {
        let left = mel_points[mel_index];
        let center = mel_points[mel_index + 1];
        let right = mel_points[mel_index + 2];
        for (freq_index, frequency) in freq_bins.iter().enumerate() {
            filters[mel_index][freq_index] = if (left..=center).contains(frequency) {
                (frequency - left) / (center - left)
            } else if *frequency > center && *frequency <= right {
                (right - frequency) / (right - center)
            } else {
                0.0
            };
        }
    }

    filters
}

fn hanning_window(frame_len: usize) -> Vec<f32> {
    if frame_len <= 1 {
        return vec![1.0; frame_len];
    }

    (0..frame_len)
        .map(|index| {
            0.5 - 0.5 * ((2.0 * std::f32::consts::PI * index as f32) / (frame_len - 1) as f32).cos()
        })
        .collect()
}

fn preemphasized_sample(samples: &[i16], index: usize) -> f32 {
    let value = samples[index] as f32;
    if index == 0 {
        value
    } else {
        value - PRE_EMPHASIS * samples[index - 1] as f32
    }
}

fn merge_overlapping_ranges(mut ranges: Vec<AudioRange>) -> Vec<AudioRange> {
    ranges.sort_by_key(|range| range.start_sample);
    let mut merged: Vec<AudioRange> = Vec::new();

    for range in ranges {
        if range.duration_samples() == 0 {
            continue;
        }

        if let Some(last) = merged.last_mut() {
            if range.start_sample <= last.end_sample {
                last.end_sample = last.end_sample.max(range.end_sample);
                continue;
            }
        }

        merged.push(range);
    }

    merged
}

fn pack_chunks(segments: &[AudioRange], total_samples: usize) -> Vec<AudioRange> {
    let max_chunk_samples = ms_to_samples(MAX_CHUNK_MS);
    // Each detected speech range already includes boundary padding. Subtract both
    // pads so the configured threshold represents silence in the source audio.
    let max_gap_ms = SPLIT_AFTER_SILENCE_MS.saturating_sub(BOUNDARY_PAD_MS.saturating_mul(2));
    let max_gap_samples = ms_to_samples(max_gap_ms);
    let mut chunks = Vec::new();
    let mut current: Option<AudioRange> = None;

    for segment in segments {
        match current {
            None => current = Some(*segment),
            Some(mut chunk) => {
                let gap = segment.start_sample.saturating_sub(chunk.end_sample);
                let merged_duration = segment.end_sample.saturating_sub(chunk.start_sample);
                if gap <= max_gap_samples && merged_duration <= max_chunk_samples {
                    chunk.end_sample = segment.end_sample;
                    current = Some(chunk);
                } else {
                    push_split_chunks(&mut chunks, chunk, total_samples);
                    current = Some(*segment);
                }
            }
        }
    }

    if let Some(chunk) = current {
        push_split_chunks(&mut chunks, chunk, total_samples);
    }

    compact_tiny_tail_chunks(chunks)
}

fn push_split_chunks(chunks: &mut Vec<AudioRange>, range: AudioRange, total_samples: usize) {
    let max_chunk_samples = ms_to_samples(MAX_CHUNK_MS);
    let mut start = range.start_sample.min(total_samples);
    let end = range.end_sample.min(total_samples).max(start);

    while end.saturating_sub(start) > max_chunk_samples {
        let split_end = (start + max_chunk_samples).min(total_samples);
        chunks.push(AudioRange::new(start, split_end));
        start = split_end;
    }

    if end > start {
        chunks.push(AudioRange::new(start, end));
    }
}

fn split_long_range(range: AudioRange) -> Vec<AudioRange> {
    let mut chunks = Vec::new();
    push_split_chunks(&mut chunks, range, range.end_sample);
    compact_tiny_tail_chunks(chunks)
}

fn compact_tiny_tail_chunks(mut chunks: Vec<AudioRange>) -> Vec<AudioRange> {
    let min_chunk_samples = ms_to_samples(MIN_CHUNK_MS);
    if chunks.len() < 2 {
        return chunks;
    }

    let last_is_tiny = chunks
        .last()
        .is_some_and(|chunk| chunk.duration_samples() < min_chunk_samples);
    if !last_is_tiny {
        return chunks;
    }

    let mut tail = chunks.pop().expect("tail should exist");
    if let Some(previous) = chunks.last_mut() {
        let missing = min_chunk_samples.saturating_sub(tail.duration_samples());
        if previous.end_sample == tail.start_sample && previous.duration_samples() > missing {
            previous.end_sample -= missing;
            tail.start_sample -= missing;
        }
    }
    chunks.push(tail);
    chunks
}

fn pad_range(range: AudioRange, total_samples: usize) -> AudioRange {
    let pad = ms_to_samples(BOUNDARY_PAD_MS);
    AudioRange::new(
        range.start_sample.saturating_sub(pad),
        range.end_sample.saturating_add(pad).min(total_samples),
    )
}

fn ms_to_samples(ms: u64) -> usize {
    ((audio::ASR_SAMPLE_RATE as u64 * ms) / 1000) as usize
}

fn seconds_to_samples(seconds: f32) -> usize {
    (seconds.max(0.0) * audio::ASR_SAMPLE_RATE as f32).round() as usize
}

fn hz_to_mel(hz: f32) -> f32 {
    1127.0 * (1.0 + hz / 700.0).ln()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * ((mel / 1127.0).exp() - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_packing_skips_long_silence_around_detected_speech() {
        let samples = vec![0i16; ms_to_samples(3_000)];
        let speech = pad_range(
            AudioRange::new(ms_to_samples(1_000), ms_to_samples(2_000)),
            samples.len(),
        );
        let chunks = pack_chunks(&[speech], samples.len());
        let chunk_audio_ms = chunks.iter().map(|chunk| chunk.duration_ms()).sum::<u64>();
        let skipped_silence_ms = audio::samples_to_ms(samples.len()).saturating_sub(chunk_audio_ms);

        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].start_ms() < 1_000);
        assert!(chunks[0].end_ms() > 2_000);
        assert!(skipped_silence_ms > 1_000);
    }

    #[test]
    fn chunk_packing_splits_after_a_natural_pause() {
        let total_samples = ms_to_samples(10_000);
        let first = pad_range(
            AudioRange::new(ms_to_samples(1_000), ms_to_samples(2_000)),
            total_samples,
        );
        let short_pause = pad_range(
            AudioRange::new(ms_to_samples(2_790), ms_to_samples(3_790)),
            total_samples,
        );
        let natural_pause = pad_range(
            AudioRange::new(ms_to_samples(2_810), ms_to_samples(3_810)),
            total_samples,
        );

        assert_eq!(pack_chunks(&[first, short_pause], total_samples).len(), 1);
        assert_eq!(pack_chunks(&[first, natural_pause], total_samples).len(), 2);
    }

    #[test]
    fn falls_back_to_whole_audio_when_no_speech_is_detected() {
        let samples = vec![0i16; ms_to_samples(2_000)];
        let mut reports = Vec::new();
        let analysis =
            analyze_with_progress(&samples, |progress| reports.push(progress.ratio)).unwrap();

        assert_eq!(analysis.chunks, vec![AudioRange::new(0, samples.len())]);
        assert_eq!(analysis.skipped_silence_ms, 0);
        assert!(reports.iter().any(|ratio| *ratio > 0.0 && *ratio < 1.0));
        assert_eq!(reports.last().copied(), Some(1.0));
        assert!(reports.windows(2).all(|window| window[0] <= window[1]));
    }

    #[test]
    fn caps_continuous_speech_at_four_minutes() {
        let chunks = split_long_range(AudioRange::new(0, ms_to_samples(10 * 60_000)));

        assert_eq!(chunks.len(), 3);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.duration_ms() <= MAX_CHUNK_MS));
    }

    #[test]
    fn extracts_expected_number_of_frames() {
        let samples = vec![1_000i16; ms_to_samples(1_000)];
        let frame_count = fbank_frame_count(samples.len());
        let features = extract_cmvn_fbank_window(&samples, 0, frame_count).unwrap();

        assert_eq!(frame_count, 98);
        assert_eq!(features.len(), 98 * NUM_MEL_BINS);
    }

    #[test]
    fn windowed_features_preserve_absolute_overlap_frames() {
        let frame_count = ONNX_WINDOW_FRAMES + ONNX_OVERLAP_FRAMES + 7;
        let sample_count =
            ms_to_samples(FRAME_LENGTH_MS) + (frame_count - 1) * ms_to_samples(FRAME_SHIFT_MS);
        let samples = (0..sample_count)
            .map(|index| ((index % 4_001) as i32 - 2_000) as i16)
            .collect::<Vec<_>>();
        let all = extract_cmvn_fbank_window(&samples, 0, frame_count).unwrap();
        let overlap_start = ONNX_WINDOW_FRAMES - ONNX_OVERLAP_FRAMES;
        let overlap = extract_cmvn_fbank_window(&samples, overlap_start, frame_count).unwrap();

        assert_eq!(
            overlap,
            all[overlap_start * NUM_MEL_BINS..frame_count * NUM_MEL_BINS]
        );
    }

    #[test]
    fn onnx_windows_bound_high_dimensional_feature_memory() {
        let windows = onnx_windows(100_000);
        let max_window_frames = windows
            .iter()
            .map(|window| window.input_end - window.input_start)
            .max()
            .unwrap();

        assert_eq!(max_window_frames, ONNX_WINDOW_FRAMES);
        assert_eq!(
            max_window_frames * NUM_MEL_BINS * std::mem::size_of::<f32>(),
            480_000
        );
    }

    #[test]
    fn parallel_windowed_onnx_matches_sequential_overlap_output() {
        let frame_count = ONNX_WINDOW_FRAMES + 1;
        let sample_count =
            ms_to_samples(FRAME_LENGTH_MS) + (frame_count - 1) * ms_to_samples(FRAME_SHIFT_MS);
        let samples = vec![0i16; sample_count];
        let windows = onnx_windows(frame_count);
        let model = cached_onnx_model().unwrap();
        let expected =
            run_onnx_sequential(model.as_ref(), &samples, frame_count, &windows, &mut |_| {})
                .unwrap();
        let actual =
            run_onnx_parallel(model, &samples, frame_count, &windows, 2, &mut |_| {}).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn computes_preemphasis_without_materializing_the_audio() {
        let samples = [1_000i16, 2_000, -1_000, 500];
        let expected = [
            1_000.0,
            2_000.0 - PRE_EMPHASIS * 1_000.0,
            -1_000.0 - PRE_EMPHASIS * 2_000.0,
            500.0 - PRE_EMPHASIS * -1_000.0,
        ];

        for (index, expected) in expected.into_iter().enumerate() {
            assert_eq!(preemphasized_sample(&samples, index), expected);
        }
    }

    #[test]
    fn reuses_cached_vad_resources() {
        let first_cmvn = cached_cmvn().unwrap();
        let second_cmvn = cached_cmvn().unwrap();
        let first_filters = cached_mel_filterbank();
        let second_filters = cached_mel_filterbank();
        let first_fft = cached_fft();
        let second_fft = cached_fft();

        assert!(std::ptr::eq(first_cmvn, second_cmvn));
        assert!(std::ptr::eq(first_filters, second_filters));
        assert!(Arc::ptr_eq(&first_fft, &second_fft));
    }

    #[test]
    fn reuses_compiled_onnx_model() {
        let first = cached_onnx_model().unwrap();
        let second = cached_onnx_model().unwrap();

        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn onnx_windows_cover_each_frame_once() {
        for frame_count in [1, 100, 1_499, 1_500, 1_501, 2_999, 10_653] {
            let windows = onnx_windows(frame_count);
            let mut covered = vec![false; frame_count];

            for window in windows {
                assert!(window.input_start < window.input_end);
                assert!(window.input_end <= frame_count);
                assert!(window.input_start <= window.output_start);
                assert!(window.output_start <= window.output_end);
                assert!(window.output_end <= window.input_end);

                for covered_frame in &mut covered[window.output_start..window.output_end] {
                    assert!(!*covered_frame);
                    *covered_frame = true;
                }
            }

            assert!(
                covered.iter().all(|covered_frame| *covered_frame),
                "frame_count {frame_count} was not fully covered"
            );
        }
    }

    #[test]
    fn assigned_onnx_windows_partition_the_original_sequence() {
        let windows = onnx_windows(10_653);
        let worker_count = 4;
        let mut assigned = Vec::new();

        for worker_index in 0..worker_count {
            assigned.extend(assigned_onnx_windows(&windows, worker_index, worker_count));
        }
        assigned.sort_by_key(|window| window.output_start);

        assert_eq!(assigned, windows);
    }
}
