export type ModelStatus = {
  id: string;
  title: string;
  repo: string;
  description: string;
  sizeHint: string;
  recommended: boolean;
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

export type TranscriptionResult = {
  audioPath: string;
  text: string;
  segments: TranscriptSegment[];
  srtPath: string | null;
  durationMs: number;
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
};

export type OptionsState = {
  language: string;
  writeSrt: boolean;
};

export type TaskStatus = "queued" | "running" | "completed" | "failed";

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

export type WorkspaceView = "tasks" | "settings";
