import { ClockIcon } from "lucide-react";

import { Progress } from "@/components/ui/progress";
import { basename, formatDuration } from "@/lib/format";
import type { TranscriptionProgress } from "@/types/transcription";

const phaseLabels: Record<string, string> = {
  preparing: "準備中",
  loadingModel: "載入模型",
  loadingAudio: "讀取音訊",
  analyzingAudio: "分析音訊",
  transcribing: "模型推論",
  transcribingSegments: "逐段轉錄",
  writingSrt: "輸出字幕",
  finalizing: "整理結果",
  complete: "完成",
  error: "失敗",
};

export function TranscriptionProgressPanel({
  progress,
}: {
  progress: TranscriptionProgress | null;
}) {
  if (!progress) return null;

  const percent = Math.max(0, Math.min(100, progress.percent));
  const filePath = progress.currentFile ?? progress.audioPath;
  const fileName = filePath ? basename(filePath) : "準備中";
  const phaseLabel = phaseLabels[progress.phase] ?? progress.phase;
  const chunkProgress =
    progress.totalChunks && progress.chunkIndex !== null
      ? `片段 ${Math.min(progress.chunkIndex, progress.totalChunks)} / ${
          progress.totalChunks
        }`
      : null;
  const processedSpeech =
    progress.processedAudioMs !== null && progress.totalSpeechMs !== null
      ? `有聲 ${formatDuration(progress.processedAudioMs)} / ${formatDuration(
          progress.totalSpeechMs,
        )}`
      : null;
  const skippedSilence =
    progress.skippedSilenceMs !== null && progress.skippedSilenceMs > 0
      ? `已跳過靜音 ${formatDuration(progress.skippedSilenceMs)}`
      : null;
  const chunkRange =
    progress.chunkStartMs !== null && progress.chunkEndMs !== null
      ? `${formatDuration(progress.chunkStartMs)}-${formatDuration(
          progress.chunkEndMs,
        )}`
      : null;
  const fileProgress =
    progress.totalFiles > 1 && progress.fileIndex > 0
      ? `第 ${progress.fileIndex} / ${progress.totalFiles} 個檔案`
      : phaseLabel;

  return (
    <div className="transcription-progress">
      <div className="transcription-progress-head">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">
            {progress.message || "轉錄中"}
          </div>
          <div className="truncate text-xs text-muted-foreground">
            {[fileName, fileProgress, chunkProgress].filter(Boolean).join(" · ")}
          </div>
        </div>
        <div className="transcription-progress-stat">
          <span className="text-sm font-medium tabular-nums">
            {percent.toFixed(0)}%
          </span>
          <span className="text-xs text-muted-foreground">
            ETA {progress.etaMs == null ? "估算中" : formatDuration(progress.etaMs)}
          </span>
        </div>
      </div>
      <Progress value={percent} aria-label="轉錄進度" />
      {processedSpeech || skippedSilence || chunkRange ? (
        <div className="transcription-progress-metrics">
          {processedSpeech ? <span>{processedSpeech}</span> : null}
          {skippedSilence ? <span>{skippedSilence}</span> : null}
          {chunkRange ? <span>目前 {chunkRange}</span> : null}
        </div>
      ) : null}
      <div className="transcription-progress-foot">
        <span className="inline-flex min-w-0 items-center gap-1 truncate">
          <ClockIcon className="size-3.5 shrink-0" />
          已用 {formatDuration(progress.elapsedMs)}
        </span>
        <span className="tabular-nums">{percent.toFixed(1)}%</span>
      </div>
    </div>
  );
}
