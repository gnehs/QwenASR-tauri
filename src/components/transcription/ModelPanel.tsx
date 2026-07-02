import { CheckCircle2Icon, DownloadIcon, RefreshCwIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Progress } from "@/components/ui/progress";
import { Spinner } from "@/components/ui/spinner";
import { formatBytes } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { DownloadProgress, ModelStatus } from "@/types/transcription";

export function ModelPanel({
  models,
  selectedModelId,
  downloadProgress,
  isDownloading,
  onSelectModel,
  onDownload,
  onRefresh,
}: {
  models: ModelStatus[];
  selectedModelId: string;
  downloadProgress: DownloadProgress | null;
  isDownloading: boolean;
  onSelectModel: (value: string) => void;
  onDownload: (modelId?: string) => void;
  onRefresh: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>模型</CardTitle>
        <CardDescription>選擇要使用的 QwenASR 模型。</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
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
            const isSelected = model.id === selectedModelId;
            const isActiveDownload =
              isDownloading && downloadProgress?.modelId === model.id;

            return (
              <div
                key={model.id}
                className={cn("model-card", isSelected && "is-selected")}
              >
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <div className="truncate text-sm font-medium">
                      {model.title}
                    </div>
                    {model.recommended ? (
                      <Badge variant="secondary">建議</Badge>
                    ) : null}
                    {isSelected ? <Badge>使用中</Badge> : null}
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">
                    {model.description}
                  </p>
                  <div className="mt-3 flex flex-wrap gap-2">
                    <Badge variant="outline">{model.sizeHint}</Badge>
                    <Badge variant={model.installed ? "secondary" : "outline"}>
                      {model.installed ? "已安裝" : "未下載"}
                    </Badge>
                  </div>
                </div>
                <Button
                  variant={model.installed ? "outline" : "default"}
                  disabled={isActiveDownload || (model.installed && isSelected)}
                  onClick={() => {
                    onSelectModel(model.id);
                    if (!model.installed) {
                      onDownload(model.id);
                    }
                  }}
                >
                  {isActiveDownload ? (
                    <Spinner data-icon="inline-start" />
                  ) : model.installed ? (
                    <CheckCircle2Icon data-icon="inline-start" />
                  ) : (
                    <DownloadIcon data-icon="inline-start" />
                  )}
                  {model.installed
                    ? isSelected
                      ? "使用中"
                      : "使用此模型"
                    : "下載模型"}
                </Button>
              </div>
            );
            })}
          </div>
        )}

        {downloadProgress ? (
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between gap-3 text-sm">
              <span className="truncate">{downloadProgress.message}</span>
              <span className="tabular-nums text-muted-foreground">
                {formatBytes(downloadProgress.speedBytesPerSec)}/s
              </span>
            </div>
            <Progress value={downloadProgress.percent}>
              <span className="ml-auto text-sm tabular-nums text-muted-foreground">
                {downloadProgress.percent.toFixed(0)}%
              </span>
            </Progress>
            <div className="truncate text-xs text-muted-foreground">
              {downloadProgress.currentFile ?? "模型"} ·{" "}
              {downloadProgress.fileIndex}/{downloadProgress.totalFiles} ·{" "}
              {formatBytes(downloadProgress.fileBytesCompleted)} /{" "}
              {formatBytes(downloadProgress.fileTotalBytes)}
            </div>
          </div>
        ) : null}

      </CardContent>
    </Card>
  );
}
