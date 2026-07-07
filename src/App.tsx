import { useState } from "react";
import type { CSSProperties } from "react";
import { Toaster } from "sonner";

import { AppToolbar } from "@/components/app/AppToolbar";
import { WorkspaceSidebar } from "@/components/app/WorkspaceSidebar";
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { BatchTranscriptionPanel } from "@/components/transcription/BatchTranscriptionPanel";
import { ResultsPanel } from "@/components/transcription/ResultsPanel";
import { SettingsPanel } from "@/components/transcription/SettingsPanel";
import { SingleTranscriptionPanel } from "@/components/transcription/SingleTranscriptionPanel";
import { useTranscriptionWorkspace } from "@/hooks/use-transcription-workspace";
import type { WorkspaceView } from "@/types/transcription";
import "./App.css";

const viewCopy: Record<WorkspaceView, { title: string; subtitle: string }> = {
  transcribe: {
    title: "轉錄工作台",
    subtitle: "選檔、設定輸出，完成後取得文字與 SRT 字幕。",
  },
  batch: {
    title: "批次轉錄",
    subtitle: "將多個檔案排入佇列，逐一轉錄並輸出字幕。",
  },
  settings: {
    title: "設定",
    subtitle: "管理 QwenASR 模型下載器與 FFmpeg 轉檔工具。",
  },
};

function App() {
  const workspace = useTranscriptionWorkspace();
  const [activeView, setActiveView] = useState<WorkspaceView>("transcribe");
  const copy = viewCopy[activeView];
  const modelLabel = workspace.selectedModel?.title ?? "未選擇";

  return (
    <TooltipProvider>
      <main className="app-shell">
        <Toaster richColors closeButton position="top-right" />
        <section className="app-window">
          <SidebarProvider
            style={
              {
                "--sidebar-width": "15rem",
                "--sidebar-width-icon": "3.25rem",
              } as CSSProperties
            }
          >
            <WorkspaceSidebar
              activeView={activeView}
              onViewChange={setActiveView}
            />
            <SidebarInset className="app-main-inset">
              <AppToolbar
                ffmpeg={workspace.ffmpeg}
                title={copy.title}
                subtitle={copy.subtitle}
                modelReady={workspace.hasReadyModel}
                modelLabel={modelLabel}
                onOpenSettings={() => setActiveView("settings")}
                onRefresh={workspace.refreshRuntime}
              />
              <div className="main-content">
                {activeView === "transcribe" ? (
                  <div className="operation-grid">
                    <SingleTranscriptionPanel
                      singleFile={workspace.singleFile}
                      outputDir={workspace.outputDir}
                      options={workspace.options}
                      isTranscribing={workspace.isTranscribing}
                      canRun={workspace.canRunSingle}
                      onPickFile={workspace.pickSingleFile}
                      onPickOutputDir={workspace.pickOutputDir}
                      onOptionsChange={workspace.setOptions}
                      onRun={workspace.runSingleTranscription}
                    />
                    <ResultsPanel
                      results={workspace.results}
                      isTranscribing={workspace.isTranscribing}
                      progress={workspace.transcriptionProgress}
                    />
                  </div>
                ) : null}
                {activeView === "batch" ? (
                  <div className="operation-grid">
                    <BatchTranscriptionPanel
                      files={workspace.batchFiles}
                      outputDir={workspace.outputDir}
                      options={workspace.options}
                      isTranscribing={workspace.isTranscribing}
                      canRun={workspace.canRunBatch}
                      onPickFiles={workspace.pickBatchFiles}
                      onPickOutputDir={workspace.pickOutputDir}
                      onRemoveFile={workspace.removeBatchFile}
                      onClearFiles={workspace.clearBatchFiles}
                      onOptionsChange={workspace.setOptions}
                      onRun={workspace.runBatchTranscription}
                    />
                    <ResultsPanel
                      results={workspace.results}
                      isTranscribing={workspace.isTranscribing}
                      progress={workspace.transcriptionProgress}
                    />
                  </div>
                ) : null}
                {activeView === "settings" ? (
                  <SettingsPanel
                    models={workspace.models}
                    selectedModelId={workspace.selectedModelId}
                    downloadProgress={workspace.downloadProgress}
                    isDownloading={workspace.isDownloading}
                    deletingModelId={workspace.deletingModelId}
                    isTranscribing={workspace.isTranscribing}
                    ffmpeg={workspace.ffmpeg}
                    onSelectModel={workspace.setSelectedModelId}
                    onDownload={workspace.downloadSelectedModel}
                    onDeleteModel={workspace.deleteModel}
                    onRefresh={workspace.refreshRuntime}
                  />
                ) : null}
              </div>
            </SidebarInset>
          </SidebarProvider>
        </section>
      </main>
    </TooltipProvider>
  );
}

export default App;
