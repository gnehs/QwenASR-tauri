use crate::tensor::{DType, Device, Tensor};
use anyhow::Result;
use std::collections::HashMap;
use std::ops::Range;

use crate::config::AudioEncoderConfig;
use crate::layers::{AudioEncoderLayer, Conv2d, LayerNorm, Linear};

/// Qwen3 ASR Audio Encoder (Whisper-style with chunk-based processing).
pub struct AudioEncoder {
    // Convolutional downsampling
    conv2d1: Conv2d,
    conv2d2: Conv2d,
    conv2d3: Conv2d,
    conv_out: Linear,

    // Positional embedding (sinusoidal, precomputed)
    positional_embedding: Tensor,

    // Transformer encoder layers
    layers: Vec<AudioEncoderLayer>,

    // Output projection
    ln_post: LayerNorm,
    proj1: Linear,
    proj2: Linear,

    config: AudioEncoderConfig,
}

impl AudioEncoder {
    pub fn load(
        weights: &HashMap<String, Tensor>,
        prefix: &str,
        config: &AudioEncoderConfig,
        device: Device,
    ) -> Result<Self> {
        let conv2d1 = Conv2d::load(weights, &format!("{}.conv2d1", prefix), [2, 2], [1, 1])?;
        let conv2d2 = Conv2d::load(weights, &format!("{}.conv2d2", prefix), [2, 2], [1, 1])?;
        let conv2d3 = Conv2d::load(weights, &format!("{}.conv2d3", prefix), [2, 2], [1, 1])?;
        let conv_out = Linear::load(weights, &format!("{}.conv_out", prefix))?;

        let mut layers = Vec::new();
        for i in 0..config.encoder_layers {
            let layer = AudioEncoderLayer::load(
                weights,
                &format!("{}.layers.{}", prefix, i),
                config.encoder_attention_heads,
                config.d_model as usize,
            )?;
            layers.push(layer);
        }

        let ln_post = LayerNorm::load(weights, &format!("{}.ln_post", prefix), 1e-5)?;
        let proj1 = Linear::load(weights, &format!("{}.proj1", prefix))?;
        let proj2 = Linear::load(weights, &format!("{}.proj2", prefix))?;

        // Create sinusoidal positional embedding
        let positional_embedding = create_sinusoidal_embedding(
            config.max_source_positions,
            config.d_model as usize,
            device,
        );

        Ok(Self {
            conv2d1,
            conv2d2,
            conv2d3,
            conv_out,
            positional_embedding,
            layers,
            ln_post,
            proj1,
            proj2,
            config: config.clone(),
        })
    }

    /// Encode mel spectrogram features into continuous audio embeddings.
    pub fn forward(&self, mel_features: &Tensor) -> Tensor {
        let num_frames = mel_features.size()[1] as usize;

        // Chunk size = n_window * 2
        let chunk_size = self.config.n_window * 2;

        // Split mel into chunks
        let num_full_chunks = num_frames / chunk_size;
        let tail_frames = num_frames % chunk_size;
        let num_chunks = num_full_chunks + if tail_frames > 0 { 1 } else { 0 };

        let device = mel_features.device();

        // Batch all chunks together
        let mut chunk_mels: Vec<Tensor> = Vec::with_capacity(num_chunks);
        let mut chunk_valid_tokens: Vec<usize> = Vec::with_capacity(num_chunks);

        for i in 0..num_full_chunks {
            let start = (i * chunk_size) as i64;
            let chunk_mel = mel_features
                .narrow(1, start, chunk_size as i64)
                .unsqueeze(0); // (1, mel_bins, chunk_size)
            chunk_mels.push(chunk_mel);
            chunk_valid_tokens.push(Self::feat_extract_output_length(chunk_size));
        }

        if tail_frames > 0 {
            let start = (num_full_chunks * chunk_size) as i64;
            let tail_mel = mel_features.narrow(1, start, tail_frames as i64);
            let pad_frames = chunk_size - tail_frames;
            let pad = Tensor::zeros(
                &[mel_features.size()[0], pad_frames as i64],
                DType::Float32,
                device,
            );
            let padded_mel = Tensor::cat(&[tail_mel, pad], 1).unsqueeze(0);
            chunk_mels.push(padded_mel);
            chunk_valid_tokens.push(Self::feat_extract_output_length(tail_frames));
        }

        // Batch all chunks: (num_chunks, 1, mel_bins, chunk_size)
        let batched = Tensor::cat(&chunk_mels, 0).unsqueeze(1);

        // Process all chunks through Conv2d stem as a batch
        let x = self.conv2d1.forward(&batched).gelu();
        let x = self.conv2d2.forward(&x).gelu();
        let x = self.conv2d3.forward(&x).gelu();

        // Reshape: (b, channels, freq, time) -> (b, time, channels*freq)
        let (b, c, f, t) = x.size4();
        let reshaped = x
            .permute(&[0, 3, 1, 2])
            .contiguous()
            .reshape(&[b, t, c * f]);
        let conv_out = self.conv_out.forward(&reshaped);

        // Add positional embedding
        let pos_emb = self.positional_embedding.narrow(0, 0, t).unsqueeze(0);
        let conv_out = conv_out + pos_emb;

        // Extract valid tokens per chunk, concatenate into flat sequence
        let mut all_valid: Vec<Tensor> = Vec::new();
        for (i, &valid) in chunk_valid_tokens.iter().enumerate() {
            let chunk_tokens = conv_out.get(i as i64).narrow(0, 0, valid as i64);
            all_valid.push(chunk_tokens);
        }

        // The reference block-diagonal mask makes windows independent. Run the
        // same windows as separate sequences instead, keeping the largest
        // attention score tensor bounded by one inference window rather than
        // allocating a total_tokens x total_tokens mask and score tensor.
        let flattened = Tensor::cat(&all_valid, 0);
        let chunk_size = self.config.n_window * 2;
        let chunks_per_window = self.config.n_window_infer / chunk_size;
        let window_ranges = token_window_ranges(&chunk_valid_tokens, chunks_per_window);
        let mut encoded_windows = Vec::with_capacity(window_ranges.len());

        for range in window_ranges {
            let mut window = flattened
                .narrow(0, range.start as i64, range.len() as i64)
                .unsqueeze(0);

            for layer in &self.layers {
                window = layer.forward(&window, None);
            }

            // MLX is lazy. Materializing at the natural window boundary keeps
            // graphs and intermediate attention activations window-bounded.
            window.eval();
            encoded_windows.push(window.squeeze_dim(0));
        }

        // Add the batch dimension back for the token-wise output projection.
        let hidden = Tensor::cat(&encoded_windows, 0).unsqueeze(0);

        // Output projection: LN -> Linear -> GELU -> Linear
        let hidden = self.ln_post.forward(&hidden);
        let hidden = self.proj1.forward(&hidden).gelu();
        let hidden = self.proj2.forward(&hidden);

        // Remove batch dim: (num_tokens, output_dim)
        hidden.squeeze_dim(0)
    }

    /// Compute output token count for a given number of input frames through 3x Conv2d.
    fn feat_extract_output_length(input_frames: usize) -> usize {
        let after_conv = |len: usize| -> usize { (len - 1) / 2 + 1 };
        after_conv(after_conv(after_conv(input_frames)))
    }

    /// Get the number of output audio tokens for a given number of mel frames.
    pub fn get_output_length(&self, input_frames: usize) -> usize {
        let chunk_size = self.config.n_window * 2;
        let num_full_chunks = input_frames / chunk_size;
        let tail_frames = input_frames % chunk_size;

        let mut total = num_full_chunks * Self::feat_extract_output_length(chunk_size);
        if tail_frames > 0 {
            total += Self::feat_extract_output_length(tail_frames);
        }
        total
    }
}

/// Convert chunk boundaries into contiguous token ranges for independent
/// encoder windows. A zero window size preserves the previous global-attention
/// fallback used for invalid/smaller-than-one-window configurations.
fn token_window_ranges(
    chunk_token_counts: &[usize],
    chunks_per_window: usize,
) -> Vec<Range<usize>> {
    if chunk_token_counts.is_empty() {
        return Vec::new();
    }

    let chunks_per_window = if chunks_per_window == 0 {
        chunk_token_counts.len()
    } else {
        chunks_per_window
    };
    let mut token_offset = 0;

    chunk_token_counts
        .chunks(chunks_per_window)
        .map(|chunk_counts| {
            let start = token_offset;
            token_offset += chunk_counts.iter().sum::<usize>();
            start..token_offset
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::token_window_ranges;

    #[test]
    fn token_windows_follow_chunk_groups_and_keep_tail() {
        assert_eq!(
            token_window_ranges(&[13, 13, 13, 7, 4], 2),
            vec![0..26, 26..46, 46..50]
        );
    }

    #[test]
    fn zero_chunks_per_window_preserves_global_attention_fallback() {
        assert_eq!(token_window_ranges(&[13, 7, 4], 0), vec![0..24]);
        assert!(token_window_ranges(&[], 8).is_empty());
    }
}

/// Create sinusoidal positional embeddings.
fn create_sinusoidal_embedding(max_len: usize, dim: usize, device: Device) -> Tensor {
    let half_dim = dim / 2;
    let log_timescale_increment = (10000.0f64).ln() / (half_dim - 1) as f64;

    let mut embeddings = vec![0.0f32; max_len * dim];

    for pos in 0..max_len {
        for i in 0..half_dim {
            let inv_timescale = (-(i as f64) * log_timescale_increment).exp();
            let angle = pos as f64 * inv_timescale;
            embeddings[pos * dim + i] = angle.sin() as f32;
            embeddings[pos * dim + half_dim + i] = angle.cos() as f32;
        }
    }

    Tensor::from_slice_f32(&embeddings)
        .reshape(&[max_len as i64, dim as i64])
        .to_device(device)
}
