import { Progress } from "@/components/ui/progress";
import { Trans, useLingui } from "@lingui/react/macro";
import { formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { TranscriptionProgress } from "@/types/transcription";

function phaseLabel(phase: string, t: ReturnType<typeof useLingui>["t"]) {
  switch (phase) {
    case "preparing":
      return t`準備處理音訊`;
    case "loadingModel":
      return t`載入模型`;
    case "loadingAudio":
      return t`讀取音訊`;
    case "analyzingAudio":
      return t`分析語音片段`;
    case "transcribing":
      return t`執行語音辨識`;
    case "transcribingSegments":
      return t`逐段轉錄`;
    case "aligningTimestamps":
      return t`校準時間軸`;
    case "writingSrt":
      return t`寫入字幕檔`;
    case "finalizing":
      return t`整理轉錄結果`;
    case "complete":
      return t`轉錄完成`;
    case "error":
      return t`轉錄失敗`;
    default:
      return null;
  }
}

function activeStageIndex(progress: TranscriptionProgress) {
  switch (progress.phase) {
    case "analyzingAudio":
      return 1;
    case "transcribing":
    case "transcribingSegments":
      return 2;
    case "aligningTimestamps":
    case "writingSrt":
    case "finalizing":
    case "complete":
      return 3;
    case "loadingModel":
      return progress.processedAudioMs != null ? 3 : 0;
    default:
      return 0;
  }
}

export function TranscriptionProgressPanel({
  now,
  progress,
  progressUpdatedAt,
}: {
  now?: number;
  progress: TranscriptionProgress | null;
  progressUpdatedAt?: number | null;
}) {
  const { t } = useLingui();
  if (!progress) return null;

  const percent = Math.max(0, Math.min(100, progress.percent));
  const stages = [t`準備`, t`分析`, t`轉錄`, t`整理`];
  const elapsedSinceProgress =
    now != null && progressUpdatedAt != null
      ? Math.max(0, now - progressUpdatedAt)
      : 0;
  const elapsedMs = progress.elapsedMs + elapsedSinceProgress;
  const etaMs =
    progress.etaMs == null
      ? null
      : Math.max(0, progress.etaMs - elapsedSinceProgress);
  const etaLabel = etaMs == null ? t`估算中` : formatDuration(etaMs);
  const activeStage = activeStageIndex(progress);
  const currentMessage =
    progress.message?.trim() || phaseLabel(progress.phase, t) || t`轉錄中`;
  const processedSpeech =
    progress.processedAudioMs != null && progress.totalSpeechMs != null
      ? `${formatDuration(progress.processedAudioMs)} / ${formatDuration(
          progress.totalSpeechMs,
        )}`
      : null;
  return (
    <section
      className="flex w-full min-w-0 flex-col gap-3 text-left"
      aria-label={t`轉錄進度摘要`}
    >
      <div className="flex min-w-0 items-center justify-between gap-3 max-[720px]:flex-col max-[720px]:items-start">
        <div className="min-w-0">
          <div className="text-xs text-muted-foreground">
            <Trans>目前進度</Trans>
          </div>
          <div className="truncate text-sm font-medium" title={currentMessage}>
            {currentMessage}
          </div>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-0 max-[720px]:items-start">
          <strong className="text-2xl font-medium tabular-nums">
            {percent.toFixed(0)}%
          </strong>
          <span className="text-xs text-muted-foreground">
            <Trans>剩餘 {etaLabel}</Trans>
          </span>
        </div>
      </div>

      <Progress
        aria-label={t`轉錄進度 ${percent.toFixed(0)}%`}
        className="gap-3 [&_[data-slot=progress-indicator]]:bg-primary [&_[data-slot=progress-track]]:h-2"
        value={percent}
      />

      <div
        className="grid grid-cols-4 gap-2 py-2.5"
        aria-label={t`處理階段`}
        role="list"
      >
        {stages.map((stage, index) => (
          <div
            aria-current={index === activeStage ? "step" : undefined}
            className={cn(
              "flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground",
              index <= activeStage && "text-foreground",
              index === activeStage && "font-medium",
            )}
            key={stage}
            role="listitem"
          >
            <span
              className={cn(
                "block size-2 shrink-0 rounded-full border border-border bg-background",
                index < activeStage && "border-foreground bg-foreground",
                index === activeStage &&
                  "border-primary bg-primary ring-4 ring-primary/20",
              )}
              aria-hidden="true"
            />
            <span>{stage}</span>
          </div>
        ))}
      </div>

      <div className="flex min-w-0 items-center justify-between gap-3 text-xs text-muted-foreground tabular-nums">
        <span>
          <Trans>已用 {formatDuration(elapsedMs)}</Trans>
        </span>
        <span className="truncate" title={processedSpeech ?? undefined}>
          {processedSpeech ? (
            <Trans>已處理 {processedSpeech}</Trans>
          ) : (
            <Trans>正在估算語音量</Trans>
          )}
        </span>
      </div>
    </section>
  );
}
