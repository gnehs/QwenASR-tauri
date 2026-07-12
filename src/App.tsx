import {
  FileUpIcon,
  ListPlusIcon,
  Settings2Icon,
  Trash2Icon,
} from "lucide-react";
import { Trans } from "@lingui/react/macro";
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

function App() {
  const workspace = useTranscriptionWorkspace();
  const hasFinishedTasks = workspace.tasks.some(
    (task) =>
      task.status === "completed" ||
      task.status === "failed" ||
      task.status === "cancelled",
  );

  return (
    <TooltipProvider>
      <main className="isolate min-h-screen bg-background text-foreground">
        <Toaster richColors closeButton position="bottom-right" />
        <section className="flex h-screen min-h-0 flex-col overflow-hidden bg-background">
          {workspace.isDraggingFiles ? (
            <div
              className="pointer-events-none fixed inset-3 z-50 grid place-items-center rounded-xl border-2 border-dashed border-primary/40 bg-background ring-1 ring-primary/20"
              role="status"
              aria-live="polite"
            >
              <div className="flex flex-col items-center gap-2 text-center">
                <div className="mb-2 grid size-16 place-items-center rounded-lg bg-primary/10 text-primary">
                  <FileUpIcon className="size-8" />
                </div>
                <strong className="font-sans text-2xl leading-tight font-semibold">
                  <Trans>把檔案拖到這裡</Trans>
                </strong>
                <span className="text-sm text-muted-foreground">
                  <Trans>放開即可建立轉錄任務</Trans>
                </span>
              </div>
            </div>
          ) : null}
          <AppToolbar
            ffmpeg={workspace.ffmpeg}
            title="QwenASR Studio"
            actions={
              workspace.tasks.length > 0 ? (
                <>
                  {hasFinishedTasks ? (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={workspace.clearFinishedTasks}
                    >
                      <Trash2Icon data-icon="inline-start" />
                      <Trans>清除已完成</Trans>
                    </Button>
                  ) : null}
                  <Button size="sm" onClick={workspace.pickFilesForTasks}>
                    <ListPlusIcon data-icon="inline-start" />
                    <Trans>新增任務</Trans>
                  </Button>
                </>
              ) : undefined
            }
            utilities={
              <Sheet>
                <SheetTrigger render={<Button variant="outline" size="sm" />}>
                  <Settings2Icon data-icon="inline-start" />
                  <Trans>設定</Trans>
                </SheetTrigger>
                <SheetContent
                  side="right"
                  className="data-[side=right]:w-[min(560px,100vw)] data-[side=right]:sm:max-w-[min(560px,100vw)]"
                >
                  <SheetHeader>
                    <SheetTitle className="text-lg">
                      <Trans>設定</Trans>
                    </SheetTitle>
                    <SheetDescription>
                      <Trans>管理模型與相關工具。</Trans>
                    </SheetDescription>
                  </SheetHeader>
                  <div className="min-h-0 flex-1 scroll-fade overflow-y-auto p-2 pb-6 sm:px-4">
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
                      onOpenModelFolder={workspace.openModelFolder}
                      onRedownload={workspace.redownloadModel}
                      onRefresh={workspace.refreshRuntime}
                    />
                  </div>
                </SheetContent>
              </Sheet>
            }
          />
          <div className="min-h-0 min-w-0 flex-1 overflow-hidden">
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
              onCancelTask={workspace.cancelTask}
              onRemoveTask={workspace.removeTask}
              onRetryTask={workspace.retryTask}
              cancellingTaskId={workspace.cancellingTaskId}
            />
          </div>
        </section>
      </main>
    </TooltipProvider>
  );
}

export default App;
