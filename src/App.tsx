import { FileUpIcon, ListPlusIcon, Settings2Icon } from "lucide-react";
import { Toaster } from "sonner";

import { AppToolbar } from "@/components/app/AppToolbar";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { TooltipProvider } from "@/components/ui/tooltip";
import { SettingsPanel } from "@/components/transcription/SettingsPanel";
import { TaskManagerPanel } from "@/components/transcription/TaskManagerPanel";
import { useTranscriptionWorkspace } from "@/hooks/use-transcription-workspace";
import "./App.css";

function App() {
  const workspace = useTranscriptionWorkspace();
  const hasFinishedTasks = workspace.tasks.some(
    (task) => task.status === "completed" || task.status === "failed",
  );

  return (
    <TooltipProvider>
      <main className="app-shell">
        <Toaster richColors closeButton position="top-right" />
        <section className="app-window">
          {workspace.isDraggingFiles ? (
            <div
              className="file-drop-overlay"
              role="status"
              aria-live="polite"
            >
              <div className="file-drop-overlay-content">
                <div className="file-drop-overlay-icon">
                  <FileUpIcon />
                </div>
                <strong>把檔案拖到這裡</strong>
                <span>放開即可建立轉錄任務</span>
              </div>
            </div>
          ) : null}
          <AppToolbar
            ffmpeg={workspace.ffmpeg}
            title="QwenASR Studio"
            actions={workspace.tasks.length > 0 ? (
              <>
                {hasFinishedTasks ? (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={workspace.clearFinishedTasks}
                  >
                    清除已結束
                  </Button>
                ) : null}
                <Button size="sm" onClick={workspace.pickFilesForTasks}>
                  <ListPlusIcon data-icon="inline-start" />
                  新增任務
                </Button>
              </>
            ) : undefined}
            utilities={
              <Sheet>
                <SheetTrigger
                  render={
                    <Button variant="outline" size="sm" />
                  }
                >
                  <Settings2Icon data-icon="inline-start" />
                  設定
                </SheetTrigger>
                <SheetContent
                  side="right"
                  className="settings-sheet data-[side=right]:w-[min(560px,100vw)] data-[side=right]:sm:max-w-[min(560px,100vw)]"
                >
                  <SheetHeader>
                    <SheetTitle>設定</SheetTitle>
                    <SheetDescription>管理模型與相關工具。</SheetDescription>
                  </SheetHeader>
                  <div className="settings-sheet-body">
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
                  </div>
                </SheetContent>
              </Sheet>
            }
          />
          <div className="main-content is-task-view">
            <TaskManagerPanel
              tasks={workspace.tasks}
              models={workspace.transcriptionModels}
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
          </div>
        </section>
      </main>
    </TooltipProvider>
  );
}

export default App;
