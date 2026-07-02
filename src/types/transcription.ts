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
};

export type OptionsState = {
  language: string;
  prompt: string;
  segmentSeconds: number;
  searchSeconds: number;
  threads: number;
  skipSilence: boolean;
  pastText: boolean;
  convertWithFfmpeg: boolean;
  writeSrt: boolean;
};

export type SelectOption = {
  label: string;
  value: string;
};

export type WorkspaceView = "transcribe" | "batch" | "settings";
