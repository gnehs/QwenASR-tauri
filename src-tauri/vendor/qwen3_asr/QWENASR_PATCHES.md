# QwenASR Studio patches

This directory vendors `second-state/qwen3_asr_rs` commit
`3fa673441682350b12da5c21429fea71ce212023`.

## Decode position budget

Upstream v0.2.0 permits 4,096 generated tokens but precomputes only 512 decode
positions. If a transcription does not emit EOS within 512 tokens, MLX receives
an empty RoPE slice and terminates while reshaping the decoder output.

QwenASR Studio uses Qwen's Transformers default of 512 generated tokens and
derives the position-table capacity from the same constant. This is sufficient
for the app's bounded 30-second audio chunks and prevents runaway repetition.
