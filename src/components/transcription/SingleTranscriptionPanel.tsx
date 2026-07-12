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
import { basename } from "@/lib/format";
import { cn } from "@/lib/utils";
import type { OptionsState } from "@/types/transcription";

export function SingleTranscriptionPanel({
  singleFile,
  outputDir,
  options,
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
  isTranscribing: boolean;
  canRun: boolean;
  onPickFile: () => void;
  onPickOutputDir: () => void;
  onOptionsChange: (next: OptionsState) => void;
  onRun: () => void;
}) {
  return (
    <div className="flex flex-col gap-3">
      <Card>
        <CardHeader>
          <CardTitle>單次轉錄</CardTitle>
          <CardDescription>
            選一個音訊或影片檔，轉成文字並輸出 SRT。
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <button
            type="button"
            className={cn(
              "flex w-full min-w-0 items-center gap-3.5 rounded-lg border border-dashed bg-muted/70 p-[18px] text-left transition-colors hover:border-primary/70 hover:bg-primary/5 focus-visible:border-primary/70 focus-visible:ring-2 focus-visible:ring-primary/20 focus-visible:outline-none",
              singleFile &&
                "border-primary/70 bg-primary/5 ring-2 ring-primary/20",
            )}
            onClick={onPickFile}
          >
            <span className="flex size-11 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
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
              <FieldLabel htmlFor="single-output-dir">
                SRT 輸出資料夾
              </FieldLabel>
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
        </CardContent>
        <CardFooter className="flex-col items-stretch gap-3 min-[721px]:flex-row min-[721px]:items-center min-[721px]:justify-between">
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
