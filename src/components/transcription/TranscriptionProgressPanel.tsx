import { Progress } from "@/components/ui/progress";
import { formatDuration } from "@/lib/format";
import type { TranscriptionProgress } from "@/types/transcription";

const stages = ["準備", "分析", "轉錄", "整理"] as const;

const phaseLabels: Record<string, string> = {
  preparing: "準備處理音訊",
  loadingModel: "載入模型",
  loadingAudio: "讀取音訊",
  analyzingAudio: "分析語音片段",
  transcribing: "執行語音辨識",
  transcribingSegments: "逐段轉錄",
  aligningTimestamps: "校準時間軸",
  writingSrt: "寫入字幕檔",
  finalizing: "整理轉錄結果",
  complete: "轉錄完成",
  error: "轉錄失敗",
};

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
  if (!progress) return null;

  const percent = Math.max(0, Math.min(100, progress.percent));
  const elapsedSinceProgress =
    now != null && progressUpdatedAt != null
      ? Math.max(0, now - progressUpdatedAt)
      : 0;
  const elapsedMs = progress.elapsedMs + elapsedSinceProgress;
  const etaMs =
    progress.etaMs == null
      ? null
      : Math.max(0, progress.etaMs - elapsedSinceProgress);
  const etaLabel =
    etaMs == null ? "估算中" : formatDuration(etaMs);
  const activeStage = activeStageIndex(progress);
  const currentMessage =
    progress.message?.trim() || phaseLabels[progress.phase] || "轉錄中";
  const processedSpeech =
    progress.processedAudioMs != null && progress.totalSpeechMs != null
      ? `${formatDuration(progress.processedAudioMs)} / ${formatDuration(
          progress.totalSpeechMs,
        )}`
      : null;

  return (
    <section className="transcription-progress" aria-label="轉錄進度摘要">
      <div className="transcription-progress-head">
        <div className="min-w-0">
          <div className="text-xs text-muted-foreground">
            目前進度
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
            剩餘 {etaLabel}
          </span>
        </div>
      </div>

      <Progress
        aria-label={`轉錄進度 ${percent.toFixed(0)}%`}
        className="transcription-progress-bar"
        value={percent}
      />

      <div
        className="transcription-stage-list"
        aria-label="處理階段"
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
        <span>已用 {formatDuration(elapsedMs)}</span>
        <span className="truncate" title={processedSpeech ?? undefined}>
          {processedSpeech ? `已處理 ${processedSpeech}` : "正在估算語音量"}
        </span>
      </div>
    </section>
  );
}
