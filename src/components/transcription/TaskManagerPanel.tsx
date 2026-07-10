import { useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import {
  ArchiveIcon,
  DownloadIcon,
  FileAudioIcon,
  FolderOpenIcon,
  PlusIcon,
  RotateCcwIcon,
  Trash2Icon,
  TriangleAlertIcon,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
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
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSet,
  FieldTitle,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { Spinner } from "@/components/ui/spinner";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { TranscriptionProgressPanel } from "@/components/transcription/TranscriptionProgressPanel";
import { languageItems } from "@/lib/app-constants";
import {
  basename,
  formatBytes,
  formatDuration,
  formatTimestamp,
} from "@/lib/format";
import { cn } from "@/lib/utils";
import type {
  DownloadProgress,
  ModelStatus,
  TaskDraft,
  TaskStatus,
  TranscriptionTask,
} from "@/types/transcription";

const statusMeta: Record<
  TaskStatus,
  {
    label: string;
    badge: "default" | "secondary" | "destructive" | "outline";
  }
> = {
  queued: { label: "排隊中", badge: "outline" },
  running: { label: "轉錄中", badge: "default" },
  completed: { label: "完成", badge: "secondary" },
  failed: { label: "失敗", badge: "destructive" },
};

function taskPercent(task: TranscriptionTask) {
  if (task.status === "completed") return 100;
  if (task.status === "queued") return 0;
  return Math.max(0, Math.min(100, task.progress?.percent ?? 0));
}

function taskEtaLabel(task: TranscriptionTask, now: number) {
  if (task.status !== "running") return "";
  if (task.progress?.etaMs == null || task.progressUpdatedAt == null) {
    return "ETA --";
  }

  const elapsedSinceProgress = Math.max(0, now - task.progressUpdatedAt);
  const etaMs = Math.max(0, task.progress.etaMs - elapsedSinceProgress);

  return `ETA ${formatDuration(etaMs)}`;
}

function taskLanguageLabel(task: TranscriptionTask) {
  return (
    languageItems.find((item) => item.value === task.options.language)?.label ??
    "自動偵測"
  );
}

export function TaskManagerPanel({
  tasks,
  models,
  taskDraft,
  draftModel,
  canConfirmTasks,
  isConfirmingTasks,
  isDownloading,
  isTaskDialogOpen,
  isModelDownloadDialogOpen,
  modelDownloadError,
  downloadProgress,
  downloadMovingAverageSpeedBytesPerSec,
  isDraggingFiles,
  onPickFiles,
  onPickOutputDir,
  onTaskDraftChange,
  onTaskDialogOpenChange,
  onModelDownloadDialogOpenChange,
  onConfirmTaskDraft,
  onRemoveTask,
  onRetryTask,
}: {
  tasks: TranscriptionTask[];
  models: ModelStatus[];
  taskDraft: TaskDraft;
  draftModel: ModelStatus | undefined;
  canConfirmTasks: boolean;
  isConfirmingTasks: boolean;
  isDownloading: boolean;
  isTaskDialogOpen: boolean;
  isModelDownloadDialogOpen: boolean;
  modelDownloadError: string | null;
  downloadProgress: DownloadProgress | null;
  downloadMovingAverageSpeedBytesPerSec: number;
  isDraggingFiles: boolean;
  onPickFiles: () => void;
  onPickOutputDir: () => void;
  onTaskDraftChange: Dispatch<SetStateAction<TaskDraft>>;
  onTaskDialogOpenChange: (open: boolean) => void;
  onModelDownloadDialogOpenChange: (open: boolean) => void;
  onConfirmTaskDraft: () => void;
  onRemoveTask: (taskId: string) => void;
  onRetryTask: (taskId: string) => void;
}) {
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [etaTick, setEtaTick] = useState(() => Date.now());
  const selectedTask = tasks.find((task) => task.id === selectedTaskId) ?? null;
  const hasRunningTasks = tasks.some((task) => task.status === "running");
  const modelItems = useMemo(
    () =>
      models.map((model) => ({
        label: model.title,
        value: model.id,
      })),
    [models],
  );
  const selectedLanguage =
    languageItems.find((item) => item.value === taskDraft.options.language) ??
    languageItems[0];
  const taskModelDownloadProgress = downloadProgress;
  const taskModelDownloadPercent = Math.max(
    0,
    Math.min(100, taskModelDownloadProgress?.percent ?? 0),
  );
  const taskModelDownloadSpeed = taskModelDownloadProgress
    ? `${formatBytes(downloadMovingAverageSpeedBytesPerSec)}/s`
    : "等待中";

  useEffect(() => {
    if (!hasRunningTasks) return;

    setEtaTick(Date.now());
    const timer = window.setInterval(() => {
      setEtaTick(Date.now());
    }, 1000);

    return () => window.clearInterval(timer);
  }, [hasRunningTasks]);

  useEffect(() => {
    if (!selectedTaskId || tasks.some((task) => task.id === selectedTaskId)) {
      return;
    }

    setSelectedTaskId(tasks[0]?.id ?? null);
  }, [selectedTaskId, tasks]);

  return (
    <>
      <div className={cn("task-workspace", isDraggingFiles && "is-dragging")}>
        <section className="task-list-panel" aria-label="轉錄任務">
          <div className="task-list-content">
            <ScrollArea
              className={cn("task-table-wrap", tasks.length === 0 && "is-empty")}
            >
              {tasks.length > 0 ? (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>檔案</TableHead>
                      <TableHead>狀態</TableHead>
                      <TableHead>進度</TableHead>
                      <TableHead>設定</TableHead>
                      <TableHead className="text-right">動作</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {tasks.map((task) => {
                      const meta = statusMeta[task.status];
                      const percent = taskPercent(task);

                      return (
                        <TableRow
                          key={task.id}
                          className="task-row"
                          data-selected={selectedTask?.id === task.id}
                          onClick={() => setSelectedTaskId(task.id)}
                        >
                          <TableCell className="task-name-cell">
                            <div className="min-w-0">
                              <div className="truncate font-medium">
                                {basename(task.audioPath)}
                              </div>
                              <div className="truncate text-xs text-muted-foreground">
                                {task.outputDir || "來源資料夾"}
                              </div>
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge variant={meta.badge}>{meta.label}</Badge>
                          </TableCell>
                          <TableCell className="task-progress-cell">
                            <div className="task-progress-stack">
                              <Progress
                                value={percent}
                                aria-label={`${basename(task.audioPath)} 進度`}
                              />
                              <div className="task-progress-meta">
                                <span>{percent.toFixed(0)}%</span>
                                <span>{taskEtaLabel(task, etaTick)}</span>
                              </div>
                            </div>
                          </TableCell>
                          <TableCell className="task-options-cell">
                            <div className="truncate">{task.modelTitle}</div>
                            <div className="truncate text-xs text-muted-foreground">
                              {taskLanguageLabel(task)}
                            </div>
                          </TableCell>
                          <TableCell className="text-right">
                            <div className="task-row-actions">
                              {task.status === "failed" ? (
                                <Button
                                  variant="ghost"
                                  size="icon-sm"
                                  onClick={(event) => {
                                    event.stopPropagation();
                                    onRetryTask(task.id);
                                  }}
                                >
                                  <RotateCcwIcon data-icon="inline-start" />
                                  <span className="sr-only">
                                    重試 {basename(task.audioPath)}
                                  </span>
                                </Button>
                              ) : null}
                              <Button
                                variant="ghost"
                                size="icon-sm"
                                disabled={task.status === "running"}
                                onClick={(event) => {
                                  event.stopPropagation();
                                  onRemoveTask(task.id);
                                }}
                              >
                                <Trash2Icon data-icon="inline-start" />
                                <span className="sr-only">
                                  移除 {basename(task.audioPath)}
                                </span>
                              </Button>
                            </div>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              ) : (
                <Empty className="task-empty-state">
                  <div className="task-empty-main">
                    <EmptyHeader>
                      <EmptyMedia variant="icon">
                        <ArchiveIcon />
                      </EmptyMedia>
                      <EmptyTitle>尚無任務</EmptyTitle>
                      <EmptyDescription>新增檔案後會自動排隊。</EmptyDescription>
                    </EmptyHeader>
                    <EmptyContent>
                      <Button
                        variant={isDraggingFiles ? "default" : "outline"}
                        onClick={onPickFiles}
                      >
                        <FileAudioIcon data-icon="inline-start" />
                        {isDraggingFiles ? "放開以加入任務" : "拖放或選取檔案"}
                      </Button>
                    </EmptyContent>
                  </div>
                  <p className="task-empty-supported">
                    wav、mp3、m4a、mp4、mov、mkv、webm
                  </p>
                </Empty>
              )}
            </ScrollArea>
          </div>
        </section>
      </div>

      <Sheet
        open={Boolean(selectedTask)}
        onOpenChange={(open) => {
          if (!open) setSelectedTaskId(null);
        }}
      >
        <SheetContent
          side="right"
          className="gap-0 data-[side=right]:w-[min(720px,100vw)] data-[side=right]:sm:max-w-[min(720px,100vw)]"
        >
          <SheetHeader className="pr-12">
            <div className="flex items-center gap-2">
              <SheetTitle>任務詳情</SheetTitle>
              {selectedTask ? (
                <Badge variant={statusMeta[selectedTask.status].badge}>
                  {statusMeta[selectedTask.status].label}
                </Badge>
              ) : null}
            </div>
            <SheetDescription
              className="truncate"
              title={selectedTask ? basename(selectedTask.audioPath) : undefined}
            >
              {selectedTask ? basename(selectedTask.audioPath) : "尚未選取任務"}
            </SheetDescription>
          </SheetHeader>
          <Separator />
          <div className="task-detail-content">
            {selectedTask ? (
              <TaskDetail now={etaTick} task={selectedTask} />
            ) : null}
          </div>
        </SheetContent>
      </Sheet>

      <Dialog open={isTaskDialogOpen} onOpenChange={onTaskDialogOpenChange}>
        <DialogContent className="task-dialog">
          <DialogHeader>
            <DialogTitle>新增轉錄任務</DialogTitle>
            <DialogDescription>
              {taskDraft.files.length} 個檔案會使用相同模型、語言與輸出資料夾。
            </DialogDescription>
          </DialogHeader>

          <div className="task-dialog-body">
            <FieldGroup>
              <Field>
                <FieldLabel>轉錄模型</FieldLabel>
                <Select
                  items={modelItems}
                  value={taskDraft.modelId}
                  onValueChange={(value) =>
                    onTaskDraftChange((current) => ({
                      ...current,
                      modelId: String(value),
                    }))
                  }
                >
                  <SelectTrigger className="w-full" aria-label="轉錄模型">
                    <SelectValue>
                      {draftModel?.title ?? "選擇轉錄模型"}
                    </SelectValue>
                  </SelectTrigger>
                  <SelectContent alignItemWithTrigger={false}>
                    <SelectGroup>
                      {models.map((model) => (
                        <SelectItem key={model.id} value={model.id}>
                          {model.title}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>

              <Field>
                <FieldLabel>轉錄語言</FieldLabel>
                <Select
                  items={languageItems}
                  value={taskDraft.options.language}
                  onValueChange={(value) =>
                    onTaskDraftChange((current) => ({
                      ...current,
                      options: {
                        ...current.options,
                        language: String(value),
                      },
                    }))
                  }
                >
                  <SelectTrigger className="w-full" aria-label="轉錄語言">
                    <SelectValue>{selectedLanguage.label}</SelectValue>
                  </SelectTrigger>
                  <SelectContent alignItemWithTrigger={false}>
                    <SelectGroup>
                      {languageItems.map((item) => (
                        <SelectItem key={item.value} value={item.value}>
                          {item.label}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>

              <Field>
                <FieldLabel htmlFor="task-output-dir">輸出資料夾</FieldLabel>
                <div className="flex gap-2">
                  <Input
                    id="task-output-dir"
                    readOnly
                    value={taskDraft.outputDir}
                    placeholder="預設使用來源檔案所在資料夾"
                  />
                  <Button variant="outline" onClick={onPickOutputDir}>
                    <FolderOpenIcon data-icon="inline-start" />
                    選取
                  </Button>
                </div>
              </Field>

              <FieldSet>
                <Field orientation="horizontal">
                  <FieldContent>
                    <FieldTitle id="task-write-srt-label">
                      輸出 SRT 字幕
                    </FieldTitle>
                    <FieldDescription>
                      11 種語言支援精準時間對齊；首次下載約 1.8 GB，其餘語言使用估算時間。
                    </FieldDescription>
                  </FieldContent>
                  <Switch
                    aria-labelledby="task-write-srt-label"
                    checked={taskDraft.options.writeSrt}
                    onCheckedChange={(checked) =>
                      onTaskDraftChange((current) => ({
                        ...current,
                        options: {
                          ...current.options,
                          writeSrt: checked,
                        },
                      }))
                    }
                  />
                </Field>
              </FieldSet>
            </FieldGroup>

            <Separator />

            <ScrollArea className="task-draft-files">
              {taskDraft.files.map((file) => (
                <div key={file} className="task-draft-file">
                  <FileAudioIcon />
                  <span className="truncate">{basename(file)}</span>
                </div>
              ))}
            </ScrollArea>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => onTaskDialogOpenChange(false)}
            >
              取消
            </Button>
            <Button disabled={!canConfirmTasks} onClick={onConfirmTaskDraft}>
              {isConfirmingTasks ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <PlusIcon data-icon="inline-start" />
              )}
              {isConfirmingTasks ? "新增中" : "新增任務"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={isModelDownloadDialogOpen}
        onOpenChange={(open) => {
          if (isConfirmingTasks || isDownloading) return;
          onModelDownloadDialogOpenChange(open);
        }}
        disablePointerDismissal={isConfirmingTasks || isDownloading}
      >
        <DialogContent
          className="task-download-dialog"
          showCloseButton={!isConfirmingTasks && !isDownloading}
        >
          <DialogHeader>
            <DialogTitle>下載模型</DialogTitle>
            <DialogDescription>
              {draftModel
                ? `新增任務需要先下載 ${draftModel.title}；完成後會自動加入佇列。`
                : "新增任務需要先下載模型；完成後會自動加入佇列。"}
            </DialogDescription>
          </DialogHeader>

          {modelDownloadError ? (
            <Alert variant="destructive">
              <TriangleAlertIcon />
              <AlertTitle>模型下載失敗</AlertTitle>
              <AlertDescription>{modelDownloadError}</AlertDescription>
            </Alert>
          ) : (
            <div className="task-model-download">
              <Progress
                aria-label="模型下載整體進度"
                className="task-model-download-progress"
                value={taskModelDownloadPercent}
              >
                <ProgressLabel className="min-w-0 flex-1 truncate font-normal text-muted-foreground">
                  下載速度 {taskModelDownloadSpeed}
                </ProgressLabel>
                <ProgressValue className="shrink-0 text-base font-medium text-foreground">
                  {() => `${taskModelDownloadPercent.toFixed(0)}%`}
                </ProgressValue>
              </Progress>
            </div>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              disabled={isConfirmingTasks || isDownloading}
              onClick={() => onModelDownloadDialogOpenChange(false)}
            >
              回到任務設定
            </Button>
            <Button
              disabled={
                isConfirmingTasks ||
                isDownloading ||
                !modelDownloadError ||
                !canConfirmTasks
              }
              onClick={onConfirmTaskDraft}
            >
              {isConfirmingTasks || isDownloading ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <DownloadIcon data-icon="inline-start" />
              )}
              {modelDownloadError ? "重新下載並加入佇列" : "下載中"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function TaskDetail({ now, task }: { now: number; task: TranscriptionTask }) {
  const progress = task.progress;
  const result = task.result;

  if (task.status === "failed") {
    return (
      <Alert variant="destructive">
        <TriangleAlertIcon />
        <AlertTitle>轉錄失敗</AlertTitle>
        <AlertDescription>{task.error ?? "未知錯誤"}</AlertDescription>
      </Alert>
    );
  }

  if (task.status === "queued") {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <ArchiveIcon />
          </EmptyMedia>
          <EmptyTitle>等待處理</EmptyTitle>
          <EmptyDescription>
            {task.modelTitle} · {taskLanguageLabel(task)}
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }

  if (task.status === "running") {
    const partialSegments = progress?.partialSegments ?? [];

    return (
      <div className="task-running-detail">
        <TranscriptionProgressPanel
          now={now}
          progress={progress}
          progressUpdatedAt={task.progressUpdatedAt}
        />
        <Separator />
        <section
          className="task-partial-result"
          aria-labelledby="live-transcript-title"
        >
          <div className="task-partial-result-head">
            <div className="min-w-0">
              <div id="live-transcript-title" className="text-sm font-medium">
                即時逐字稿
              </div>
              <p className="text-xs text-muted-foreground">
                時間為 VAD 推估，內容會隨轉錄進度更新。
              </p>
            </div>
            <Badge variant="outline">{partialSegments.length} 段</Badge>
          </div>
          {partialSegments.length > 0 ? (
            <ScrollArea className="task-partial-scroll">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-40">時間</TableHead>
                    <TableHead>文字</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {partialSegments.map((segment, index) => (
                    <TableRow key={`${segment.startMs}-${index}`}>
                      <TableCell className="whitespace-nowrap align-top font-mono text-xs text-muted-foreground">
                        {formatTimestamp(segment.startMs)} -{" "}
                        {formatTimestamp(segment.endMs)}
                      </TableCell>
                      <TableCell className="srt-preview-text align-top">
                        {segment.text}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </ScrollArea>
          ) : (
            <div className="task-partial-empty text-sm text-muted-foreground">
              辨識到語音後，逐字稿會顯示在這裡。
            </div>
          )}
        </section>
      </div>
    );
  }

  if (!result) return null;

  return (
    <ScrollArea className="task-result-scroll">
      <div className="task-result-stack">
        <div className="task-result-meta">
          <div>
            <div className="break-words text-sm font-medium">
              {basename(result.audioPath)}
            </div>
            <div className="truncate text-sm text-muted-foreground">
              {result.srtPath ? `SRT: ${result.srtPath}` : "未輸出 SRT"}
            </div>
          </div>
          <Badge variant="outline">
            總處理 {formatDuration(result.durationMs)}
          </Badge>
        </div>
        <Textarea readOnly value={result.text} className="min-h-36 resize-none" />
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>時間</TableHead>
              <TableHead>文字</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {result.segments.map((segment, index) => (
              <TableRow key={`${segment.startMs}-${index}`}>
                <TableCell className="whitespace-nowrap text-muted-foreground">
                  {formatTimestamp(segment.startMs)} - {formatTimestamp(segment.endMs)}
                </TableCell>
                <TableCell className="srt-preview-text">{segment.text}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </ScrollArea>
  );
}
