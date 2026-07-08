import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { toast } from "sonner";

import { audioFilters, defaultOptions } from "@/lib/app-constants";
import { basename, formatInvokeError, uniquePaths } from "@/lib/format";
import type {
  DownloadProgress,
  FfmpegStatus,
  ModelStatus,
  OptionsState,
  TaskDraft,
  TranscriptionProgress,
  TranscriptionResult,
  TranscriptionTask,
} from "@/types/transcription";

const selectedModelStorageKey = "qwenasr:selected-model-id";
const supportedExtensions = new Set(
  audioFilters.flatMap((filter) =>
    filter.extensions.map((extension) => extension.toLowerCase()),
  ),
);
let notificationPermissionRequested = false;
const downloadSpeedWindowMs = 5000;

type DownloadSpeedSample = {
  timestamp: number;
  completedBytes: number;
};

type DownloadSpeedMovingAverageTracker = {
  modelId: string;
  currentFile: string | null;
  completedBeforeCurrentFile: number;
  currentFileCompleted: number;
  samples: DownloadSpeedSample[];
};

function readStoredSelectedModelId() {
  try {
    return window.localStorage.getItem(selectedModelStorageKey) ?? "";
  } catch {
    return "";
  }
}

function writeStoredSelectedModelId(modelId: string) {
  try {
    window.localStorage.setItem(selectedModelStorageKey, modelId);
  } catch {
    // Ignore storage failures so model selection still works in restricted webviews.
  }
}

function createTaskId() {
  return window.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

function isSupportedMediaPath(path: string) {
  const extension = path.split(".").pop()?.toLowerCase();
  return Boolean(extension && supportedExtensions.has(extension));
}

function pathsFromDialogSelection(selected: string | string[] | null) {
  if (typeof selected === "string") return [selected];
  if (Array.isArray(selected)) return selected;
  return [];
}

function buildOptionsPayload(task: TranscriptionTask) {
  return {
    modelId: task.modelId,
    language: task.options.language,
    writeSrt: task.options.writeSrt,
    outputDir: task.outputDir || null,
  };
}

function movingAverageDownloadSpeed(
  progress: DownloadProgress,
  now: number,
  previous: DownloadSpeedMovingAverageTracker | null,
) {
  let tracker = previous;
  const completedBytes = Math.max(0, progress.fileBytesCompleted);

  if (
    !tracker ||
    tracker.modelId !== progress.modelId ||
    progress.state === "starting"
  ) {
    tracker = {
      modelId: progress.modelId,
      currentFile: progress.currentFile,
      completedBeforeCurrentFile: 0,
      currentFileCompleted: completedBytes,
      samples: [],
    };
  } else {
    const nextFile = progress.currentFile;

    if (nextFile && tracker.currentFile && nextFile !== tracker.currentFile) {
      tracker = {
        ...tracker,
        currentFile: nextFile,
        completedBeforeCurrentFile:
          tracker.completedBeforeCurrentFile + tracker.currentFileCompleted,
        currentFileCompleted: completedBytes,
      };
    } else {
      tracker = {
        ...tracker,
        currentFile: tracker.currentFile ?? nextFile,
        currentFileCompleted: Math.max(
          tracker.currentFileCompleted,
          completedBytes,
        ),
      };
    }
  }

  const totalCompletedBytes =
    tracker.completedBeforeCurrentFile + tracker.currentFileCompleted;
  const windowStart = now - downloadSpeedWindowMs;
  const samples = [
    ...tracker.samples.filter((sample) => sample.timestamp >= windowStart),
    { timestamp: now, completedBytes: totalCompletedBytes },
  ];
  const firstSample = samples[0];
  const lastSample = samples[samples.length - 1];
  const elapsedSeconds =
    firstSample && lastSample
      ? Math.max((lastSample.timestamp - firstSample.timestamp) / 1000, 0)
      : 0;
  const speedBytesPerSec =
    elapsedSeconds > 0
      ? Math.max(
          0,
          (lastSample.completedBytes - firstSample.completedBytes) /
            elapsedSeconds,
        )
      : 0;

  return { tracker: { ...tracker, samples }, speedBytesPerSec };
}

async function sendTaskNotification(
  task: TranscriptionTask,
  result: TranscriptionResult,
) {
  try {
    let permissionGranted = await isPermissionGranted();

    if (!permissionGranted) {
      if (notificationPermissionRequested) return;

      notificationPermissionRequested = true;
      const permission = await requestPermission();
      permissionGranted = permission === "granted";
    }

    if (permissionGranted) {
      sendNotification({
        title: "轉錄完成",
        body: `${basename(task.audioPath)} 已完成${
          result.srtPath ? "，SRT 已輸出" : ""
        }`,
      });
    }
  } catch {
    // Notification failure should not affect transcription state.
  }
}

export function useTranscriptionWorkspace() {
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [ffmpeg, setFfmpeg] = useState<FfmpegStatus>({
    available: false,
    version: null,
  });
  const [selectedModelId, setSelectedModelId] = useState(
    readStoredSelectedModelId,
  );
  const [outputDir, setOutputDir] = useState("");
  const [options, setOptions] = useState<OptionsState>(defaultOptions);
  const [taskDraft, setTaskDraft] = useState<TaskDraft>({
    files: [],
    modelId: "",
    outputDir: "",
    options: defaultOptions,
  });
  const [isTaskDialogOpen, setTaskDialogOpen] = useState(false);
  const [isDraggingFiles, setIsDraggingFiles] = useState(false);
  const [tasks, setTasks] = useState<TranscriptionTask[]>([]);
  const [downloadProgress, setDownloadProgress] =
    useState<DownloadProgress | null>(null);
  const [
    downloadMovingAverageSpeedBytesPerSec,
    setDownloadMovingAverageSpeedBytesPerSec,
  ] = useState(0);
  const [transcriptionProgress, setTranscriptionProgress] =
    useState<TranscriptionProgress | null>(null);
  const [isDownloading, setIsDownloading] = useState(false);
  const [isConfirmingTasks, setIsConfirmingTasks] = useState(false);
  const [isTaskModelDownloadDialogOpen, setTaskModelDownloadDialogOpen] =
    useState(false);
  const [taskModelDownloadError, setTaskModelDownloadError] = useState<
    string | null
  >(null);
  const [deletingModelId, setDeletingModelId] = useState<string | null>(null);
  const runningTaskIdRef = useRef<string | null>(null);
  const downloadSpeedMovingAverageRef =
    useRef<DownloadSpeedMovingAverageTracker | null>(null);

  const selectedModel = models.find((model) => model.id === selectedModelId);
  const draftModel = models.find((model) => model.id === taskDraft.modelId);
  const hasReadyModel = Boolean(selectedModel?.installed);
  const runningTask = tasks.find((task) => task.status === "running") ?? null;
  const isTranscribing = Boolean(runningTask);
  const queuedCount = tasks.filter((task) => task.status === "queued").length;
  const completedCount = tasks.filter(
    (task) => task.status === "completed",
  ).length;
  const failedCount = tasks.filter((task) => task.status === "failed").length;
  const canConfirmTasks =
    taskDraft.files.length > 0 &&
    Boolean(draftModel) &&
    !isConfirmingTasks &&
    (Boolean(draftModel?.installed) || !isDownloading);

  const defaultModelId = useCallback(() => {
    const installedSelected = models.find(
      (model) => model.id === selectedModelId && model.installed,
    );
    if (installedSelected) return installedSelected.id;

    return (
      models.find((model) => model.recommended && model.installed)?.id ??
      models.find((model) => model.installed)?.id ??
      selectedModelId ??
      ""
    );
  }, [models, selectedModelId]);

  const openTaskDialog = useCallback(
    (paths: string[]) => {
      const acceptedPaths = uniquePaths(paths.filter(isSupportedMediaPath));
      const rejectedCount = paths.length - acceptedPaths.length;

      if (acceptedPaths.length === 0) {
        toast.error("沒有可加入的音訊或影片檔");
        return;
      }

      if (rejectedCount > 0) {
        toast.warning(`已略過 ${rejectedCount} 個不支援的檔案`);
      }

      setTaskDraft({
        files: acceptedPaths,
        modelId: defaultModelId(),
        outputDir,
        options,
      });
      setTaskDialogOpen(true);
    },
    [defaultModelId, options, outputDir],
  );

  const runQueuedTask = useCallback(async (task: TranscriptionTask) => {
    runningTaskIdRef.current = task.id;
    setTranscriptionProgress(null);
    setTasks((current) =>
      current.map((item) =>
        item.id === task.id
          ? {
              ...item,
              status: "running",
              error: null,
              progress: null,
              progressUpdatedAt: null,
              result: null,
              startedAt: Date.now(),
              completedAt: null,
            }
          : item,
      ),
    );

    try {
      const result = await invoke<TranscriptionResult>("transcribe_file", {
        request: {
          audioPath: task.audioPath,
          options: buildOptionsPayload(task),
        },
      });

      setTasks((current) =>
        current.map((item) =>
          item.id === task.id
            ? {
                ...item,
                status: "completed",
                progress: null,
                progressUpdatedAt: null,
                result,
                completedAt: Date.now(),
              }
            : item,
        ),
      );
      toast.success(`${basename(task.audioPath)} 轉錄完成`);
      void sendTaskNotification(task, result);
    } catch (error) {
      const message = formatInvokeError(error);
      setTasks((current) =>
        current.map((item) =>
          item.id === task.id
            ? {
                ...item,
                status: "failed",
                error: message,
                progressUpdatedAt: null,
                completedAt: Date.now(),
              }
            : item,
        ),
      );
      toast.error(`${basename(task.audioPath)} 轉錄失敗`, {
        description: message,
      });
    } finally {
      runningTaskIdRef.current = null;
      setTranscriptionProgress(null);
    }
  }, []);

  useEffect(() => {
    refreshRuntime();

    const unlisteners = Promise.all([
      listen<DownloadProgress>("model-download-progress", (event) => {
        const averaged = movingAverageDownloadSpeed(
          event.payload,
          Date.now(),
          downloadSpeedMovingAverageRef.current,
        );
        downloadSpeedMovingAverageRef.current = averaged.tracker;
        setDownloadMovingAverageSpeedBytesPerSec(averaged.speedBytesPerSec);
        setDownloadProgress(event.payload);
        if (event.payload.state === "complete") {
          setIsDownloading(false);
          refreshModels();
        }
      }),
      listen<TranscriptionProgress>("transcription-progress", (event) => {
        const runningTaskId = runningTaskIdRef.current;
        const progressUpdatedAt = Date.now();

        setTranscriptionProgress(event.payload);
        if (runningTaskId) {
          setTasks((current) =>
            current.map((task) =>
              task.id === runningTaskId
                ? { ...task, progress: event.payload, progressUpdatedAt }
                : task,
            ),
          );
        }
      }),
    ]);

    return () => {
      unlisteners.then((items) => items.forEach((unlisten) => unlisten()));
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          setIsDraggingFiles(true);
          return;
        }

        setIsDraggingFiles(false);
        if (event.payload.type === "drop") {
          openTaskDialog(event.payload.paths);
        }
      })
      .then((handler) => {
        if (active) {
          unlisten = handler;
        } else {
          handler();
        }
      })
      .catch(() => {
        setIsDraggingFiles(false);
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [openTaskDialog]);

  useEffect(() => {
    if (models.length === 0) return;

    const hasSelectedModel = models.some(
      (model) => model.id === selectedModelId,
    );

    if (!hasSelectedModel) {
      setSelectedModelId(
        models.find((model) => model.recommended)?.id ?? models[0].id,
      );
    }
  }, [models, selectedModelId]);

  useEffect(() => {
    if (models.length === 0 || !selectedModelId) return;

    const hasSelectedModel = models.some(
      (model) => model.id === selectedModelId,
    );

    if (hasSelectedModel) {
      writeStoredSelectedModelId(selectedModelId);
    }
  }, [models, selectedModelId]);

  useEffect(() => {
    if (!isTaskDialogOpen || taskDraft.modelId) return;

    setTaskDraft((current) => ({
      ...current,
      modelId: defaultModelId(),
    }));
  }, [defaultModelId, isTaskDialogOpen, taskDraft.modelId]);

  useEffect(() => {
    if (runningTaskIdRef.current) return;

    const nextTask = tasks.find((task) => task.status === "queued");
    if (nextTask) {
      void runQueuedTask(nextTask);
    }
  }, [runQueuedTask, tasks]);

  async function refreshRuntime() {
    await Promise.all([refreshModels(), refreshFfmpeg()]);
  }

  async function refreshModels() {
    try {
      setModels(await invoke<ModelStatus[]>("list_available_models"));
    } catch (error) {
      toast.error(formatInvokeError(error));
    }
  }

  async function refreshFfmpeg() {
    try {
      setFfmpeg(await invoke<FfmpegStatus>("get_ffmpeg_status"));
    } catch (error) {
      toast.error(formatInvokeError(error));
    }
  }

  async function pickFilesForTasks() {
    const selected = await open({
      multiple: true,
      filters: audioFilters,
    });
    const selectedPaths = pathsFromDialogSelection(selected);
    if (selectedPaths.length > 0) {
      openTaskDialog(selectedPaths);
    }
  }

  async function pickTaskOutputDir() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (typeof selected === "string") {
      setTaskDraft((current) => ({ ...current, outputDir: selected }));
    }
  }

  async function downloadModel(modelId: string) {
    setIsDownloading(true);
    setDownloadProgress(null);
    setDownloadMovingAverageSpeedBytesPerSec(0);
    downloadSpeedMovingAverageRef.current = null;
    try {
      const downloadedModel = await invoke<ModelStatus>("download_model", {
        modelId,
      });
      setModels((current) =>
        current.map((model) =>
          model.id === downloadedModel.id ? downloadedModel : model,
        ),
      );
      await refreshModels();
      setIsDownloading(false);
      toast.success("模型已可使用");
      return downloadedModel;
    } catch (error) {
      setIsDownloading(false);
      const message = formatInvokeError(error);
      toast.error(message);
      throw new Error(message);
    }
  }

  async function downloadSelectedModel(modelId = selectedModelId) {
    if (!modelId) return;

    try {
      await downloadModel(modelId);
    } catch {
      // Error toast is emitted by downloadModel.
    }
  }

  async function deleteModel(modelId: string) {
    const modelInQueue = tasks.some(
      (task) =>
        task.modelId === modelId &&
        (task.status === "queued" || task.status === "running"),
    );

    if (modelInQueue) {
      toast.error("模型正在任務佇列中使用");
      return false;
    }

    if (!modelId || isDownloading || isTranscribing || deletingModelId) {
      return false;
    }

    setDeletingModelId(modelId);
    try {
      const deletedModel = await invoke<ModelStatus>("delete_model", {
        modelId,
      });
      setModels((current) =>
        current.map((model) =>
          model.id === deletedModel.id ? deletedModel : model,
        ),
      );
      await refreshModels();
      setDownloadProgress(null);
      setDownloadMovingAverageSpeedBytesPerSec(0);
      downloadSpeedMovingAverageRef.current = null;
      toast.success("模型已刪除");
      return true;
    } catch (error) {
      toast.error(formatInvokeError(error));
      return false;
    } finally {
      setDeletingModelId(null);
    }
  }

  async function confirmTaskDraft() {
    if (!canConfirmTasks || !draftModel) return;

    const needsDownload = !draftModel.installed;
    setIsConfirmingTasks(true);
    setTaskModelDownloadError(null);
    if (needsDownload) {
      setTaskModelDownloadDialogOpen(true);
    }

    try {
      let readyModel = draftModel;

      if (needsDownload) {
        readyModel = await downloadModel(draftModel.id);
      }

      const createdAt = Date.now();
      const nextTasks: TranscriptionTask[] = taskDraft.files.map(
        (audioPath, index) => ({
          id: createTaskId(),
          audioPath,
          modelId: readyModel.id,
          modelTitle: readyModel.title,
          outputDir: taskDraft.outputDir,
          options: taskDraft.options,
          status: "queued",
          progress: null,
          progressUpdatedAt: null,
          result: null,
          error: null,
          createdAt: createdAt + index,
          startedAt: null,
          completedAt: null,
        }),
      );

      setTasks((current) => [...current, ...nextTasks]);
      setSelectedModelId(readyModel.id);
      setOptions(taskDraft.options);
      setOutputDir(taskDraft.outputDir);
      setTaskDialogOpen(false);
      setTaskModelDownloadDialogOpen(false);
      toast.success(`已加入 ${nextTasks.length} 個轉錄任務`);
    } catch (error) {
      setTaskModelDownloadError(
        error instanceof Error ? error.message : formatInvokeError(error),
      );
      // Error toast is emitted by downloadModel.
    } finally {
      setIsConfirmingTasks(false);
    }
  }

  function removeTask(taskId: string) {
    const task = tasks.find((item) => item.id === taskId);
    if (task?.status === "running") {
      toast.error("執行中的任務無法移除");
      return;
    }

    setTasks((current) => current.filter((item) => item.id !== taskId));
  }

  function retryTask(taskId: string) {
    setTasks((current) =>
      current.map((task) =>
        task.id === taskId && task.status !== "running"
          ? {
              ...task,
              status: "queued",
              progress: null,
              progressUpdatedAt: null,
              result: null,
              error: null,
              startedAt: null,
              completedAt: null,
            }
          : task,
      ),
    );
  }

  function clearFinishedTasks() {
    setTasks((current) =>
      current.filter(
        (task) => task.status === "queued" || task.status === "running",
      ),
    );
  }

  return {
    models,
    ffmpeg,
    selectedModel,
    selectedModelId,
    taskDraft,
    draftModel,
    tasks,
    outputDir,
    options,
    downloadProgress,
    downloadMovingAverageSpeedBytesPerSec,
    transcriptionProgress,
    isDownloading,
    isConfirmingTasks,
    isTaskModelDownloadDialogOpen,
    taskModelDownloadError,
    deletingModelId,
    isTranscribing,
    isTaskDialogOpen,
    isDraggingFiles,
    runningTask,
    queuedCount,
    completedCount,
    failedCount,
    hasReadyModel,
    canConfirmTasks,
    setSelectedModelId,
    setTaskDraft,
    setTaskDialogOpen,
    setTaskModelDownloadDialogOpen,
    pickFilesForTasks,
    pickTaskOutputDir,
    confirmTaskDraft,
    removeTask,
    retryTask,
    clearFinishedTasks,
    downloadSelectedModel,
    deleteModel,
    refreshRuntime,
  };
}
