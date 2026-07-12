import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui as useLinguiRuntime } from "@lingui/react";
import { Trans, useLingui } from "@lingui/react/macro";
import {
  ArchiveIcon,
  CircleStopIcon,
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
import { Checkbox } from "@/components/ui/checkbox";
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
  FieldLegend,
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { TranscriptionProgressPanel } from "@/components/transcription/TranscriptionProgressPanel";
import { localizeLanguageGroups } from "@/lib/app-constants";
import {
  basename,
  formatBytes,
  formatDuration,
  formatTimestamp,
  formatTiming,
} from "@/lib/format";
import type {
  DownloadProgress,
  ModelStatus,
  SelectOption,
  TaskDraft,
  TaskStatus,
  TranscriptionTask,
  TranscriptionTimings,
} from "@/types/transcription";

const statusMeta: Record<
  TaskStatus,
  {
    badge: "default" | "secondary" | "destructive" | "outline";
  }
> = {
  queued: { badge: "outline" },
  running: { badge: "default" },
  completed: { badge: "secondary" },
  failed: { badge: "destructive" },
  cancelled: { badge: "secondary" },
};

const statusMessages: Record<TaskStatus, MessageDescriptor> = {
  queued: msg`排隊中`,
  running: msg`轉錄中`,
  completed: msg`完成`,
  failed: msg`失敗`,
  cancelled: msg`已終止`,
};

type Translate = ReturnType<typeof useLingui>["t"];
type RuntimeTranslate = ReturnType<typeof useLinguiRuntime>["_"];

function statusLabel(status: TaskStatus, translate: RuntimeTranslate) {
  return translate(statusMessages[status]);
}

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

function taskLanguageLabel(
  task: TranscriptionTask,
  t: Translate,
  localizedLanguageItems: SelectOption[],
) {
  if (task.options.language === "auto") return t`自動偵測`;

  return (
    localizedLanguageItems.find((item) => item.value === task.options.language)
      ?.label ?? t`自動偵測`
  );
}

function timingRows(
  timings: TranscriptionTimings,
  translate: RuntimeTranslate,
) {
  return [
    [
      translate(msg`總時間`),
      formatTiming(timings.totalMs),
      translate(msg`ASR`),
      formatTiming(timings.asrMs),
    ],
    [
      translate(msg`解碼`),
      formatTiming(timings.asrDecodeMs),
      translate(msg`編碼器`),
      formatTiming(timings.asrEncoderMs),
    ],
    [
      translate(msg`VAD`),
      formatTiming(timings.vadMs),
      translate(msg`對齊`),
      formatTiming(timings.alignmentMs),
    ],
    [
      translate(msg`音訊準備`),
      formatTiming(timings.audioPrepareMs),
      translate(msg`收尾`),
      formatTiming(timings.finalizeMs),
    ],
    [
      translate(msg`Mel 特徵`),
      formatTiming(timings.asrMelMs),
      translate(msg`提示詞`),
      formatTiming(timings.asrPromptMs),
    ],
    [
      translate(msg`預填`),
      formatTiming(timings.asrPrefillMs),
      translate(msg`後處理`),
      formatTiming(timings.asrPostprocessMs),
    ],
    [
      translate(msg`ASR 分段`),
      String(timings.asrChunkCount),
      translate(msg`產生 token`),
      String(timings.asrGeneratedTokens),
    ],
  ] as const;
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
  onCancelTask,
  onRemoveTask,
  onRetryTask,
  cancellingTaskId,
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
  onCancelTask: (taskId: string) => void;
  onRemoveTask: (taskId: string) => void;
  onRetryTask: (taskId: string) => void;
  cancellingTaskId: string | null;
}) {
  const { t } = useLingui();
  const { _ } = useLinguiRuntime();
  const outputId = useId();
  const segmentId = useId();
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
  const localizedLanguageGroups = useMemo(() => localizeLanguageGroups(_), [_]);
  const localizedLanguageItems = useMemo(
    () => localizedLanguageGroups.flatMap((group) => group.items),
    [localizedLanguageGroups],
  );
  const selectedLanguage =
    localizedLanguageItems.find(
      (item) => item.value === taskDraft.options.language,
    ) ?? localizedLanguageItems[0];
  const taskModelDownloadProgress = downloadProgress;
  const taskModelDownloadPercent = Math.max(
    0,
    Math.min(100, taskModelDownloadProgress?.percent ?? 0),
  );
  const taskModelDownloadSpeed = taskModelDownloadProgress
    ? `${formatBytes(downloadMovingAverageSpeedBytesPerSec)}/s`
    : t`等待中`;

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
      <div className="flex h-full min-h-0 flex-col gap-3">
        <section
          className="flex min-h-0 flex-1 flex-col gap-3"
          aria-label={t`轉錄任務`}
        >
          <div className="flex min-h-0 flex-1 flex-col gap-3">
            <ScrollArea className="relative min-h-0 flex-1">
              {tasks.length > 0 ? (
                <Table className="[&_td:first-child]:pl-4 [&_td:last-child]:pr-4 [&_th:first-child]:pl-4 [&_th:last-child]:pr-4">
                  <TableHeader>
                    <TableRow>
                      <TableHead>
                        <Trans>檔案</Trans>
                      </TableHead>
                      <TableHead>
                        <Trans>狀態</Trans>
                      </TableHead>
                      <TableHead>
                        <Trans>進度</Trans>
                      </TableHead>
                      <TableHead>
                        <Trans>設定</Trans>
                      </TableHead>
                      <TableHead className="text-right">
                        <Trans>動作</Trans>
                      </TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {tasks.map((task) => {
                      const meta = statusMeta[task.status];
                      const percent = taskPercent(task);

                      return (
                        <TableRow
                          key={task.id}
                          className="cursor-pointer data-[selected=true]:bg-primary/5"
                          data-selected={selectedTask?.id === task.id}
                          onClick={() => setSelectedTaskId(task.id)}
                        >
                          <TableCell className="w-[42%] max-w-0 max-[720px]:min-w-[180px]">
                            <div className="min-w-0">
                              <div className="truncate font-medium">
                                {basename(task.audioPath)}
                              </div>
                              <div className="truncate text-xs text-muted-foreground">
                                {task.outputDir || <Trans>來源資料夾</Trans>}
                              </div>
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge variant={meta.badge}>
                              {statusLabel(task.status, _)}
                            </Badge>
                          </TableCell>
                          <TableCell className="min-w-[170px]">
                            <div className="flex min-w-0 flex-col gap-1.5">
                              <Progress
                                value={percent}
                                aria-label={t`${basename(task.audioPath)} 進度`}
                              />
                              <div className="flex min-w-0 items-center justify-between gap-2.5 text-xs/none text-muted-foreground tabular-nums">
                                <span>{percent.toFixed(0)}%</span>
                                <span>{taskEtaLabel(task, etaTick)}</span>
                              </div>
                            </div>
                          </TableCell>
                          <TableCell className="max-w-[180px] max-[720px]:min-w-[180px]">
                            <div className="truncate">{task.modelTitle}</div>
                            <div className="truncate text-xs text-muted-foreground">
                              {taskLanguageLabel(
                                task,
                                t,
                                localizedLanguageItems,
                              )}
                            </div>
                          </TableCell>
                          <TableCell className="text-right">
                            <div className="flex items-center justify-end gap-2">
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
                                    {t`重試 ${basename(task.audioPath)}`}
                                  </span>
                                </Button>
                              ) : null}
                              {task.status === "running" ? (
                                <Button
                                  variant="ghost"
                                  size="icon-sm"
                                  disabled={cancellingTaskId === task.id}
                                  onClick={(event) => {
                                    event.stopPropagation();
                                    onCancelTask(task.id);
                                  }}
                                >
                                  {cancellingTaskId === task.id ? (
                                    <Spinner />
                                  ) : (
                                    <CircleStopIcon data-icon="inline-start" />
                                  )}
                                  <span className="sr-only">
                                    {cancellingTaskId === task.id
                                      ? t`終止中 ${basename(task.audioPath)}`
                                      : t`終止 ${basename(task.audioPath)}`}
                                  </span>
                                </Button>
                              ) : (
                                <Button
                                  variant="ghost"
                                  size="icon-sm"
                                  onClick={(event) => {
                                    event.stopPropagation();
                                    onRemoveTask(task.id);
                                  }}
                                >
                                  <Trash2Icon data-icon="inline-start" />
                                  <span className="sr-only">
                                    {t`移除 ${basename(task.audioPath)}`}
                                  </span>
                                </Button>
                              )}
                            </div>
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              ) : (
                <Empty className="h-full min-h-[360px] justify-between gap-6 px-6 pb-6">
                  <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4">
                    <EmptyHeader>
                      <EmptyMedia variant="icon" className="size-14 rounded-xl">
                        <ArchiveIcon className="size-8" />
                      </EmptyMedia>
                      <EmptyTitle className="text-2xl leading-tight font-semibold">
                        <Trans>尚無任務</Trans>
                      </EmptyTitle>
                      <EmptyDescription>
                        <Trans>拖放檔案到這裡來建立任務</Trans>
                      </EmptyDescription>
                    </EmptyHeader>
                    <EmptyContent>
                      <Button
                        variant={isDraggingFiles ? "default" : "outline"}
                        onClick={onPickFiles}
                      >
                        <FileAudioIcon data-icon="inline-start" />
                        {isDraggingFiles ? (
                          <Trans>放開以加入任務</Trans>
                        ) : (
                          <Trans>選擇檔案</Trans>
                        )}
                      </Button>
                    </EmptyContent>
                  </div>
                  <p className="m-0 text-center text-sm/relaxed text-muted-foreground">
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
              <SheetTitle>
                <Trans>任務詳情</Trans>
              </SheetTitle>
              {selectedTask ? (
                <Badge variant={statusMeta[selectedTask.status].badge}>
                  {statusLabel(selectedTask.status, _)}
                </Badge>
              ) : null}
            </div>
            <SheetDescription
              className="truncate"
              title={
                selectedTask ? basename(selectedTask.audioPath) : undefined
              }
            >
              {selectedTask ? (
                basename(selectedTask.audioPath)
              ) : (
                <Trans>尚未選取任務</Trans>
              )}
            </SheetDescription>
          </SheetHeader>
          <Separator />
          <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden p-3.5 px-4 pb-4">
            {selectedTask ? (
              <TaskDetail
                now={etaTick}
                task={selectedTask}
                localizedLanguageItems={localizedLanguageItems}
              />
            ) : null}
          </div>
        </SheetContent>
      </Sheet>

      <Dialog open={isTaskDialogOpen} onOpenChange={onTaskDialogOpenChange}>
        <DialogContent className="w-[min(920px,calc(100vw-2rem))] !max-w-[min(920px,calc(100vw-2rem))]">
          <DialogHeader>
            <DialogTitle>
              <Trans>新增轉錄任務</Trans>
            </DialogTitle>
          </DialogHeader>

          <div className="grid max-h-[min(620px,calc(100vh-220px))] min-h-0 scroll-fade grid-cols-1 gap-x-6 gap-y-5 overflow-x-hidden overflow-y-auto p-1 md:grid-cols-2">
            <FieldGroup className="min-w-0">
              <Field>
                <FieldLabel>
                  <Trans>轉錄模型</Trans>
                </FieldLabel>
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
                  <SelectTrigger className="w-full" aria-label={t`轉錄模型`}>
                    <SelectValue>
                      {draftModel?.title ?? <Trans>選擇轉錄模型</Trans>}
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
                <FieldLabel>
                  <Trans>轉錄語言</Trans>
                </FieldLabel>
                <Select
                  items={localizedLanguageItems}
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
                  <SelectTrigger className="w-full" aria-label={t`轉錄語言`}>
                    <SelectValue>{selectedLanguage.label}</SelectValue>
                  </SelectTrigger>
                  <SelectContent alignItemWithTrigger={false}>
                    {localizedLanguageGroups.map((group) => (
                      <SelectGroup key={group.id}>
                        <SelectLabel>{group.label}</SelectLabel>
                        {group.items.map((item) => (
                          <SelectItem key={item.value} value={item.value}>
                            {item.label}
                          </SelectItem>
                        ))}
                      </SelectGroup>
                    ))}
                  </SelectContent>
                </Select>
              </Field>

              <Field>
                <div className="flex items-center justify-between gap-3">
                  <FieldLabel htmlFor="task-output-dir">
                    <Trans>輸出資料夾</Trans>
                  </FieldLabel>
                  {taskDraft.outputDir ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      type="button"
                      onClick={() =>
                        onTaskDraftChange((current) => ({
                          ...current,
                          outputDir: "",
                        }))
                      }
                    >
                      <RotateCcwIcon data-icon="inline-start" />
                      <Trans>重置</Trans>
                    </Button>
                  ) : null}
                </div>
                <div className="flex gap-2">
                  <Input
                    id="task-output-dir"
                    readOnly
                    value={taskDraft.outputDir}
                    placeholder={t`預設使用來源檔案所在資料夾`}
                  />
                  <Button variant="outline" onClick={onPickOutputDir}>
                    <FolderOpenIcon data-icon="inline-start" />
                    <Trans>選取</Trans>
                  </Button>
                </div>
              </Field>

              <Field>
                <FieldLabel htmlFor="task-context">
                  <Trans>辨識提示詞</Trans>
                </FieldLabel>
                <Textarea
                  id="task-context"
                  className="min-h-24 resize-y"
                  value={taskDraft.options.context}
                  placeholder={t`例如：AI、LLM、台灣`}
                  onChange={(event) =>
                    onTaskDraftChange((current) => ({
                      ...current,
                      options: {
                        ...current.options,
                        context: event.target.value,
                      },
                    }))
                  }
                />
                <FieldDescription>
                  <Trans>
                    輸入容易辨識錯誤的人名、專有名詞或對話背景，幫助模型選擇正確用字。
                  </Trans>
                </FieldDescription>
              </Field>
            </FieldGroup>

            <FieldGroup className="min-w-0">
              <FieldSet>
                <FieldLegend variant="label">
                  <Trans>輸出格式</Trans>
                </FieldLegend>
                <FieldDescription>
                  <Trans>可同時輸出多種格式至指定資料夾。</Trans>
                </FieldDescription>
                <FieldGroup className="gap-3">
                  <Field orientation="horizontal">
                    <FieldLabel htmlFor={`${outputId}-txt`}>
                      <FieldContent>
                        <FieldTitle>
                          TXT <Trans>文字</Trans>
                        </FieldTitle>
                        <FieldDescription id={`${outputId}-txt-description`}>
                          <Trans>輸出純文字逐字稿。</Trans>
                        </FieldDescription>
                      </FieldContent>
                    </FieldLabel>
                    <Checkbox
                      id={`${outputId}-txt`}
                      aria-describedby={`${outputId}-txt-description`}
                      checked={taskDraft.options.writeTxt}
                      onCheckedChange={(checked) =>
                        onTaskDraftChange((current) => ({
                          ...current,
                          options: {
                            ...current.options,
                            writeTxt: checked === true,
                          },
                        }))
                      }
                    />
                  </Field>
                  <Field orientation="horizontal">
                    <FieldLabel htmlFor={`${outputId}-srt`}>
                      <FieldContent>
                        <FieldTitle>
                          SRT <Trans>字幕</Trans>
                        </FieldTitle>
                        <FieldDescription id={`${outputId}-srt-description`}>
                          <Trans>
                            建立字幕檔，首次使用時需下載約 1.8 GB 的對齊模型。
                          </Trans>
                        </FieldDescription>
                      </FieldContent>
                    </FieldLabel>
                    <Checkbox
                      id={`${outputId}-srt`}
                      aria-describedby={`${outputId}-srt-description`}
                      checked={taskDraft.options.writeSrt}
                      onCheckedChange={(checked) =>
                        onTaskDraftChange((current) => ({
                          ...current,
                          options: {
                            ...current.options,
                            writeSrt: checked === true,
                          },
                        }))
                      }
                    />
                  </Field>
                  <Field orientation="horizontal">
                    <FieldLabel htmlFor={`${outputId}-json`}>
                      <FieldContent>
                        <FieldTitle>
                          JSON <Trans>資料</Trans>
                        </FieldTitle>
                        <FieldDescription id={`${outputId}-json-description`}>
                          <Trans>
                            輸出含分段資料，支援語言會附逐字時間戳。
                          </Trans>
                        </FieldDescription>
                      </FieldContent>
                    </FieldLabel>
                    <Checkbox
                      id={`${outputId}-json`}
                      aria-describedby={`${outputId}-json-description`}
                      checked={taskDraft.options.writeJson}
                      onCheckedChange={(checked) =>
                        onTaskDraftChange((current) => ({
                          ...current,
                          options: {
                            ...current.options,
                            writeJson: checked === true,
                          },
                        }))
                      }
                    />
                  </Field>
                </FieldGroup>
              </FieldSet>

              <Field orientation="horizontal">
                <FieldContent>
                  <FieldTitle id={`${segmentId}-label`}>
                    <Trans>依標點符號斷句</Trans>
                  </FieldTitle>
                  <FieldDescription id={`${segmentId}-description`}>
                    <Trans>
                      自動依句號、逗號等標點符號斷句，適用於 SRT
                      字幕產生等情況。
                    </Trans>
                  </FieldDescription>
                </FieldContent>
                <Switch
                  aria-labelledby={`${segmentId}-label`}
                  aria-describedby={`${segmentId}-description`}
                  checked={taskDraft.options.segmentByPunctuation}
                  onCheckedChange={(checked) =>
                    onTaskDraftChange((current) => ({
                      ...current,
                      options: {
                        ...current.options,
                        segmentByPunctuation: checked,
                      },
                    }))
                  }
                />
              </Field>
            </FieldGroup>

            <Separator className="md:col-span-2" />

            <ScrollArea
              className="h-40 max-w-full min-w-0 rounded-lg border md:col-span-2"
              viewportClassName="overflow-x-hidden"
            >
              {taskDraft.files.map((file) => (
                <div
                  key={file}
                  className="flex w-full max-w-full min-w-0 items-center gap-2 overflow-hidden border-b p-2 text-sm last:border-b-0"
                >
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
              <Trans>取消</Trans>
            </Button>
            <Button disabled={!canConfirmTasks} onClick={onConfirmTaskDraft}>
              {isConfirmingTasks ? (
                <Spinner data-icon="inline-start" />
              ) : (
                <PlusIcon data-icon="inline-start" />
              )}
              {isConfirmingTasks ? (
                <Trans>新增中</Trans>
              ) : (
                <Trans>新增任務</Trans>
              )}
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
          className="w-[min(460px,calc(100vw-2rem))] max-w-[min(460px,calc(100vw-2rem))]"
          showCloseButton={!isConfirmingTasks && !isDownloading}
        >
          <DialogHeader>
            <DialogTitle>
              <Trans>下載模型</Trans>
            </DialogTitle>
            <DialogDescription>
              {draftModel ? (
                <Trans>
                  新增任務需要先下載 {draftModel.title}；完成後會自動加入佇列。
                </Trans>
              ) : (
                <Trans>新增任務需要先下載模型；完成後會自動加入佇列。</Trans>
              )}
            </DialogDescription>
          </DialogHeader>

          {modelDownloadError ? (
            <Alert variant="destructive">
              <TriangleAlertIcon />
              <AlertTitle>
                <Trans>模型下載失敗</Trans>
              </AlertTitle>
              <AlertDescription>{modelDownloadError}</AlertDescription>
            </Alert>
          ) : (
            <div className="flex min-w-0 flex-col gap-3.5 py-1">
              <Progress
                aria-label={t`模型下載整體進度`}
                className="gap-3"
                value={taskModelDownloadPercent}
              >
                <ProgressLabel className="min-w-0 flex-1 truncate font-normal text-muted-foreground">
                  <Trans>下載速度 {taskModelDownloadSpeed}</Trans>
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
              <Trans>回到任務設定</Trans>
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
              {modelDownloadError ? (
                <Trans>重新下載並加入佇列</Trans>
              ) : (
                <Trans>下載中</Trans>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

function TaskDetail({
  now,
  task,
  localizedLanguageItems,
}: {
  now: number;
  task: TranscriptionTask;
  localizedLanguageItems: SelectOption[];
}) {
  const { t } = useLingui();
  const { _ } = useLinguiRuntime();
  const progress = task.progress;
  const result = task.result;
  const partialSegments = progress?.partialSegments ?? [];
  const liveTranscriptViewportRef = useRef<HTMLDivElement>(null);
  const latestPartialSegment =
    partialSegments.length > 0
      ? partialSegments[partialSegments.length - 1]
      : null;
  const latestPartialSegmentFingerprint = latestPartialSegment
    ? `${latestPartialSegment.startMs}-${latestPartialSegment.text}-${latestPartialSegment.endMs}`
    : "";

  useEffect(() => {
    if (partialSegments.length === 0) return;
    const viewport = liveTranscriptViewportRef.current;
    if (!viewport) return;

    const animationFrameId = requestAnimationFrame(() => {
      viewport.scrollTop = viewport.scrollHeight;
    });

    return () => window.cancelAnimationFrame(animationFrameId);
  }, [partialSegments.length, latestPartialSegmentFingerprint]);

  if (task.status === "failed") {
    return (
      <Alert variant="destructive">
        <TriangleAlertIcon />
        <AlertTitle>
          <Trans>轉錄失敗</Trans>
        </AlertTitle>
        <AlertDescription>
          {task.error ?? <Trans>未知錯誤</Trans>}
        </AlertDescription>
      </Alert>
    );
  }

  if (task.status === "cancelled") {
    return (
      <Alert>
        <CircleStopIcon />
        <AlertTitle>
          <Trans>轉錄已終止</Trans>
        </AlertTitle>
        <AlertDescription>
          <Trans>此任務已停止，未產生新的轉錄結果。</Trans>
        </AlertDescription>
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
          <EmptyTitle>
            <Trans>等待處理</Trans>
          </EmptyTitle>
          <EmptyDescription>
            {task.modelTitle} ·{" "}
            {taskLanguageLabel(task, t, localizedLanguageItems)}
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }

  if (task.status === "running") {
    return (
      <div className="flex min-h-0 w-full min-w-0 flex-1 flex-col gap-3">
        <TranscriptionProgressPanel
          now={now}
          progress={progress}
          progressUpdatedAt={task.progressUpdatedAt}
        />
        <Separator />
        <section
          className="flex min-h-0 min-w-0 flex-1 flex-col gap-2.5"
          aria-labelledby="live-transcript-title"
        >
          <div className="flex min-w-0 items-start justify-between gap-3">
            <div className="min-w-0">
              <div id="live-transcript-title" className="text-sm font-medium">
                <Trans>即時逐字稿</Trans>
              </div>
            </div>
            <Badge variant="outline">
              <Trans>{partialSegments.length} 段</Trans>
            </Badge>
          </div>
          {partialSegments.length > 0 ? (
            <ScrollArea
              className="min-h-0 flex-1 overflow-auto scroll-auto"
              viewportClassName="overflow-x-hidden"
              viewportRef={liveTranscriptViewportRef}
            >
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>
                      <Trans>文字</Trans>
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {partialSegments.map((segment, index) => (
                    <TableRow key={`${segment.startMs}-${index}`}>
                      <TableCell className="align-top leading-relaxed break-words whitespace-pre-wrap">
                        {segment.text}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </ScrollArea>
          ) : (
            <div className="grid min-h-36 place-items-center text-sm text-muted-foreground">
              <Trans>辨識到語音後，逐字稿會顯示在這裡。</Trans>
            </div>
          )}
        </section>
      </div>
    );
  }

  if (!result) return null;

  const resultTimingRows = timingRows(result.timings, _);
  const detectedLanguage = result.detectedLanguage
    ? (localizedLanguageItems.find(
        (item) => item.value === result.detectedLanguage,
      )?.label ?? result.detectedLanguage)
    : t`未提供`;
  const languageSource =
    result.languageSource === "specified"
      ? t`指定語言`
      : result.languageSource === "detected"
        ? t`模型偵測`
        : t`未知來源`;
  const languageLabel =
    result.languageSource === "specified" ? t`轉錄語言` : t`偵測語言`;
  const alignmentStatus =
    result.alignmentStatus === "forced"
      ? t`精準對齊`
      : result.alignmentStatus === "partial"
        ? t`部分對齊`
        : t`估算時間`;
  const alignmentCoverage =
    typeof result.alignmentCoverage === "number"
      ? Math.max(0, Math.min(1, result.alignmentCoverage))
      : 0;
  const outputPaths = [
    ["TXT", result.txtPath],
    ["SRT", result.srtPath],
    ["JSON", result.jsonPath],
  ].filter(([, path]) => path) as [string, string][];

  return (
    <Tabs defaultValue="transcript" className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-w-0 items-center justify-between gap-3 text-sm max-[720px]:flex-col max-[720px]:items-start">
        <span className="text-muted-foreground">{languageLabel}</span>
        <span className="truncate" title={detectedLanguage}>
          {detectedLanguage} · {languageSource}
        </span>
      </div>
      <div className="flex min-w-0 items-center justify-between gap-3 text-sm max-[720px]:flex-col max-[720px]:items-start">
        <span className="text-muted-foreground">
          <Trans>時間對齊</Trans>
        </span>
        <span>
          {alignmentStatus} · {(alignmentCoverage * 100).toFixed(0)}%
        </span>
      </div>
      {outputPaths.length > 0 ? (
        <div className="flex min-w-0 flex-col gap-1 text-xs text-muted-foreground">
          <span>
            <Trans>輸出檔案</Trans>
          </span>
          {outputPaths.map(([format, path]) => (
            <div key={format} className="truncate" title={path}>
              {format}: {path}
            </div>
          ))}
        </div>
      ) : null}
      <TabsList className="w-full" variant="line">
        <TabsTrigger value="transcript">
          <Trans>逐字稿</Trans>
        </TabsTrigger>
        <TabsTrigger value="subtitles">
          <Trans>字幕</Trans>
        </TabsTrigger>
        <TabsTrigger value="stats">
          <Trans>統計</Trans>
        </TabsTrigger>
      </TabsList>

      <TabsContent value="transcript" className="min-h-0 flex-1">
        <ScrollArea className="min-h-0 flex-1">
          <div className="flex min-w-0 flex-col gap-3 p-0.5 pr-2">
            <div className="text-sm leading-7 break-words whitespace-pre-wrap">
              {result.text}
            </div>
          </div>
        </ScrollArea>
      </TabsContent>

      <TabsContent value="subtitles" className="min-h-0 flex-1">
        <ScrollArea className="min-h-0 flex-1">
          <div className="flex min-w-0 flex-col gap-3 p-0.5 pr-2">
            <div className="truncate text-sm text-muted-foreground">
              {result.srtPath ? (
                <Trans>SRT: {result.srtPath}</Trans>
              ) : (
                <Trans>未輸出 SRT</Trans>
              )}
            </div>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>
                    <Trans>時間</Trans>
                  </TableHead>
                  <TableHead>
                    <Trans>文字</Trans>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {result.segments.map((segment, index) => (
                  <TableRow key={`${segment.startMs}-${index}`}>
                    <TableCell className="whitespace-nowrap text-muted-foreground">
                      {formatTimestamp(segment.startMs)} -{" "}
                      {formatTimestamp(segment.endMs)}
                    </TableCell>
                    <TableCell className="leading-relaxed break-words whitespace-pre-wrap">
                      {segment.text}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        </ScrollArea>
      </TabsContent>

      <TabsContent value="stats" className="min-h-0 flex-1">
        <ScrollArea className="min-h-0 flex-1">
          <div className="flex min-w-0 flex-col gap-3 p-0.5 pr-2">
            <div className="flex items-center justify-between gap-3">
              <span className="text-sm font-medium">
                <Trans>總處理時間</Trans>
              </span>
              <Badge variant="outline">
                {formatDuration(result.durationMs)}
              </Badge>
            </div>
            <Table className="table-fixed">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-1/4">
                    <Trans>項目</Trans>
                  </TableHead>
                  <TableHead className="w-1/4 text-right">
                    <Trans>數值</Trans>
                  </TableHead>
                  <TableHead className="w-1/4">
                    <Trans>項目</Trans>
                  </TableHead>
                  <TableHead className="w-1/4 text-right">
                    <Trans>數值</Trans>
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {resultTimingRows.map(
                  ([leftLabel, leftValue, rightLabel, rightValue]) => (
                    <TableRow key={leftLabel}>
                      <TableCell className="max-w-0 truncate" title={leftLabel}>
                        {leftLabel}
                      </TableCell>
                      <TableCell className="max-w-0 truncate text-right font-mono text-muted-foreground tabular-nums">
                        {leftValue}
                      </TableCell>
                      <TableCell
                        className="max-w-0 truncate"
                        title={rightLabel}
                      >
                        {rightLabel}
                      </TableCell>
                      <TableCell className="max-w-0 truncate text-right font-mono text-muted-foreground tabular-nums">
                        {rightValue}
                      </TableCell>
                    </TableRow>
                  ),
                )}
              </TableBody>
            </Table>
          </div>
        </ScrollArea>
      </TabsContent>
    </Tabs>
  );
}
