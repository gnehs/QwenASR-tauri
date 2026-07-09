use std::{env, path::PathBuf, time::Instant};

use qwen3_asr::{inference::AsrInference, tensor::Device};
use tokenizers::{
    decoders::byte_level::ByteLevel as ByteLevelDecoder, models::bpe::BPE,
    pre_tokenizers::byte_level::ByteLevel, Tokenizer,
};

#[path = "../src/error.rs"]
#[allow(dead_code)]
mod error;
#[path = "../src/forced_alignment.rs"]
mod forced_alignment;

use forced_alignment::ForcedAlignerInference;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [asr_model_dir, aligner_model_dir, audio_path, language] = args.as_slice() else {
        return Err(
            "Usage: cargo run --release --example forced_alignment_smoke -- \
             <asr_model_dir> <aligner_model_dir> <audio_path> <language>"
                .into(),
        );
    };
    let asr_model_dir = PathBuf::from(asr_model_dir);
    let aligner_model_dir = PathBuf::from(aligner_model_dir);
    ensure_tokenizer_json(&asr_model_dir)?;
    ensure_tokenizer_json(&aligner_model_dir)?;

    let device = default_device();
    let started = Instant::now();
    let asr = AsrInference::load(&asr_model_dir, device)?;
    let asr_load_elapsed = started.elapsed();

    let started = Instant::now();
    let transcription = asr.transcribe(audio_path, Some(language))?;
    let asr_elapsed = started.elapsed();
    if transcription.text.trim().is_empty() {
        return Err("ASR returned an empty transcript; forced alignment cannot run.".into());
    }

    let started = Instant::now();
    let aligner = ForcedAlignerInference::load(&aligner_model_dir, device)?;
    let aligner_load_elapsed = started.elapsed();
    let started = Instant::now();
    let aligned = aligner.align(audio_path, &transcription.text, language)?;
    let align_elapsed = started.elapsed();

    println!("backend: MLX (Metal GPU)");
    println!("language: {language}");
    println!("audio_seconds: {:.2}", transcription.duration_seconds);
    println!("asr_load_seconds: {:.2}", asr_load_elapsed.as_secs_f64());
    println!("asr_seconds: {:.2}", asr_elapsed.as_secs_f64());
    println!(
        "aligner_load_seconds: {:.2}",
        aligner_load_elapsed.as_secs_f64()
    );
    println!("alignment_seconds: {:.2}", align_elapsed.as_secs_f64());
    println!("aligned_units: {}", aligned.len());
    println!("text:\n{}", transcription.text.trim());
    println!("alignment:");
    for item in aligned {
        println!("{}\t{}\t{}", item.start_ms, item.end_ms, item.text);
    }

    Ok(())
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

fn ensure_tokenizer_json(model_dir: &std::path::Path) -> Result<()> {
    let tokenizer_path = model_dir.join("tokenizer.json");
    if tokenizer_path.exists() {
        return Ok(());
    }

    let vocab_path = model_dir.join("vocab.json");
    let merges_path = model_dir.join("merges.txt");
    if !vocab_path.exists() || !merges_path.exists() {
        return Err(format!(
            "Cannot generate tokenizer.json before vocab.json and merges.txt exist in {}",
            model_dir.display()
        )
        .into());
    }

    let vocab = vocab_path.to_string_lossy();
    let merges = merges_path.to_string_lossy();
    let bpe = BPE::from_file(vocab.as_ref(), merges.as_ref()).build()?;
    let mut tokenizer = Tokenizer::new(bpe);
    tokenizer.with_pre_tokenizer(Some(ByteLevel::new(false, true, true)));
    tokenizer.with_decoder(Some(ByteLevelDecoder::default()));
    tokenizer.save(tokenizer_path, false)?;
    Ok(())
}
