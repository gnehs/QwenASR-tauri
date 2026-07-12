use crate::tensor::{Device, Tensor};
use anyhow::{Context, Result};
use std::ops::Range;
use std::path::Path;
use std::time::Instant;

use crate::audio;
use crate::audio_encoder::AudioEncoder;
use crate::config::AsrConfig;
use crate::layers::compute_mrope_cos_sin;
use crate::mel::WhisperFeatureExtractor;
use crate::text_decoder::{KvCache, TextDecoder, PREFILL_BLOCK_SIZE};
use crate::tokenizer::{AsrTokenizer, AUDIO_PAD_TOKEN_ID, ENDOFTEXT_TOKEN_ID, IM_END_TOKEN_ID};
use crate::weights;

const MEL_SAMPLE_RATE: u32 = 16000;
// Qwen's Transformers backend uses a 512-token generation budget. Keeping
// this limit aligned with the precomputed RoPE table also prevents a missing
// EOS token from reading an empty position slice and terminating MLX.
const MAX_NEW_TOKENS: usize = 512;
// Full tokenizer.decode is linear in all generated IDs. Keep callback polling
// per-token for cancellation, but only rebuild cumulative text periodically.
const STREAMING_DECODE_INTERVAL_TOKENS: usize = 4;

#[derive(Debug, Clone, Default)]
pub struct InferenceTimings {
    pub mel_ms: u128,
    pub audio_encoder_ms: u128,
    pub prompt_ms: u128,
    pub prefill_ms: u128,
    pub decode_ms: u128,
    pub postprocess_ms: u128,
    pub total_ms: u128,
    pub generated_tokens: usize,
}

/// ASR inference engine.
pub struct AsrInference {
    audio_encoder: AudioEncoder,
    text_decoder: TextDecoder,
    mel_extractor: WhisperFeatureExtractor,
    tokenizer: AsrTokenizer,
    config: AsrConfig,
    device: Device,
}

impl AsrInference {
    /// Load model from directory containing config.json, model.safetensors, tokenizer.json
    pub fn load(model_dir: &Path, device: Device) -> Result<Self> {
        tracing::info!("Loading model from {:?}", model_dir);

        // Load config
        let config = AsrConfig::from_file(&model_dir.join("config.json"))
            .context("Failed to load config")?;

        // Load weights (supports both single-file and sharded safetensors)
        let all_weights =
            weights::load_model_weights(model_dir, device).context("Failed to load weights")?;

        tracing::info!("Loaded {} weight tensors", all_weights.len());

        // Load audio encoder
        tracing::info!("Loading audio encoder...");
        let audio_encoder = AudioEncoder::load(
            &all_weights,
            "thinker.audio_tower",
            &config.thinker_config.audio_config,
            device,
        )
        .context("Failed to load audio encoder")?;

        // Load text decoder
        tracing::info!("Loading text decoder...");
        let text_decoder = TextDecoder::load(
            &all_weights,
            "thinker.model",
            &config.thinker_config.text_config,
        )
        .context("Failed to load text decoder")?;

        // Load tokenizer
        tracing::info!("Loading tokenizer...");
        let tokenizer = AsrTokenizer::from_dir(model_dir).context("Failed to load tokenizer")?;

        // Create mel feature extractor
        let mel_extractor = WhisperFeatureExtractor::new(
            400, // n_fft
            160, // hop_length
            config.thinker_config.audio_config.num_mel_bins,
            MEL_SAMPLE_RATE,
            device,
        );

        tracing::info!("Model loaded successfully");

        Ok(Self {
            audio_encoder,
            text_decoder,
            mel_extractor,
            tokenizer,
            config,
            device,
        })
    }

    /// Transcribe an audio file.
    pub fn transcribe(&self, audio_path: &str, language: Option<&str>) -> Result<TranscribeResult> {
        self.transcribe_with_context(audio_path, "", language)
    }

    /// Transcribe an audio file with optional context that guides recognition.
    pub fn transcribe_with_context(
        &self,
        audio_path: &str,
        context: &str,
        language: Option<&str>,
    ) -> Result<TranscribeResult> {
        tracing::info!("Loading audio from {}", audio_path);
        let samples = audio::load_audio(audio_path, MEL_SAMPLE_RATE)?;

        self.transcribe_samples_with_context(&samples, context, language)
    }

    /// Transcribe decoded mono audio samples at 16 kHz.
    pub fn transcribe_samples(
        &self,
        samples: &[f32],
        language: Option<&str>,
    ) -> Result<TranscribeResult> {
        self.transcribe_samples_with_context(samples, "", language)
    }

    /// Transcribe decoded mono audio samples at 16 kHz with optional context.
    pub fn transcribe_samples_with_context(
        &self,
        samples: &[f32],
        context: &str,
        language: Option<&str>,
    ) -> Result<TranscribeResult> {
        Ok(self
            .transcribe_samples_with_context_impl::<fn(&PartialTranscription) -> StreamingControl>(
                samples, context, language, None,
            )?
            .transcription)
    }

    /// Transcribe decoded mono audio samples while reporting each generated token.
    ///
    /// The callback is polled after every non-EOS token so cancellation remains
    /// responsive. Text fields are refreshed periodically and otherwise reuse
    /// the latest parsed snapshot. Returning [`StreamingControl::Stop`] prevents
    /// the next decoder step from starting. Callers must inspect
    /// [`StreamingStatus`] to distinguish a callback stop from completion.
    pub fn transcribe_samples_with_context_streaming<F>(
        &self,
        samples: &[f32],
        context: &str,
        language: Option<&str>,
        on_partial: F,
    ) -> Result<StreamingTranscribeResult>
    where
        F: FnMut(&PartialTranscription) -> StreamingControl,
    {
        self.transcribe_samples_with_context_impl(samples, context, language, Some(on_partial))
    }

    fn transcribe_samples_with_context_impl<F>(
        &self,
        samples: &[f32],
        context: &str,
        language: Option<&str>,
        mut on_partial: Option<F>,
    ) -> Result<StreamingTranscribeResult>
    where
        F: FnMut(&PartialTranscription) -> StreamingControl,
    {
        let inference_started = Instant::now();
        let mut timings = InferenceTimings::default();

        // Step 1: Preprocess audio
        let duration_seconds = samples.len() as f64 / MEL_SAMPLE_RATE as f64;

        // Step 2: Compute mel spectrogram
        let mel_started = Instant::now();
        let mel = self.mel_extractor.extract(samples, self.device)?;
        timings.mel_ms = mel_started.elapsed().as_millis();
        let num_mel_frames = mel.size()[1] as usize;
        tracing::info!("Mel spectrogram: {} frames", num_mel_frames);

        // Step 3: Run audio encoder
        let audio_encoder_started = Instant::now();
        let audio_embeds = self.audio_encoder.forward(&mel);
        audio_embeds.eval(); // Materialize encoder output before decode phase
        timings.audio_encoder_ms = audio_encoder_started.elapsed().as_millis();
        let num_audio_tokens = audio_embeds.size()[0] as usize;
        tracing::info!("Audio encoder: {} tokens", num_audio_tokens);

        // Step 4: Build input token sequence
        let prompt_started = Instant::now();
        let (input_ids, audio_positions) =
            self.build_prompt(num_audio_tokens, context, language)?;
        let seq_len = input_ids.len();

        // Step 5: Build embeddings with audio injection
        let input_tensor = Tensor::from_slice_i64(&input_ids).to_device(self.device);
        let token_embeds = self.text_decoder.embed(&input_tensor).unsqueeze(0);

        // Audio placeholders are contiguous. Replace the whole range at once so
        // MLX does not build one full-sequence slice-update node per audio token.
        let prefix = token_embeds.narrow(1, 0, audio_positions.start as i64);
        let audio = audio_embeds.unsqueeze(0);
        let suffix = token_embeds.narrow(
            1,
            audio_positions.end as i64,
            (seq_len - audio_positions.end) as i64,
        );
        let hidden_states = Tensor::cat(&[prefix, audio, suffix], 1);
        timings.prompt_ms = prompt_started.elapsed().as_millis();

        // Step 6: Precompute MRoPE cos/sin for all positions (prefill + max decode)
        let prefill_started = Instant::now();
        let text_config = &self.config.thinker_config.text_config;
        // The decode loop and position table must share the same budget.
        let max_total_positions = seq_len + MAX_NEW_TOKENS;
        let all_positions: Vec<i64> = (0..max_total_positions as i64).collect();
        let all_pos_ids: [Vec<i64>; 3] =
            [all_positions.clone(), all_positions.clone(), all_positions];
        let (all_cos, all_sin) = compute_mrope_cos_sin(
            &all_pos_ids,
            text_config.head_dim,
            text_config.rope_theta,
            &text_config.mrope_section(),
            text_config.mrope_interleaved(),
            self.device,
        );

        // Prefill cos/sin: positions 0..seq_len
        let cos = all_cos.narrow(0, 0, seq_len as i64);
        let sin = all_sin.narrow(0, 0, seq_len as i64);

        // Step 7: Prefill
        let mut kv_cache = KvCache::new(text_config.num_hidden_layers);

        let logits = self.text_decoder.prefill_last_token(
            &hidden_states,
            &cos,
            &sin,
            &mut kv_cache,
            PREFILL_BLOCK_SIZE,
        );
        // Eval prefill output to materialize computation graph before decode loop
        logits.eval();
        timings.prefill_ms = prefill_started.elapsed().as_millis();

        // Step 8: Autoregressive generation
        let decode_started = Instant::now();
        let mut generated_ids: Vec<i64> = Vec::with_capacity(MAX_NEW_TOKENS);
        let eos_token_ids = [ENDOFTEXT_TOKEN_ID, IM_END_TOKEN_ID];
        let mut streaming_status = StreamingStatus::Completed;
        let mut latest_partial_text: Option<String> = None;

        let mut next_logits = logits.squeeze_dim(1);

        let mut current_pos = seq_len;
        // Derive the loop bound from the actual table capacity so future
        // budget changes cannot make decode positions diverge again.
        for _ in seq_len..max_total_positions {
            let next_token_tensor = next_logits.argmax(-1, false);
            let next_token = next_token_tensor.int64_value(&[0]);

            if eos_token_ids.contains(&next_token) {
                break;
            }

            generated_ids.push(next_token);

            if let Some(on_partial) = on_partial.as_mut() {
                if should_refresh_streaming_text(generated_ids.len()) {
                    latest_partial_text = Some(self.tokenizer.decode(&generated_ids)?);
                }
                let raw_text = latest_partial_text
                    .as_deref()
                    .expect("the first generated token always refreshes streaming text");
                if dispatch_partial(
                    raw_text,
                    language.is_some(),
                    next_token,
                    generated_ids.len(),
                    on_partial,
                ) == StreamingControl::Stop
                {
                    streaming_status = StreamingStatus::StoppedByCallback;
                    break;
                }
            }

            let next_hidden = self.text_decoder.embed(&next_token_tensor).unsqueeze(0);

            // Index into precomputed cos/sin for this position
            let new_cos = all_cos.narrow(0, current_pos as i64, 1);
            let new_sin = all_sin.narrow(0, current_pos as i64, 1);

            // Single-token decode: causal mask is all-zeros (no masking needed)
            next_logits =
                self.text_decoder
                    .forward(&next_hidden, &new_cos, &new_sin, &mut kv_cache, None);
            next_logits = next_logits.squeeze_dim(1);

            current_pos += 1;
        }
        timings.decode_ms = decode_started.elapsed().as_millis();

        // Step 9: Parse output
        tracing::info!("Generated {} tokens", generated_ids.len());
        let postprocess_started = Instant::now();
        let raw_text = self.tokenizer.decode(&generated_ids)?;
        tracing::debug!("Raw output: {:?}", raw_text);
        let (language_detected, transcription) = parse_asr_output(&raw_text, language.is_some());
        timings.postprocess_ms = postprocess_started.elapsed().as_millis();
        timings.generated_tokens = generated_ids.len();
        timings.total_ms = inference_started.elapsed().as_millis();

        Ok(StreamingTranscribeResult {
            transcription: TranscribeResult {
                text: transcription,
                language: language_detected,
                raw_output: raw_text,
                duration_seconds,
                timings,
            },
            status: streaming_status,
        })
    }

    fn build_prompt(
        &self,
        num_audio_tokens: usize,
        context: &str,
        language: Option<&str>,
    ) -> Result<(Vec<i64>, Range<usize>)> {
        let mut tokens: Vec<i64> = vec![
            151644, // <|im_start|>
            8948,   // system
            198,    // \n
        ];
        tokens.extend(self.tokenizer.encode(context.trim())?);
        tokens.extend_from_slice(&[
            151645, // <|im_end|>
            198,    // \n
            151644, // <|im_start|>
            872,    // user
            198,    // \n
            151669, // <|audio_start|>
        ]);

        let audio_start_pos = tokens.len();
        for _ in 0..num_audio_tokens {
            tokens.push(AUDIO_PAD_TOKEN_ID);
        }
        let audio_positions = audio_start_pos..audio_start_pos + num_audio_tokens;

        tokens.extend_from_slice(&[
            151670, // <|audio_end|>
            151645, // <|im_end|>
            198,    // \n
            151644, // <|im_start|>
        ]);

        if let Some(lang) = language {
            tokens.push(77091); // assistant
            tokens.push(198); // \n
            let prefix = format!("language {}", capitalize_first(lang));
            tokens.extend(self.tokenizer.encode(&prefix)?);
        } else {
            tokens.push(77091); // assistant
            tokens.push(198); // \n
        }

        Ok((tokens, audio_positions))
    }
}

/// Result of ASR transcription.
pub struct TranscribeResult {
    pub text: String,
    pub language: String,
    pub raw_output: String,
    pub duration_seconds: f64,
    pub timings: InferenceTimings,
}

/// Control returned by a streaming transcription callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingControl {
    Continue,
    Stop,
}

/// Indicates whether decoding completed normally or was stopped by a callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingStatus {
    Completed,
    StoppedByCallback,
}

/// Cumulative parsed transcription emitted after a generated token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialTranscription {
    pub text: String,
    pub language: String,
    pub raw_output: String,
    pub latest_token_id: i64,
    pub generated_tokens: usize,
}

/// Result of a streaming transcription, including its termination status.
pub struct StreamingTranscribeResult {
    pub transcription: TranscribeResult,
    pub status: StreamingStatus,
}

fn dispatch_partial<F>(
    raw_output: &str,
    language_forced: bool,
    latest_token_id: i64,
    generated_tokens: usize,
    on_partial: &mut F,
) -> StreamingControl
where
    F: FnMut(&PartialTranscription) -> StreamingControl,
{
    let (language, text) = parse_asr_output(raw_output, language_forced);
    on_partial(&PartialTranscription {
        text,
        language,
        raw_output: raw_output.to_string(),
        latest_token_id,
        generated_tokens,
    })
}

fn should_refresh_streaming_text(generated_tokens: usize) -> bool {
    generated_tokens == 1 || generated_tokens % STREAMING_DECODE_INTERVAL_TOKENS == 0
}

fn parse_asr_output(raw: &str, language_forced: bool) -> (String, String) {
    if language_forced {
        return ("forced".to_string(), raw.trim().to_string());
    }

    let raw = raw.trim();

    if let Some(rest) = raw.strip_prefix("language ") {
        if let Some(asr_pos) = rest.find("<asr_text>") {
            let lang = rest[..asr_pos].trim().to_string();
            let text = rest[asr_pos + "<asr_text>".len()..].trim().to_string();
            return (lang, text);
        }
        let mut lang_end = 0;
        for (i, c) in rest.char_indices() {
            if c.is_whitespace() || !c.is_alphabetic() {
                lang_end = i;
                break;
            }
            lang_end = i + c.len_utf8();
        }
        if lang_end > 0 {
            let lang = rest[..lang_end].to_string();
            let text = rest[lang_end..].trim().to_string();
            return (lang, text);
        }
    }

    ("unknown".to_string(), raw.to_string())
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_partial_emits_cumulative_parsed_snapshots_and_stops() {
        let mut snapshots = Vec::new();
        let mut callback = |partial: &PartialTranscription| {
            snapshots.push(partial.clone());
            if partial.generated_tokens == 2 {
                StreamingControl::Stop
            } else {
                StreamingControl::Continue
            }
        };

        assert_eq!(
            dispatch_partial(
                "language English<asr_text>Hello",
                false,
                101,
                1,
                &mut callback,
            ),
            StreamingControl::Continue
        );
        assert_eq!(
            dispatch_partial(
                "language English<asr_text>Hello world",
                false,
                202,
                2,
                &mut callback,
            ),
            StreamingControl::Stop
        );

        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].language, "English");
        assert_eq!(snapshots[0].text, "Hello");
        assert_eq!(snapshots[0].latest_token_id, 101);
        assert_eq!(snapshots[1].text, "Hello world");
        assert_eq!(
            snapshots[1].raw_output,
            "language English<asr_text>Hello world"
        );
    }

    #[test]
    fn dispatch_partial_marks_forced_language_without_stripping_text() {
        let mut snapshot = None;
        let control = dispatch_partial(" hello ", true, 42, 1, &mut |partial| {
            snapshot = Some(partial.clone());
            StreamingControl::Continue
        });

        assert_eq!(control, StreamingControl::Continue);
        let snapshot = snapshot.expect("callback should receive a snapshot");
        assert_eq!(snapshot.language, "forced");
        assert_eq!(snapshot.text, "hello");
        assert_eq!(snapshot.generated_tokens, 1);
    }

    #[test]
    fn streaming_text_refresh_is_throttled_but_starts_immediately() {
        let refreshes: Vec<usize> = (1..=10)
            .filter(|&tokens| should_refresh_streaming_text(tokens))
            .collect();

        assert_eq!(refreshes, vec![1, 4, 8]);
    }
}
