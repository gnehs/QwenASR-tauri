use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
pub struct KnownModel {
    pub id: &'static str,
    pub title: &'static str,
    pub repo: &'static str,
    pub files: &'static [&'static str],
    pub description: &'static str,
    pub size_hint: &'static str,
    pub recommended: bool,
}

pub const KNOWN_MODELS: &[KnownModel] = &[
    KnownModel {
        id: "qwen3-asr-0.6b",
        title: "Qwen3-ASR 0.6B",
        repo: "Qwen/Qwen3-ASR-0.6B",
        files: &["model.safetensors", "vocab.json", "merges.txt"],
        description: "快速、適合大多數單次與批次轉錄工作。",
        size_hint: "~490 MB",
        recommended: true,
    },
    KnownModel {
        id: "qwen3-asr-1.7b",
        title: "Qwen3-ASR 1.7B",
        repo: "Qwen/Qwen3-ASR-1.7B",
        files: &[
            "model.safetensors.index.json",
            "model-00001-of-00002.safetensors",
            "model-00002-of-00002.safetensors",
            "vocab.json",
            "merges.txt",
        ],
        description: "較高準確度，適合重要錄音或較複雜的聲學環境。",
        size_hint: "~3.4 GB",
        recommended: false,
    },
];

pub fn find_known_model(id: &str) -> Option<KnownModel> {
    let normalized = id.to_lowercase();
    KNOWN_MODELS
        .iter()
        .copied()
        .find(|model| model.id == normalized)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub id: String,
    pub title: String,
    pub repo: String,
    pub description: String,
    pub size_hint: String,
    pub recommended: bool,
    pub installed: bool,
    pub path: String,
    pub files: Vec<String>,
    pub missing_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub state: String,
    pub current_file: Option<String>,
    pub file_index: usize,
    pub total_files: usize,
    pub file_bytes_completed: u64,
    pub file_total_bytes: u64,
    pub speed_bytes_per_sec: f64,
    pub percent: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeOptions {
    pub model_id: String,
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub segment_seconds: f32,
    pub search_seconds: f32,
    pub skip_silence: bool,
    pub past_text: bool,
    pub threads: Option<usize>,
    pub convert_with_ffmpeg: bool,
    pub write_srt: bool,
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeFileRequest {
    pub audio_path: String,
    pub options: TranscribeOptions,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeBatchRequest {
    pub audio_paths: Vec<String>,
    pub options: TranscribeOptions,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub audio_path: String,
    pub text: String,
    pub segments: Vec<TranscriptSegment>,
    pub srt_path: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionProgress {
    pub state: String,
    pub phase: String,
    pub message: String,
    pub audio_path: Option<String>,
    pub current_file: Option<String>,
    pub file_index: usize,
    pub total_files: usize,
    pub percent: f64,
    pub elapsed_ms: u128,
    pub eta_ms: Option<u128>,
}
