import { useState } from "react";
import type { CSSProperties } from "react";
import { ListPlusIcon } from "lucide-react";
import { Toaster } from "sonner";

import { AppToolbar } from "@/components/app/AppToolbar";
import { WorkspaceSidebar } from "@/components/app/WorkspaceSidebar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsPanel } from "@/components/transcription/SettingsPanel";
import { TaskManagerPanel } from "@/components/transcription/TaskManagerPanel";
import { useTranscriptionWorkspace } from "@/hooks/use-transcription-workspace";
import type { WorkspaceView } from "@/types/transcription";
import "./App.css";

const viewCopy: Record<WorkspaceView, { title: string; subtitle: string }> = {
  tasks: {
    title: "轉錄任務",
    subtitle: "在這裡管理你的轉錄任務。",
  },
  settings: {
    title: "設定",
    subtitle: "管理模型與相關工具。",
  },
};

function App() {
  const workspace = useTranscriptionWorkspace();
  const [activeView, setActiveView] = useState<WorkspaceView>("tasks");
  const copy = viewCopy[activeView];
  const hasFinishedTasks = workspace.completedCount + workspace.failedCount > 0;

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
                actions={
                  activeView === "tasks" ? (
                    <>
                      <Badge variant="outline">
                        {workspace.queuedCount} 排隊
                      </Badge>
                      <Badge variant="secondary">
                        {workspace.completedCount} 完成
                      </Badge>
                      {workspace.failedCount > 0 ? (
                        <Badge variant="destructive">
                          {workspace.failedCount} 失敗
                        </Badge>
                      ) : null}
                      <Button size="sm" onClick={workspace.pickFilesForTasks}>
                        <ListPlusIcon data-icon="inline-start" />
                        新增任務
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={!hasFinishedTasks}
                        onClick={workspace.clearFinishedTasks}
                      >
                        清除已結束
                      </Button>
                    </>
                  ) : null
                }
              />
              <div className="main-content">
                {activeView === "tasks" ? (
                  <TaskManagerPanel
                    tasks={workspace.tasks}
                    models={workspace.models}
                    taskDraft={workspace.taskDraft}
                    draftModel={workspace.draftModel}
                    canConfirmTasks={workspace.canConfirmTasks}
                    isConfirmingTasks={workspace.isConfirmingTasks}
                    isDownloading={workspace.isDownloading}
                    isTaskDialogOpen={workspace.isTaskDialogOpen}
                    isModelDownloadDialogOpen={
                      workspace.isTaskModelDownloadDialogOpen
                    }
                    modelDownloadError={workspace.taskModelDownloadError}
                    downloadProgress={workspace.downloadProgress}
                    downloadMovingAverageSpeedBytesPerSec={
                      workspace.downloadMovingAverageSpeedBytesPerSec
                    }
                    isDraggingFiles={workspace.isDraggingFiles}
                    onPickFiles={workspace.pickFilesForTasks}
                    onPickOutputDir={workspace.pickTaskOutputDir}
                    onTaskDraftChange={workspace.setTaskDraft}
                    onTaskDialogOpenChange={workspace.setTaskDialogOpen}
                    onModelDownloadDialogOpenChange={
                      workspace.setTaskModelDownloadDialogOpen
                    }
                    onConfirmTaskDraft={workspace.confirmTaskDraft}
                    onRemoveTask={workspace.removeTask}
                    onRetryTask={workspace.retryTask}
                  />
                ) : null}
                {activeView === "settings" ? (
                  <SettingsPanel
                    models={workspace.models}
                    downloadProgress={workspace.downloadProgress}
                    downloadMovingAverageSpeedBytesPerSec={
                      workspace.downloadMovingAverageSpeedBytesPerSec
                    }
                    isDownloading={workspace.isDownloading}
                    deletingModelId={workspace.deletingModelId}
                    isTranscribing={workspace.isTranscribing}
                    ffmpeg={workspace.ffmpeg}
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
