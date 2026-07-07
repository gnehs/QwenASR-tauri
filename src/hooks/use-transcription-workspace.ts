import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";

import { audioFilters, defaultOptions } from "@/lib/app-constants";
import { formatInvokeError, uniquePaths } from "@/lib/format";
import type {
  DownloadProgress,
  FfmpegStatus,
  ModelStatus,
  OptionsState,
  TranscriptionProgress,
  TranscriptionResult,
} from "@/types/transcription";

const selectedModelStorageKey = "qwenasr:selected-model-id";

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

export function useTranscriptionWorkspace() {
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [ffmpeg, setFfmpeg] = useState<FfmpegStatus>({
    available: false,
    version: null,
  });
  const [selectedModelId, setSelectedModelId] = useState(
    readStoredSelectedModelId,
  );
  const [singleFile, setSingleFile] = useState("");
  const [batchFiles, setBatchFiles] = useState<string[]>([]);
  const [outputDir, setOutputDir] = useState("");
  const [options, setOptions] = useState<OptionsState>(defaultOptions);
  const [downloadProgress, setDownloadProgress] =
    useState<DownloadProgress | null>(null);
  const [transcriptionProgress, setTranscriptionProgress] =
    useState<TranscriptionProgress | null>(null);
  const [results, setResults] = useState<TranscriptionResult[]>([]);
  const [isDownloading, setIsDownloading] = useState(false);
  const [deletingModelId, setDeletingModelId] = useState<string | null>(null);
  const [isTranscribing, setIsTranscribing] = useState(false);

  useEffect(() => {
    refreshRuntime();

    const unlisteners = Promise.all([
      listen<DownloadProgress>("model-download-progress", (event) => {
        setDownloadProgress(event.payload);
        if (event.payload.state === "complete") {
          setIsDownloading(false);
          refreshModels();
        }
      }),
      listen<TranscriptionProgress>("transcription-progress", (event) => {
        setTranscriptionProgress(event.payload);
      }),
    ]);

    return () => {
      unlisteners.then((items) => items.forEach((unlisten) => unlisten()));
    };
  }, []);

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

  const selectedModel = models.find((model) => model.id === selectedModelId);
  const hasReadyModel = Boolean(selectedModel?.installed);
  const canRunSingle = Boolean(singleFile && hasReadyModel && !isTranscribing);
  const canRunBatch = batchFiles.length > 0 && hasReadyModel && !isTranscribing;

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

  async function pickSingleFile() {
    const selected = await open({
      multiple: false,
      filters: audioFilters,
    });
    if (typeof selected === "string") {
      setSingleFile(selected);
    }
  }

  async function pickBatchFiles() {
    const selected = await open({
      multiple: true,
      filters: audioFilters,
    });
    if (Array.isArray(selected)) {
      setBatchFiles((current) => uniquePaths([...current, ...selected]));
    }
  }

  async function pickOutputDir() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (typeof selected === "string") {
      setOutputDir(selected);
    }
  }

  async function downloadSelectedModel(modelId = selectedModelId) {
    if (!modelId) return;

    setIsDownloading(true);
    setDownloadProgress(null);
    try {
      await invoke<ModelStatus>("download_model", { modelId });
      await refreshModels();
      toast.success("模型已可使用");
    } catch (error) {
      setIsDownloading(false);
      toast.error(formatInvokeError(error));
    }
  }

  async function deleteModel(modelId: string) {
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
      toast.success("模型已刪除");
      return true;
    } catch (error) {
      toast.error(formatInvokeError(error));
      return false;
    } finally {
      setDeletingModelId(null);
    }
  }

  async function runSingleTranscription() {
    if (!singleFile) return;

    setIsTranscribing(true);
    setResults([]);
    setTranscriptionProgress(null);
    try {
      const result = await invoke<TranscriptionResult>("transcribe_file", {
        request: {
          audioPath: singleFile,
          options: buildOptionsPayload(),
        },
      });
      setResults([result]);
      toast.success("轉錄完成");
    } catch (error) {
      toast.error(formatInvokeError(error));
    } finally {
      setIsTranscribing(false);
    }
  }

  async function runBatchTranscription() {
    if (batchFiles.length === 0) return;

    setIsTranscribing(true);
    setResults([]);
    setTranscriptionProgress(null);
    try {
      const batchResults = await invoke<TranscriptionResult[]>(
        "transcribe_batch",
        {
          request: {
            audioPaths: batchFiles,
            options: buildOptionsPayload(),
          },
        },
      );
      setResults(batchResults);
      toast.success("批次轉錄完成");
    } catch (error) {
      toast.error(formatInvokeError(error));
    } finally {
      setIsTranscribing(false);
    }
  }

  function removeBatchFile(path: string) {
    setBatchFiles((current) => current.filter((item) => item !== path));
  }

  function buildOptionsPayload() {
    return {
      modelId: selectedModelId,
      language: options.language,
      writeSrt: options.writeSrt,
      outputDir: outputDir || null,
    };
  }

  return {
    models,
    ffmpeg,
    selectedModel,
    selectedModelId,
    singleFile,
    batchFiles,
    outputDir,
    options,
    downloadProgress,
    transcriptionProgress,
    results,
    isDownloading,
    deletingModelId,
    isTranscribing,
    hasReadyModel,
    canRunSingle,
    canRunBatch,
    setSelectedModelId,
    setOptions,
    pickSingleFile,
    pickBatchFiles,
    pickOutputDir,
    downloadSelectedModel,
    deleteModel,
    runSingleTranscription,
    runBatchTranscription,
    removeBatchFile,
    clearBatchFiles: () => setBatchFiles([]),
    refreshRuntime,
  };
}
