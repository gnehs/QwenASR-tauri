use std::{ops::Range, path::Path};

use qwen3_asr::audio;
use qwen3_asr::audio_encoder::AudioEncoder;
use qwen3_asr::config::ThinkerConfig;
use qwen3_asr::layers::compute_mrope_cos_sin;
use qwen3_asr::mel::WhisperFeatureExtractor;
use qwen3_asr::tensor::{Device, Tensor};
use qwen3_asr::text_decoder::{create_causal_mask, KvCache, TextDecoder};
use qwen3_asr::tokenizer::{
    AsrTokenizer, AUDIO_END_TOKEN_ID, AUDIO_PAD_TOKEN_ID, AUDIO_START_TOKEN_ID,
};
use qwen3_asr::weights;
use serde::Deserialize;

use crate::error::{AppError, AppResult};

const SAMPLE_RATE: u32 = 16_000;
const MAX_POSITION_EMBEDDINGS: usize = 8_192;

#[derive(Debug, Clone, Deserialize)]
struct ForcedAlignerConfig {
    timestamp_token_id: i64,
    timestamp_segment_time: f64,
    thinker_config: ThinkerConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignedUnit {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// Qwen3-ForcedAligner inference implemented on the same qwen3_asr tensor
/// abstraction as transcription. On macOS that abstraction is backed by MLX.
pub struct ForcedAlignerInference {
    audio_encoder: AudioEncoder,
    text_decoder: TextDecoder,
    mel_extractor: WhisperFeatureExtractor,
    tokenizer: AsrTokenizer,
    config: ForcedAlignerConfig,
    device: Device,
}

impl ForcedAlignerInference {
    pub fn load(model_dir: &Path, device: Device) -> AppResult<Self> {
        let config_path = model_dir.join("config.json");
        let config = std::fs::read_to_string(&config_path)
            .map_err(|error| {
                AppError::Model(format!("Failed to read {}: {error}", config_path.display()))
            })
            .and_then(|contents| {
                serde_json::from_str::<ForcedAlignerConfig>(&contents).map_err(|error| {
                    AppError::Model(format!(
                        "Failed to parse forced aligner config at {}: {error}",
                        config_path.display()
                    ))
                })
            })?;

        let all_weights = weights::load_model_weights(model_dir, device).map_err(|error| {
            AppError::Model(format!("Failed to load forced aligner weights: {error}"))
        })?;
        let audio_encoder = AudioEncoder::load(
            &all_weights,
            "thinker.audio_tower",
            &config.thinker_config.audio_config,
            device,
        )
        .map_err(|error| {
            AppError::Model(format!(
                "Failed to load forced aligner audio encoder: {error}"
            ))
        })?;
        let text_decoder = TextDecoder::load(
            &all_weights,
            "thinker.model",
            &config.thinker_config.text_config,
        )
        .map_err(|error| {
            AppError::Model(format!(
                "Failed to load forced aligner text decoder: {error}"
            ))
        })?;
        let tokenizer = AsrTokenizer::from_dir(model_dir).map_err(|error| {
            AppError::Model(format!("Failed to load forced aligner tokenizer: {error}"))
        })?;
        let mel_extractor = WhisperFeatureExtractor::new(
            400,
            160,
            config.thinker_config.audio_config.num_mel_bins,
            SAMPLE_RATE,
            device,
        );

        Ok(Self {
            audio_encoder,
            text_decoder,
            mel_extractor,
            tokenizer,
            config,
            device,
        })
    }

    #[allow(dead_code)]
    pub fn align(
        &self,
        audio_path: &str,
        text: &str,
        language: &str,
    ) -> AppResult<Vec<AlignedUnit>> {
        let samples = audio::load_audio(audio_path, SAMPLE_RATE).map_err(|error| {
            AppError::Transcription(format!(
                "Failed to load audio for forced alignment: {error}"
            ))
        })?;

        self.align_samples(&samples, text, language)
    }

    /// Align text against decoded mono audio samples at 16 kHz.
    pub fn align_samples(
        &self,
        samples: &[f32],
        text: &str,
        language: &str,
    ) -> AppResult<Vec<AlignedUnit>> {
        let units = tokenize_alignment_units(text, language);
        if units.is_empty() {
            return Ok(Vec::new());
        }

        let mel = self
            .mel_extractor
            .extract(samples, self.device)
            .map_err(|error| {
                AppError::Transcription(format!(
                    "Failed to compute forced aligner audio features: {error}"
                ))
            })?;
        let audio_embeds = self.audio_encoder.forward(&mel);
        audio_embeds.eval();
        let num_audio_tokens = audio_embeds.size()[0] as usize;
        let (input_ids, audio_positions) = self.build_prompt(&units, num_audio_tokens)?;
        let seq_len = input_ids.len();
        if seq_len > MAX_POSITION_EMBEDDINGS {
            return Err(AppError::ForcedAlignmentTooLong {
                tokens: seq_len,
                max_tokens: MAX_POSITION_EMBEDDINGS,
            });
        }

        let input_tensor = Tensor::from_slice_i64(&input_ids).to_device(self.device);
        let token_embeds = self.text_decoder.embed(&input_tensor).unsqueeze(0);
        let prefix = token_embeds.narrow(1, 0, audio_positions.start as i64);
        let audio = audio_embeds.unsqueeze(0);
        let suffix = token_embeds.narrow(
            1,
            audio_positions.end as i64,
            (seq_len - audio_positions.end) as i64,
        );
        let hidden_states = Tensor::cat(&[prefix, audio, suffix], 1);

        let positions = (0..seq_len as i64).collect::<Vec<_>>();
        let position_ids = [positions.clone(), positions.clone(), positions];
        let text_config = &self.config.thinker_config.text_config;
        let (cos, sin) = compute_mrope_cos_sin(
            &position_ids,
            text_config.head_dim,
            text_config.rope_theta,
            &text_config.mrope_section(),
            text_config.mrope_interleaved(),
            self.device,
        );
        let mask = create_causal_mask(seq_len as i64, 0, self.device);
        let mut cache = KvCache::new(text_config.num_hidden_layers);
        let logits = self
            .text_decoder
            .forward(&hidden_states, &cos, &sin, &mut cache, Some(&mask));
        logits.eval();

        let timestamp_bins = input_ids
            .iter()
            .enumerate()
            .filter(|(_, token_id)| **token_id == self.config.timestamp_token_id)
            .map(|(position, _)| {
                logits
                    .narrow(1, position as i64, 1)
                    .squeeze_dim(1)
                    .argmax(-1, false)
                    .int64_value(&[0])
            })
            .collect::<Vec<_>>();
        let timestamp_ms = timestamp_bins
            .into_iter()
            .map(|bin| (bin as f64 * self.config.timestamp_segment_time).round() as u64)
            .collect::<Vec<_>>();

        Ok(parse_aligned_units(&units, &timestamp_ms))
    }

    fn build_prompt(
        &self,
        units: &[String],
        num_audio_tokens: usize,
    ) -> AppResult<(Vec<i64>, Range<usize>)> {
        let mut tokens = Vec::with_capacity(num_audio_tokens + units.len() * 4 + 2);
        tokens.push(AUDIO_START_TOKEN_ID);
        let audio_start = tokens.len();
        tokens.extend(std::iter::repeat_n(AUDIO_PAD_TOKEN_ID, num_audio_tokens));
        let audio_positions = audio_start..audio_start + num_audio_tokens;
        tokens.push(AUDIO_END_TOKEN_ID);

        for unit in units {
            tokens.extend(self.tokenizer.encode(unit).map_err(|error| {
                AppError::Transcription(format!(
                    "Failed to tokenize forced alignment text: {error}"
                ))
            })?);
            tokens.push(self.config.timestamp_token_id);
            tokens.push(self.config.timestamp_token_id);
        }

        Ok((tokens, audio_positions))
    }
}

pub fn tokenize_alignment_units(text: &str, language: &str) -> Vec<String> {
    let language = language.trim().to_ascii_lowercase();
    if matches!(language.as_str(), "japanese" | "ja" | "jp") {
        return text
            .chars()
            .filter(|character| is_kept_character(*character))
            .map(|character| character.to_string())
            .collect();
    }

    text.split_whitespace()
        .flat_map(|segment| {
            let cleaned = segment
                .chars()
                .filter(|character| is_kept_character(*character))
                .collect::<String>();
            split_cjk_units(&cleaned)
        })
        .filter(|unit| !unit.is_empty())
        .collect()
}

fn is_kept_character(character: char) -> bool {
    character == '\'' || character.is_alphanumeric()
}

fn is_cjk_character(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
    )
}

fn split_cjk_units(text: &str) -> Vec<String> {
    let mut units = Vec::new();
    let mut buffer = String::new();
    for character in text.chars() {
        if is_cjk_character(character) {
            if !buffer.is_empty() {
                units.push(std::mem::take(&mut buffer));
            }
            units.push(character.to_string());
        } else {
            buffer.push(character);
        }
    }
    if !buffer.is_empty() {
        units.push(buffer);
    }
    units
}

fn parse_aligned_units(units: &[String], timestamps_ms: &[u64]) -> Vec<AlignedUnit> {
    let fixed = fix_timestamps(timestamps_ms);
    units
        .iter()
        .enumerate()
        .filter_map(|(index, unit)| {
            let start_ms = fixed.get(index * 2).copied()?;
            let end_ms = fixed.get(index * 2 + 1).copied()?;
            Some(AlignedUnit {
                text: unit.clone(),
                start_ms,
                end_ms: end_ms.max(start_ms),
            })
        })
        .collect()
}

fn fix_timestamps(timestamps: &[u64]) -> Vec<u64> {
    if timestamps.is_empty() {
        return Vec::new();
    }

    let lis_indices = longest_non_decreasing_subsequence(timestamps);
    let mut normal = vec![false; timestamps.len()];
    for index in lis_indices {
        normal[index] = true;
    }

    let mut fixed = timestamps.to_vec();
    let mut index = 0;
    while index < fixed.len() {
        if normal[index] {
            index += 1;
            continue;
        }

        let start = index;
        while index < fixed.len() && !normal[index] {
            index += 1;
        }
        let end = index;
        let anomaly_count = end - start;
        let left = (0..start)
            .rev()
            .find(|candidate| normal[*candidate])
            .map(|candidate| fixed[candidate]);
        let right = (end..fixed.len())
            .find(|candidate| normal[*candidate])
            .map(|candidate| fixed[candidate]);

        if anomaly_count <= 2 {
            for (candidate, value) in fixed.iter_mut().enumerate().take(end).skip(start) {
                *value = match (left, right) {
                    (None, Some(right)) => right,
                    (Some(left), None) => left,
                    (Some(left), Some(right)) => {
                        if candidate - (start.saturating_sub(1)) <= end - candidate {
                            left
                        } else {
                            right
                        }
                    }
                    (None, None) => *value,
                };
            }
        } else if let (Some(left), Some(right)) = (left, right) {
            for (offset, candidate) in (start..end).enumerate() {
                let ratio = (offset + 1) as f64 / (anomaly_count + 1) as f64;
                fixed[candidate] = (left as f64 + (right as f64 - left as f64) * ratio) as u64;
            }
        } else if let Some(value) = left.or(right) {
            fixed[start..end].fill(value);
        }
    }

    fixed
}

fn longest_non_decreasing_subsequence(values: &[u64]) -> Vec<usize> {
    let mut lengths = vec![1usize; values.len()];
    let mut parents = vec![None; values.len()];
    let mut best_end = 0usize;
    for index in 0..values.len() {
        for previous in 0..index {
            if values[previous] <= values[index] && lengths[previous] + 1 > lengths[index] {
                lengths[index] = lengths[previous] + 1;
                parents[index] = Some(previous);
            }
        }
        if lengths[index] > lengths[best_end] {
            best_end = index;
        }
    }

    let mut indices = Vec::new();
    let mut cursor = Some(best_end);
    while let Some(index) = cursor {
        indices.push(index);
        cursor = parents[index];
    }
    indices.reverse();
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_chinese_per_character_and_latin_per_word() {
        assert_eq!(
            tokenize_alignment_units("測試 Qwen3-ASR。", "Chinese"),
            vec!["測", "試", "Qwen3ASR"]
        );
    }

    #[test]
    fn repairs_non_monotonic_timestamps() {
        let fixed = fix_timestamps(&[0, 160, 80, 240, 320, 400]);
        assert!(fixed.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn parses_start_and_end_pairs() {
        let aligned = parse_aligned_units(&["Hello".into(), "world".into()], &[0, 240, 320, 560]);
        assert_eq!(aligned[0].text, "Hello");
        assert_eq!(aligned[1].start_ms, 320);
        assert_eq!(aligned[1].end_ms, 560);
    }
}
