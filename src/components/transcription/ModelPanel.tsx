import { useState } from "react";
import { msg } from "@lingui/core/macro";
import { useLingui as useLinguiRuntime } from "@lingui/react";
import { Trans, useLingui as useLinguiMacro } from "@lingui/react/macro";
import {
  DownloadIcon,
  EllipsisIcon,
  FolderOpenIcon,
  HardDriveIcon,
  RefreshCwIcon,
  Trash2Icon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
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
import { SettingsSection } from "@/components/transcription/SettingsSection";
import type { DownloadProgress, ModelStatus } from "@/types/transcription";

const modelDescriptionMessages = {
  "qwen3-asr-0.6b": msg`快速、適合大多數單次與批次轉錄工作。`,
  "qwen3-asr-1.7b": msg`較高準確度，適合重要錄音或較複雜的聲學環境。`,
  "qwen3-forced-aligner-0.6b": msg`對齊音訊與逐字稿，產生精準的字詞級字幕時間戳。`,
};

const approximateLabel = msg`約`;

export function ModelPanel({
  models,
  downloadProgress,
  downloadMovingAverageSpeedBytesPerSec,
  isDownloading,
  deletingModelId,
  isTranscribing,
  onDownload,
  onDeleteModel,
  onOpenModelFolder,
  onRedownload,
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
  onOpenModelFolder: (modelId: string) => Promise<void>;
  onRedownload: (modelId: string) => Promise<void>;
  onRefresh: () => void;
}) {
  const { _ } = useLinguiRuntime();
  const { t } = useLinguiMacro();
  const [deleteDialogModelId, setDeleteDialogModelId] = useState<string | null>(
    null,
  );
  const isDeletingModel = deletingModelId !== null;

  return (
    <SettingsSection
      id="model-panel-title"
      title={<Trans>模型</Trans>}
      description={<Trans>在這裡管理 QwenASR 模型。</Trans>}
    >
      {models.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <DownloadIcon />
            </EmptyMedia>
            <EmptyTitle>
              <Trans>尚未取得模型清單</Trans>
            </EmptyTitle>
            <EmptyDescription>
              <Trans>
                請重新整理模型與工具狀態；在 Tauri app 內會從後端讀取支援模型。
              </Trans>
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button variant="outline" onClick={onRefresh}>
              <RefreshCwIcon data-icon="inline-start" />
              <Trans>重新整理</Trans>
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="flex flex-col gap-3">
          {models.map((model) => {
            const descriptionMessage =
              modelDescriptionMessages[
                model.id as keyof typeof modelDescriptionMessages
              ];
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
                  <CardTitle className="flex min-w-0 flex-wrap items-center gap-2">
                    <span>{model.title}</span>
                    {model.recommended ? (
                      <Badge variant="secondary">
                        <Trans>建議</Trans>
                      </Badge>
                    ) : null}
                  </CardTitle>
                  {model.installed ? (
                    <CardAction>
                      <DropdownMenu>
                        <DropdownMenuTrigger
                          render={
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              aria-label={t`${model.title} 更多操作`}
                            />
                          }
                        >
                          <EllipsisIcon />
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-48">
                          <DropdownMenuItem
                            onClick={() => void onOpenModelFolder(model.id)}
                          >
                            <FolderOpenIcon />
                            <Trans>在 Finder 顯示</Trans>
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            disabled={!canDeleteModel}
                            onClick={() => void onRedownload(model.id)}
                          >
                            <RefreshCwIcon />
                            <Trans>重新下載</Trans>
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </CardAction>
                  ) : null}
                  <CardDescription>
                    {descriptionMessage
                      ? _(descriptionMessage)
                      : model.description}
                  </CardDescription>
                </CardHeader>

                <CardFooter className="flex flex-col items-stretch gap-3 py-2">
                  <div className="flex min-w-0 flex-wrap items-center justify-between gap-2.5">
                    <div className="flex min-w-0 flex-wrap items-center gap-x-3.5 gap-y-2 text-[0.8125rem] text-muted-foreground">
                      <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
                        <HardDriveIcon className="size-4" />
                        <Trans>
                          下載大小
                          {model.sizeHint.replace(
                            /^~\s*/,
                            `${_(approximateLabel)} `,
                          )}
                        </Trans>
                      </span>
                      {isActiveDownload ? (
                        <span className="inline-flex items-center gap-1.5 font-medium whitespace-nowrap text-foreground">
                          <Spinner />
                          <Trans>下載中</Trans>
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
                        <Trans>下載模型</Trans>
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
                          <Trans>移除</Trans>
                        </DialogTrigger>
                        <DialogContent showCloseButton={!isDeletingThisModel}>
                          <DialogHeader>
                            <DialogTitle>
                              <Trans>移除 {model.title}</Trans>
                            </DialogTitle>
                            <DialogDescription>
                              <Trans>
                                這會移除已下載的模型檔案；之後需重新下載才能使用此模型。
                              </Trans>
                            </DialogDescription>
                          </DialogHeader>
                          <DialogFooter>
                            <DialogClose
                              render={<Button variant="outline" />}
                              disabled={isDeletingThisModel}
                            >
                              <Trans>取消</Trans>
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
                              <Trans>移除模型</Trans>
                            </Button>
                          </DialogFooter>
                        </DialogContent>
                      </Dialog>
                    ) : null}
                  </div>

                  {isActiveDownload ? (
                    <div className="flex min-w-0 flex-col items-stretch gap-2.5">
                      <Progress
                        aria-label={t`${model.title} 下載進度`}
                        className="gap-2"
                        value={activeDownloadProgress.percent}
                      >
                        <ProgressLabel className="min-w-0 flex-1 truncate">
                          <Trans>正在下載模型檔案</Trans>
                        </ProgressLabel>
                        <ProgressValue className="shrink-0 text-foreground">
                          {() =>
                            `${activeDownloadProgress.percent.toFixed(0)}%`
                          }
                        </ProgressValue>
                      </Progress>
                      <div className="grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-x-3.5 gap-y-2 text-xs/relaxed text-muted-foreground tabular-nums">
                        <span className="truncate">
                          {activeDownloadProgress.currentFile ?? (
                            <Trans>準備下載</Trans>
                          )}
                        </span>
                        <span>
                          <Trans>
                            {activeDownloadProgress.fileIndex}/
                            {activeDownloadProgress.totalFiles} 個檔案
                          </Trans>
                        </span>
                        <span>
                          {formatBytes(
                            activeDownloadProgress.fileBytesCompleted,
                          )}{" "}
                          / {formatBytes(activeDownloadProgress.fileTotalBytes)}
                        </span>
                        <span>
                          {formatBytes(downloadMovingAverageSpeedBytesPerSec)}
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
    </SettingsSection>
  );
}
