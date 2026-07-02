import { FileAudioIcon, FolderOpenIcon, PlayIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Spinner } from "@/components/ui/spinner";
import { OptionsPanel } from "@/components/transcription/OptionsPanel";
import { TranscriptionProgressPanel } from "@/components/transcription/TranscriptionProgressPanel";
import { basename } from "@/lib/format";
import { cn } from "@/lib/utils";
import type {
  OptionsState,
  TranscriptionProgress,
} from "@/types/transcription";

export function SingleTranscriptionPanel({
  singleFile,
  outputDir,
  options,
  progress,
  isTranscribing,
  canRun,
  onPickFile,
  onPickOutputDir,
  onOptionsChange,
  onRun,
}: {
  singleFile: string;
  outputDir: string;
  options: OptionsState;
  progress: TranscriptionProgress | null;
  isTranscribing: boolean;
  canRun: boolean;
  onPickFile: () => void;
  onPickOutputDir: () => void;
  onOptionsChange: (next: OptionsState) => void;
  onRun: () => void;
}) {
  return (
    <div className="panel-stack">
      <Card className="workflow-card">
        <CardHeader>
          <CardTitle>單次轉錄</CardTitle>
          <CardDescription>
            選一個音訊或影片檔，轉成文字並輸出 SRT。
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <button
            type="button"
            className={cn("file-pick-surface", singleFile && "is-selected")}
            onClick={onPickFile}
          >
            <span className="file-pick-icon">
              <FileAudioIcon />
            </span>
            <span className="min-w-0">
              <span className="block truncate font-medium">
                {singleFile ? basename(singleFile) : "選取音訊或影片"}
              </span>
              <span className="block truncate text-sm text-muted-foreground">
                {singleFile || "支援 wav、mp3、m4a、mp4、mov、mkv"}
              </span>
            </span>
          </button>
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="single-output-dir">SRT 輸出資料夾</FieldLabel>
              <div className="flex gap-2">
                <Input
                  id="single-output-dir"
                  readOnly
                  value={outputDir}
                  placeholder="預設輸出到來源檔案所在資料夾"
                />
                <Button variant="outline" onClick={onPickOutputDir}>
                  <FolderOpenIcon data-icon="inline-start" />
                  選取
                </Button>
              </div>
            </Field>
          </FieldGroup>
          <OptionsPanel options={options} onChange={onOptionsChange} />
          {isTranscribing ? (
            <TranscriptionProgressPanel progress={progress} />
          ) : null}
        </CardContent>
        <CardFooter className="workflow-footer">
          <p className="text-sm text-muted-foreground">
            {canRun ? "準備好後即可開始。" : "請先選檔並確認模型已下載。"}
          </p>
          <Button disabled={!canRun} onClick={onRun}>
            {isTranscribing ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <PlayIcon data-icon="inline-start" />
            )}
            {isTranscribing ? "轉錄中" : "開始轉錄"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
