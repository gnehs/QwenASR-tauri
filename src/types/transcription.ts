export type ModelStatus = {
  id: string;
  title: string;
  repo: string;
  description: string;
  sizeHint: string;
  recommended: boolean;
  role: "transcription" | "forcedAlignment";
  installed: boolean;
  path: string;
  files: string[];
  missingFiles: string[];
};

export type DownloadProgress = {
  modelId: string;
  state: string;
  currentFile: string | null;
  fileIndex: number;
  totalFiles: number;
  fileBytesCompleted: number;
  fileTotalBytes: number;
  speedBytesPerSec: number;
  percent: number;
  message: string;
};

export type FfmpegStatus = {
  available: boolean;
  version: string | null;
};

export type TranscriptSegment = {
  startMs: number;
  endMs: number;
  text: string;
};

export type TranscriptWord = {
  startMs: number;
  endMs: number;
  text: string;
};

export type LanguageSource = "specified" | "detected" | "unknown";

export type AlignmentStatus = "forced" | "partial" | "approximate";

export type TranscriptionTimings = {
  audioPrepareMs: number;
  vadMs: number;
  asrMs: number;
  asrMelMs: number;
  asrEncoderMs: number;
  asrPromptMs: number;
  asrPrefillMs: number;
  asrDecodeMs: number;
  asrPostprocessMs: number;
  alignmentMs: number;
  finalizeMs: number;
  asrChunkCount: number;
  asrGeneratedTokens: number;
  totalMs: number;
};

export type TranscriptionResult = {
  audioPath: string;
  text: string;
  segments: TranscriptSegment[];
  words: TranscriptWord[];
  detectedLanguage: string | null;
  languageSource: LanguageSource;
  alignmentStatus: AlignmentStatus;
  alignmentCoverage: number;
  txtPath: string | null;
  srtPath: string | null;
  jsonPath: string | null;
  durationMs: number;
  timings: TranscriptionTimings;
};

export type TranscriptionProgress = {
  state: string;
  phase: string;
  message: string;
  audioPath: string | null;
  currentFile: string | null;
  fileIndex: number;
  totalFiles: number;
  percent: number;
  elapsedMs: number;
  etaMs: number | null;
  chunkIndex: number | null;
  totalChunks: number | null;
  chunkStartMs: number | null;
  chunkEndMs: number | null;
  processedAudioMs: number | null;
  totalSpeechMs: number | null;
  skippedSilenceMs: number | null;
  partialSegments: TranscriptSegment[] | null;
  timings: TranscriptionTimings | null;
};

export type OptionsState = {
  language: string;
  context: string;
  writeTxt: boolean;
  writeSrt: boolean;
  writeJson: boolean;
  segmentByPunctuation: boolean;
};

export type TaskStatus =
  "queued" | "running" | "completed" | "failed" | "cancelled";

export type TranscriptionTask = {
  id: string;
  audioPath: string;
  modelId: string;
  modelTitle: string;
  outputDir: string;
  options: OptionsState;
  status: TaskStatus;
  progress: TranscriptionProgress | null;
  progressUpdatedAt: number | null;
  result: TranscriptionResult | null;
  error: string | null;
  createdAt: number;
  startedAt: number | null;
  completedAt: number | null;
};

export type TaskDraft = {
  files: string[];
  modelId: string;
  outputDir: string;
  options: OptionsState;
};

export type SelectOption = {
  label: string;
  value: string;
};
