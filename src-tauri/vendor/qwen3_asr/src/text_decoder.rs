use anyhow::Result;
use std::collections::HashMap;

use crate::tensor::{DType, Device, Tensor};

use crate::config::TextDecoderConfig;
use crate::layers::{LayerKvCache, RmsNorm, TextDecoderLayer};
use crate::weights::get_weight;

/// Bound prompt activations and attention masks while staying aligned with the
/// cache allocator's 256-token growth step.
pub const PREFILL_BLOCK_SIZE: usize = 512;

/// KV cache for autoregressive generation.
pub struct KvCache {
    layers: Vec<LayerKvCache>,
}

impl KvCache {
    pub fn new(num_layers: usize) -> Self {
        let layers = std::iter::repeat_with(LayerKvCache::new)
            .take(num_layers)
            .collect();
        Self { layers }
    }

    pub fn seq_len(&self) -> i64 {
        self.layers.first().map_or(0, LayerKvCache::seq_len)
    }
}

/// Qwen3 Text Decoder model.
pub struct TextDecoder {
    embed_tokens: Tensor,
    layers: Vec<TextDecoderLayer>,
    norm: RmsNorm,
    lm_head_weight_t: Tensor, // Pre-transposed for matmul
    config: TextDecoderConfig,
}

impl TextDecoder {
    pub fn load(
        weights: &HashMap<String, Tensor>,
        prefix: &str,
        config: &TextDecoderConfig,
    ) -> Result<Self> {
        let embed_tokens = get_weight(weights, &format!("{}.embed_tokens", prefix), "weight")?;

        let mut layers = Vec::new();
        for i in 0..config.num_hidden_layers {
            let layer = TextDecoderLayer::load(
                weights,
                &format!("{}.layers.{}", prefix, i),
                config.num_attention_heads,
                config.num_key_value_heads,
                config.head_dim,
                config.rms_norm_eps,
            )?;
            layers.push(layer);
        }

        let norm = RmsNorm::load(weights, &format!("{}.norm", prefix), config.rms_norm_eps)?;

        let lm_head_key = format!("{}", prefix.replace(".model", ".lm_head"));
        let lm_head_weight = if config.tie_word_embeddings {
            embed_tokens.shallow_clone()
        } else {
            get_weight(weights, &lm_head_key, "weight")?
        };

        Ok(Self {
            embed_tokens,
            layers,
            norm,
            lm_head_weight_t: lm_head_weight.tr(), // Pre-transpose at load time
            config: config.clone(),
        })
    }

    pub fn embed(&self, input_ids: &Tensor) -> Tensor {
        Tensor::embedding(&self.embed_tokens, input_ids)
    }

    pub fn forward(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        kv_cache: &mut KvCache,
        mask: Option<&Tensor>,
    ) -> Tensor {
        self.forward_hidden(hidden_states, cos, sin, kv_cache, mask)
            .matmul(&self.lm_head_weight_t)
    }

    /// Run a prefill while projecting only its final position to vocabulary logits.
    pub fn forward_last_token(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        kv_cache: &mut KvCache,
        mask: Option<&Tensor>,
    ) -> Tensor {
        let hidden = self.forward_hidden(hidden_states, cos, sin, kv_cache, mask);
        let last_position = hidden.size()[1] - 1;
        hidden
            .narrow(1, last_position, 1)
            .matmul(&self.lm_head_weight_t)
    }

    /// Prefill a long prompt in fixed-size blocks while reusing the KV cache.
    ///
    /// Each block attends to all cached prefix tokens and causally to its own
    /// tokens. This is mathematically equivalent to one full causal prefill,
    /// but bounds query activations to `block_size` and avoids a full square
    /// causal mask.
    pub fn prefill_last_token(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        kv_cache: &mut KvCache,
        block_size: usize,
    ) -> Tensor {
        assert!(block_size > 0, "prefill block size must be positive");
        let seq_len = hidden_states.size()[1];
        assert!(seq_len > 0, "prefill requires at least one token");
        assert_eq!(cos.size()[0], seq_len, "cos positions must match prompt");
        assert_eq!(sin.size()[0], seq_len, "sin positions must match prompt");

        let mut block_start = 0;
        loop {
            let block_len = std::cmp::min(block_size as i64, seq_len - block_start);
            let block_end = block_start + block_len;
            let block_hidden = hidden_states.narrow(1, block_start, block_len);
            let block_cos = cos.narrow(0, block_start, block_len);
            let block_sin = sin.narrow(0, block_start, block_len);
            let past_len = kv_cache.seq_len();
            let mask = create_causal_mask(block_len, past_len, hidden_states.device());
            let hidden =
                self.forward_layers(&block_hidden, &block_cos, &block_sin, kv_cache, Some(&mask));

            if block_end == seq_len {
                let hidden = self.norm.forward(&hidden);
                return hidden
                    .narrow(1, block_len - 1, 1)
                    .matmul(&self.lm_head_weight_t);
            }

            // MLX records operations lazily. Evaluate once per prefill block so
            // cache updates do not retain the entire prompt activation graph.
            hidden.eval();
            block_start = block_end;
        }
    }

    fn forward_hidden(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        kv_cache: &mut KvCache,
        mask: Option<&Tensor>,
    ) -> Tensor {
        let hidden = self.forward_layers(hidden_states, cos, sin, kv_cache, mask);
        self.norm.forward(&hidden)
    }

    fn forward_layers(
        &self,
        hidden_states: &Tensor,
        cos: &Tensor,
        sin: &Tensor,
        kv_cache: &mut KvCache,
        mask: Option<&Tensor>,
    ) -> Tensor {
        let mut hidden = hidden_states.shallow_clone();

        for (layer, layer_cache) in self.layers.iter().zip(kv_cache.layers.iter_mut()) {
            hidden = layer.forward(&hidden, cos, sin, layer_cache, mask);
        }

        hidden
    }

    pub fn config(&self) -> &TextDecoderConfig {
        &self.config
    }
}

/// Create a causal attention mask.
pub fn create_causal_mask(seq_len: i64, past_len: i64, device: Device) -> Tensor {
    let total_len = past_len + seq_len;
    let mask = Tensor::full(
        &[seq_len, total_len],
        f64::NEG_INFINITY,
        DType::Float32,
        device,
    );
    let mask = mask.triu(past_len + 1);
    mask.unsqueeze(0).unsqueeze(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "tch-backend")]
    #[test]
    fn chunked_causal_mask_exposes_prefix_and_prior_block_positions() {
        let mask = create_causal_mask(3, 2, Device::Cpu);
        assert_eq!(mask.size(), vec![1, 1, 3, 5]);

        let values = mask.to_vec_f32();
        assert_eq!(&values[0..3], &[0.0, 0.0, 0.0]);
        assert!(values[3].is_infinite() && values[3].is_sign_negative());
        assert!(values[4].is_infinite() && values[4].is_sign_negative());
        assert_eq!(&values[5..9], &[0.0, 0.0, 0.0, 0.0]);
        assert!(values[9].is_infinite() && values[9].is_sign_negative());
        assert_eq!(&values[10..15], &[0.0, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn prefill_block_size_is_cache_allocator_aligned() {
        assert_eq!(PREFILL_BLOCK_SIZE % 256, 0);
    }
}
