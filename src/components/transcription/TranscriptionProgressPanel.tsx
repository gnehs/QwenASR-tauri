import { ClockIcon } from "lucide-react";

import { Progress } from "@/components/ui/progress";
import { basename, formatDuration } from "@/lib/format";
import type { TranscriptionProgress } from "@/types/transcription";

const phaseLabels: Record<string, string> = {
  preparing: "準備中",
  loadingModel: "載入模型",
  loadingAudio: "讀取音訊",
  transcribing: "模型推論",
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
            {fileName} · {fileProgress}
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
