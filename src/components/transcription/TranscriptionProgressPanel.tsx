import { Progress } from "@/components/ui/progress";
import { Trans, useLingui } from "@lingui/react/macro";
import { formatDuration } from "@/lib/format";
import type { TranscriptionProgress } from "@/types/transcription";

function phaseLabel(phase: string, t: ReturnType<typeof useLingui>["t"]) {
  switch (phase) {
    case "preparing": return t`準備處理音訊`;
    case "loadingModel": return t`載入模型`;
    case "loadingAudio": return t`讀取音訊`;
    case "analyzingAudio": return t`分析語音片段`;
    case "transcribing": return t`執行語音辨識`;
    case "transcribingSegments": return t`逐段轉錄`;
    case "aligningTimestamps": return t`校準時間軸`;
    case "writingSrt": return t`寫入字幕檔`;
    case "finalizing": return t`整理轉錄結果`;
    case "complete": return t`轉錄完成`;
    case "error": return t`轉錄失敗`;
    default: return null;
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
    <section className="transcription-progress" aria-label={t`轉錄進度摘要`}>
      <div className="transcription-progress-head">
        <div className="min-w-0">
          <div className="text-xs text-muted-foreground">
            <Trans>目前進度</Trans>
          </div>
          <div className="truncate text-sm font-medium" title={currentMessage}>
            {currentMessage}
          </div>
        </div>
        <div className="transcription-progress-stat">
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
        className="transcription-progress-bar"
        value={percent}
      />

      <div
        className="transcription-stage-list"
        aria-label={t`處理階段`}
        role="list"
      >
        {stages.map((stage, index) => (
          <div
            aria-current={index === activeStage ? "step" : undefined}
            className={
              index < activeStage
                ? "is-complete"
                : index === activeStage
                  ? "is-active"
                  : undefined
            }
            key={stage}
            role="listitem"
          >
            <span className="transcription-stage-dot" aria-hidden="true" />
            <span>{stage}</span>
          </div>
        ))}
      </div>

      <div className="transcription-progress-foot">
        <span><Trans>已用 {formatDuration(elapsedMs)}</Trans></span>
        <span className="truncate" title={processedSpeech ?? undefined}>
          {processedSpeech ? <Trans>已處理 {processedSpeech}</Trans> : <Trans>正在估算語音量</Trans>}
        </span>
      </div>
    </section>
  );
}
