import { ClockIcon } from "lucide-react";

import { Progress } from "@/components/ui/progress";
import { formatDuration } from "@/lib/format";
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

const etaPendingLabels: Record<string, string> = {
  preparing: "ETA 待推論",
  loadingModel: "ETA 待推論",
  loadingAudio: "ETA 待推論",
  analyzingAudio: "ETA 待推論",
};

export function TranscriptionProgressPanel({
  progress,
}: {
  progress: TranscriptionProgress | null;
}) {
  if (!progress) return null;

  const percent = Math.max(0, Math.min(100, progress.percent));
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
  const detailLine = [fileProgress, chunkProgress].filter(Boolean).join(" · ");
  const etaLabel =
    progress.etaMs == null
      ? (etaPendingLabels[progress.phase] ?? "ETA 估算中")
      : `ETA ${formatDuration(progress.etaMs)}`;

  return (
    <div className="transcription-progress">
      <div className="transcription-progress-head">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">
            {progress.message || "轉錄中"}
          </div>
          <div className="truncate text-xs text-muted-foreground">
            {detailLine}
          </div>
        </div>
        <div className="transcription-progress-stat">
          <span className="text-sm font-medium tabular-nums">
            {percent.toFixed(0)}%
          </span>
          <span className="text-xs text-muted-foreground">{etaLabel}</span>
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
      </div>
    </div>
  );
}
