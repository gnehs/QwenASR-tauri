#![allow(dead_code)]

use std::{env, time::Instant};

#[path = "../src/audio.rs"]
mod audio;
#[path = "../src/error.rs"]
mod error;
#[path = "../src/models.rs"]
mod models;
#[path = "../src/vad.rs"]
mod vad;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    let audio_path = env::args()
        .nth(1)
        .ok_or("Usage: cargo run --release --example vad_smoke -- <audio_path>")?;
    let started = Instant::now();
    let prepared = audio::prepare_audio_for_asr(&audio_path)?;
    let samples = audio::read_normalized_i16(prepared.inference_path())?;
    let audio_seconds = samples.len() as f64 / audio::ASR_SAMPLE_RATE as f64;

    let analysis = vad::analyze_with_progress(&samples, |progress| {
        println!(
            "{:>6.2}s  {:>5.1}%  {}",
            started.elapsed().as_secs_f64(),
            progress.ratio * 100.0,
            progress.message
        );
    })?;

    println!("audio_seconds: {audio_seconds:.2}");
    println!("chunks: {}", analysis.chunks.len());
    println!(
        "chunk_audio_seconds: {:.2}",
        analysis.chunk_audio_ms as f64 / 1000.0
    );
    println!(
        "skipped_silence_seconds: {:.2}",
        analysis.skipped_silence_ms as f64 / 1000.0
    );
    println!("vad_seconds: {:.2}", started.elapsed().as_secs_f64());

    for (index, chunk) in analysis.chunks.iter().enumerate() {
        println!(
            "chunk {:>2}: {:>8.2}s - {:>8.2}s ({:>6.2}s)",
            index + 1,
            chunk.start_ms() as f64 / 1000.0,
            chunk.end_ms() as f64 / 1000.0,
            chunk.duration_ms() as f64 / 1000.0
        );
    }

    Ok(())
}
