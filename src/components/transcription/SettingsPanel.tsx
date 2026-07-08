import { ModelPanel } from "@/components/transcription/ModelPanel";
import { RuntimePanel } from "@/components/transcription/RuntimePanel";
import type {
  DownloadProgress,
  FfmpegStatus,
  ModelStatus,
} from "@/types/transcription";

export function SettingsPanel({
  models,
  downloadProgress,
  downloadMovingAverageSpeedBytesPerSec,
  isDownloading,
  deletingModelId,
  isTranscribing,
  ffmpeg,
  onDownload,
  onDeleteModel,
  onRefresh,
}: {
  models: ModelStatus[];
  downloadProgress: DownloadProgress | null;
  downloadMovingAverageSpeedBytesPerSec: number;
  isDownloading: boolean;
  deletingModelId: string | null;
  isTranscribing: boolean;
  ffmpeg: FfmpegStatus;
  onDownload: (modelId?: string) => void;
  onDeleteModel: (modelId: string) => Promise<boolean>;
  onRefresh: () => void;
}) {
  return (
    <div className="settings-grid">
      <ModelPanel
        models={models}
        downloadProgress={downloadProgress}
        downloadMovingAverageSpeedBytesPerSec={
          downloadMovingAverageSpeedBytesPerSec
        }
        isDownloading={isDownloading}
        deletingModelId={deletingModelId}
        isTranscribing={isTranscribing}
        onDownload={onDownload}
        onDeleteModel={onDeleteModel}
        onRefresh={onRefresh}
      />
      <RuntimePanel ffmpeg={ffmpeg} />
    </div>
  );
}
