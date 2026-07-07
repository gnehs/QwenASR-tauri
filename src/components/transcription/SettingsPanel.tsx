import { ModelPanel } from "@/components/transcription/ModelPanel";
import { RuntimePanel } from "@/components/transcription/RuntimePanel";
import type {
  DownloadProgress,
  FfmpegStatus,
  ModelStatus,
} from "@/types/transcription";

export function SettingsPanel({
  models,
  selectedModelId,
  downloadProgress,
  isDownloading,
  deletingModelId,
  isTranscribing,
  ffmpeg,
  onSelectModel,
  onDownload,
  onDeleteModel,
  onRefresh,
}: {
  models: ModelStatus[];
  selectedModelId: string;
  downloadProgress: DownloadProgress | null;
  isDownloading: boolean;
  deletingModelId: string | null;
  isTranscribing: boolean;
  ffmpeg: FfmpegStatus;
  onSelectModel: (value: string) => void;
  onDownload: (modelId?: string) => void;
  onDeleteModel: (modelId: string) => Promise<boolean>;
  onRefresh: () => void;
}) {
  return (
    <div className="settings-grid">
      <ModelPanel
        models={models}
        selectedModelId={selectedModelId}
        downloadProgress={downloadProgress}
        isDownloading={isDownloading}
        deletingModelId={deletingModelId}
        isTranscribing={isTranscribing}
        onSelectModel={onSelectModel}
        onDownload={onDownload}
        onDeleteModel={onDeleteModel}
        onRefresh={onRefresh}
      />
      <RuntimePanel ffmpeg={ffmpeg} />
    </div>
  );
}
