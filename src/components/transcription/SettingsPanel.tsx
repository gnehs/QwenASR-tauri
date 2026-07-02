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
  ffmpeg,
  onSelectModel,
  onDownload,
  onRefresh,
}: {
  models: ModelStatus[];
  selectedModelId: string;
  downloadProgress: DownloadProgress | null;
  isDownloading: boolean;
  ffmpeg: FfmpegStatus;
  onSelectModel: (value: string) => void;
  onDownload: (modelId?: string) => void;
  onRefresh: () => void;
}) {
  return (
    <div className="settings-grid">
      <ModelPanel
        models={models}
        selectedModelId={selectedModelId}
        downloadProgress={downloadProgress}
        isDownloading={isDownloading}
        onSelectModel={onSelectModel}
        onDownload={onDownload}
        onRefresh={onRefresh}
      />
      <RuntimePanel ffmpeg={ffmpeg} />
    </div>
  );
}
