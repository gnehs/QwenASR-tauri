import { FileTextIcon, Settings2Icon, SubtitlesIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
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
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import { TranscriptionProgressPanel } from "@/components/transcription/TranscriptionProgressPanel";
import { basename, formatTimestamp } from "@/lib/format";
import type {
  TranscriptionProgress,
  TranscriptionResult,
} from "@/types/transcription";

export function ResultsPanel({
  results,
  isTranscribing,
  progress,
}: {
  results: TranscriptionResult[];
  isTranscribing: boolean;
  progress: TranscriptionProgress | null;
}) {
  const latest = results[0];

  return (
    <Card className="h-full">
      <CardHeader>
        <CardTitle>輸出結果</CardTitle>
        <CardDescription>完成後可檢查全文、SRT 路徑與時間分段。</CardDescription>
        {latest ? (
          <CardAction>
            <Badge variant="secondary">
              {(latest.durationMs / 1000).toFixed(1)} 秒
            </Badge>
          </CardAction>
        ) : null}
      </CardHeader>
      <CardContent className="results-content">
        {isTranscribing ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <Settings2Icon />
              </EmptyMedia>
              <EmptyTitle>轉錄中</EmptyTitle>
              <EmptyDescription>模型載入與推論需要一些時間。</EmptyDescription>
            </EmptyHeader>
            <EmptyContent>
              <TranscriptionProgressPanel progress={progress} />
            </EmptyContent>
          </Empty>
        ) : results.length === 0 ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <SubtitlesIcon />
              </EmptyMedia>
              <EmptyTitle>尚無結果</EmptyTitle>
              <EmptyDescription>完成轉錄後會在這裡顯示。</EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          <ScrollArea
            className="h-full"
            viewportClassName="scroll-fade"
          >
            <div className="flex flex-col gap-4 pr-3">
              <div className="result-summary">
                <FileTextIcon />
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">
                    {basename(latest.audioPath)}
                  </div>
                  <div className="truncate text-sm text-muted-foreground">
                    {latest.srtPath ? `SRT: ${latest.srtPath}` : "未輸出 SRT"}
                  </div>
                </div>
              </div>
              <Separator />
              <Textarea
                readOnly
                value={latest.text}
                className="min-h-40 resize-none"
              />
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>時間</TableHead>
                    <TableHead>文字</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {latest.segments.map((segment, index) => (
                    <TableRow key={`${segment.startMs}-${index}`}>
                      <TableCell className="whitespace-nowrap text-muted-foreground">
                        {formatTimestamp(segment.startMs)}
                      </TableCell>
                      <TableCell className="srt-preview-text">
                        {segment.text}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              {results.length > 1 ? (
                <>
                  <Separator />
                  <div className="text-sm font-medium">批次結果</div>
                  {results.map((result) => (
                    <div key={result.audioPath} className="result-row">
                      <span className="truncate">
                        {basename(result.audioPath)}
                      </span>
                      <Badge variant={result.srtPath ? "secondary" : "outline"}>
                        {result.srtPath ? "SRT" : "文字"}
                      </Badge>
                    </div>
                  ))}
                </>
              ) : null}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
