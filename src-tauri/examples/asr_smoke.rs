use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use qwen3_asr::{inference::AsrInference, tensor::Device};
use tokenizers::{
    decoders::byte_level::ByteLevel as ByteLevelDecoder, models::bpe::BPE,
    pre_tokenizers::byte_level::ByteLevel, Tokenizer,
};
use uuid::Uuid;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
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

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let (model_dir, audio_path, language) = match args.as_slice() {
        [audio_path] => (default_model_dir()?, audio_path.as_str(), None),
        [model_dir, audio_path] => (PathBuf::from(model_dir), audio_path.as_str(), None),
        [model_dir, audio_path, language] => (
            PathBuf::from(model_dir),
            audio_path.as_str(),
            Some(language.as_str()),
        ),
        _ => {
            return Err(
                "Usage: cargo run --example asr_smoke -- [model_dir] <audio_path> [language]"
                    .into(),
            )
        }
    };

    ensure_tokenizer_json(&model_dir)?;
    let prepared_audio = prepare_audio(audio_path)?;

    let started = Instant::now();
    let engine = AsrInference::load(&model_dir, default_device())?;
    let load_elapsed = started.elapsed();

    let started = Instant::now();
    let result = engine.transcribe(prepared_audio.path_str()?, language)?;
    let transcribe_elapsed = started.elapsed();
    let realtime_factor = transcribe_elapsed.as_secs_f64() / result.duration_seconds.max(0.001);
    let text = normalize_asr_text(&result, language.is_some());

    println!("language: {}", result.language);
    println!("audio_seconds: {:.2}", result.duration_seconds);
    println!("load_seconds: {:.2}", load_elapsed.as_secs_f64());
    println!(
        "transcribe_seconds: {:.2}",
        transcribe_elapsed.as_secs_f64()
    );
    println!("rtf: {:.2}x", realtime_factor);
    println!("text:\n{}", text);

    Ok(())
}

struct TempAudio {
    path: PathBuf,
}

impl TempAudio {
    fn path_str(&self) -> Result<&str> {
        self.path
            .to_str()
            .ok_or_else(|| "Prepared audio path contains unsupported characters.".into())
    }
}

impl Drop for TempAudio {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn prepare_audio(audio_path: &str) -> Result<TempAudio> {
    let input = Path::new(audio_path);
    if !input.is_file() {
        return Err(format!("Audio file does not exist: {}", input.display()).into());
    }

    let output = std::env::temp_dir().join(format!("qwenasr-smoke-{}.wav", Uuid::new_v4()));
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            audio_path,
            "-map",
            "0:a:0",
            "-vn",
            "-af",
            "aresample=16000,apad=whole_len=320",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-acodec",
            "pcm_s16le",
            "-f",
            "wav",
        ])
        .arg(&output)
        .status()?;

    if !status.success() {
        let _ = fs::remove_file(&output);
        return Err("FFmpeg failed to normalize the smoke-test audio.".into());
    }

    Ok(TempAudio { path: output })
}

fn default_model_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir().ok_or("Could not resolve the application support directory")?;
    Ok(data_dir
        .join("QwenASR Studio")
        .join("models")
        .join("qwen3-asr-0.6b"))
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

fn ensure_tokenizer_json(model_dir: &Path) -> Result<()> {
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

fn normalize_asr_text(
    result: &qwen3_asr::inference::TranscribeResult,
    language_forced: bool,
) -> String {
    let text = if language_forced {
        result.text.trim()
    } else {
        parse_auto_asr_text(&result.raw_output).unwrap_or_else(|| result.text.trim())
    };

    collapse_spaced_acronyms(text)
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
