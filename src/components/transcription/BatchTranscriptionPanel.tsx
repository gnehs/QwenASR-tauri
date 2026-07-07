import {
  ArchiveIcon,
  FileAudioIcon,
  FolderOpenIcon,
  PlayIcon,
  XIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Spinner } from "@/components/ui/spinner";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { OptionsPanel } from "@/components/transcription/OptionsPanel";
import { basename } from "@/lib/format";
import type { OptionsState } from "@/types/transcription";

export function BatchTranscriptionPanel({
  files,
  outputDir,
  options,
  isTranscribing,
  canRun,
  onPickFiles,
  onPickOutputDir,
  onRemoveFile,
  onClearFiles,
  onOptionsChange,
  onRun,
}: {
  files: string[];
  outputDir: string;
  options: OptionsState;
  isTranscribing: boolean;
  canRun: boolean;
  onPickFiles: () => void;
  onPickOutputDir: () => void;
  onRemoveFile: (path: string) => void;
  onClearFiles: () => void;
  onOptionsChange: (next: OptionsState) => void;
  onRun: () => void;
}) {
  return (
    <div className="panel-stack">
      <Card className="workflow-card">
        <CardHeader>
          <CardTitle>批次佇列</CardTitle>
          <CardDescription>加入多個檔案後依序轉錄並輸出字幕。</CardDescription>
          <CardAction>
            <Badge variant={files.length > 0 ? "secondary" : "outline"}>
              {files.length} 個檔案
            </Badge>
          </CardAction>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="workflow-actions">
            <Button onClick={onPickFiles}>
              <FileAudioIcon data-icon="inline-start" />
              加入檔案
            </Button>
            <Button variant="outline" onClick={onPickOutputDir}>
              <FolderOpenIcon data-icon="inline-start" />
              輸出資料夾
            </Button>
            <Button variant="ghost" onClick={onClearFiles}>
              清空佇列
            </Button>
          </div>
          <Input
            aria-label="批次輸出資料夾"
            readOnly
            value={outputDir}
            placeholder="預設輸出到各來源檔案所在資料夾"
          />
          <OptionsPanel options={options} onChange={onOptionsChange} />
          <ScrollArea className="batch-list">
            {files.length > 0 ? (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>檔案</TableHead>
                    <TableHead className="text-right">動作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {files.map((file) => (
                    <TableRow key={file}>
                      <TableCell className="max-w-0 truncate">
                        {basename(file)}
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => onRemoveFile(file)}
                        >
                          <XIcon data-icon="inline-start" />
                          <span className="sr-only">移除 {basename(file)}</span>
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            ) : (
              <Empty>
                <EmptyHeader>
                  <EmptyMedia variant="icon">
                    <ArchiveIcon />
                  </EmptyMedia>
                  <EmptyTitle>尚未加入檔案</EmptyTitle>
                  <EmptyDescription>選取多個檔案後即可批次轉錄。</EmptyDescription>
                </EmptyHeader>
                <EmptyContent>
                  <Button variant="outline" onClick={onPickFiles}>
                    <FileAudioIcon data-icon="inline-start" />
                    加入檔案
                  </Button>
                </EmptyContent>
              </Empty>
            )}
          </ScrollArea>
        </CardContent>
        <CardFooter className="workflow-footer">
          <p className="text-sm text-muted-foreground">
            {canRun ? "佇列會由上而下處理。" : "請加入檔案並確認模型已下載。"}
          </p>
          <Button disabled={!canRun} onClick={onRun}>
            {isTranscribing ? (
              <Spinner data-icon="inline-start" />
            ) : (
              <PlayIcon data-icon="inline-start" />
            )}
            {isTranscribing ? "批次轉錄中" : "開始批次轉錄"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
