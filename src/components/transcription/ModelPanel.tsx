import { useState } from "react";
import {
  DownloadIcon,
  HardDriveIcon,
  RefreshCwIcon,
  Trash2Icon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import { formatBytes } from "@/lib/format";
import type { DownloadProgress, ModelStatus } from "@/types/transcription";

const modelComparisonMeta: Record<string, string> = {
  "qwen3-asr-0.6b": "速度：快 · 品質：標準",
  "qwen3-asr-1.7b": "速度：較慢 · 品質：高",
  "qwen3-forced-aligner-0.6b": "用途：產生精準字幕時間",
};

function modelSizeLabel(sizeHint: string) {
  return sizeHint.replace(/^~\s*/, "約 ");
}

export function ModelPanel({
  models,
  downloadProgress,
  downloadMovingAverageSpeedBytesPerSec,
  isDownloading,
  deletingModelId,
  isTranscribing,
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
  onDownload: (modelId?: string) => void;
  onDeleteModel: (modelId: string) => Promise<boolean>;
  onRefresh: () => void;
}) {
  const [deleteDialogModelId, setDeleteDialogModelId] = useState<string | null>(
    null,
  );
  const isDeletingModel = deletingModelId !== null;

  return (
    <section className="settings-section" aria-labelledby="model-panel-title">
      <div className="settings-section-header">
        <h2 id="model-panel-title" className="settings-section-title">
          模型
        </h2>
        <p className="settings-section-description">
          在這裡管理 QwenASR 模型。
        </p>
      </div>
      <div className="settings-section-content">
        {models.length === 0 ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <DownloadIcon />
              </EmptyMedia>
              <EmptyTitle>尚未取得模型清單</EmptyTitle>
              <EmptyDescription>
                請重新整理模型與工具狀態；在 Tauri app 內會從後端讀取支援模型。
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <Button variant="outline" onClick={onRefresh}>
                <RefreshCwIcon data-icon="inline-start" />
                重新整理
              </Button>
            </EmptyContent>
          </Empty>
        ) : (
          <div className="model-list">
            {models.map((model) => {
              const comparisonMeta = modelComparisonMeta[model.id];
              const activeDownloadProgress =
                isDownloading && downloadProgress?.modelId === model.id
                  ? downloadProgress
                  : null;
              const isActiveDownload = activeDownloadProgress !== null;
              const isDeletingThisModel = deletingModelId === model.id;
              const isDeleteDialogOpen = deleteDialogModelId === model.id;
              const canDeleteModel =
                model.installed &&
                !isDownloading &&
                !isDeletingModel &&
                !isTranscribing;

              return (
                <Card key={model.id}>
                  <CardHeader>
                    <CardTitle className="model-card-title">
                      <span>{model.title}</span>
                      {model.recommended ? (
                        <Badge variant="secondary">建議</Badge>
                      ) : model.role === "forcedAlignment" ? (
                        <Badge variant="outline">字幕對齊</Badge>
                      ) : null}
                    </CardTitle>
                    <CardDescription>{model.description}</CardDescription>
                  </CardHeader>

                  <CardContent className="model-card-content">
                    {comparisonMeta ? (
                      <p className="model-card-comparison">
                        {comparisonMeta}
                      </p>
                    ) : null}
                  </CardContent>

                  <CardFooter className="model-card-footer">
                    <div className="model-card-footer-row">
                      <div className="model-card-meta">
                        <span className="model-card-meta-item">
                          <HardDriveIcon className="size-4" />
                          下載大小{modelSizeLabel(model.sizeHint)}
                        </span>
                        {isActiveDownload ? (
                          <span className="model-card-meta-item model-card-status">
                            <Spinner />
                            下載中
                          </span>
                        ) : null}
                      </div>

                      {!model.installed && !isActiveDownload ? (
                        <Button
                          size="sm"
                          disabled={isDownloading || isDeletingModel}
                          onClick={() => onDownload(model.id)}
                        >
                          <DownloadIcon data-icon="inline-start" />
                          下載模型
                        </Button>
                      ) : null}

                      {model.installed ? (
                        <Dialog
                          open={isDeleteDialogOpen}
                          onOpenChange={(open) =>
                            setDeleteDialogModelId(open ? model.id : null)
                          }
                          disablePointerDismissal={isDeletingThisModel}
                        >
                          <DialogTrigger
                            disabled={!canDeleteModel}
                            render={
                              <Button
                                variant="outline"
                                size="sm"
                                disabled={!canDeleteModel}
                              />
                            }
                          >
                            <Trash2Icon data-icon="inline-start" />
                            移除
                          </DialogTrigger>
                          <DialogContent
                            showCloseButton={!isDeletingThisModel}
                          >
                            <DialogHeader>
                              <DialogTitle>移除 {model.title}</DialogTitle>
                              <DialogDescription>
                                這會移除已下載的模型檔案；之後需重新下載才能使用此模型。
                              </DialogDescription>
                            </DialogHeader>
                            <DialogFooter>
                              <DialogClose
                                render={<Button variant="outline" />}
                                disabled={isDeletingThisModel}
                              >
                                取消
                              </DialogClose>
                              <Button
                                variant="destructive"
                                disabled={isDeletingThisModel}
                                onClick={async () => {
                                  const deleted = await onDeleteModel(model.id);
                                  if (deleted) {
                                    setDeleteDialogModelId(null);
                                  }
                                }}
                              >
                                {isDeletingThisModel ? (
                                  <Spinner data-icon="inline-start" />
                                ) : (
                                  <Trash2Icon data-icon="inline-start" />
                                )}
                                移除模型
                              </Button>
                            </DialogFooter>
                          </DialogContent>
                        </Dialog>
                      ) : null}
                    </div>

                    {isActiveDownload ? (
                      <div className="model-download-footer">
                        <Progress
                          aria-label={`${model.title} 下載進度`}
                          className="model-download-progress"
                          value={activeDownloadProgress.percent}
                        >
                          <ProgressLabel className="min-w-0 flex-1 truncate">
                            正在下載模型檔案
                          </ProgressLabel>
                          <ProgressValue className="shrink-0 text-foreground">
                            {() =>
                              `${activeDownloadProgress.percent.toFixed(0)}%`
                            }
                          </ProgressValue>
                        </Progress>
                        <div className="model-download-meta">
                          <span className="truncate">
                            {activeDownloadProgress.currentFile ?? "準備下載"}
                          </span>
                          <span>
                            {activeDownloadProgress.fileIndex}/
                            {activeDownloadProgress.totalFiles} 個檔案
                          </span>
                          <span>
                            {formatBytes(
                              activeDownloadProgress.fileBytesCompleted,
                            )} /{" "}
                            {formatBytes(
                              activeDownloadProgress.fileTotalBytes,
                            )}
                          </span>
                          <span>
                            {formatBytes(
                              downloadMovingAverageSpeedBytesPerSec,
                            )}
                            /s
                          </span>
                        </div>
                      </div>
                    ) : null}
                  </CardFooter>
                </Card>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
