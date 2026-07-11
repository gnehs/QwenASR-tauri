import { FileTextIcon, Settings2Icon, SubtitlesIcon } from "lucide-react";
import { Trans } from "@lingui/react/macro";

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
        <CardTitle><Trans>輸出結果</Trans></CardTitle>
        <CardDescription><Trans>完成後可檢查全文、偵測語言、輸出路徑與時間分段。</Trans></CardDescription>
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
                    <div className="truncate">
                      {latest.languageSource === "specified" ? <Trans>轉錄語言</Trans> : <Trans>偵測語言</Trans>}：
                      {latest.detectedLanguage ?? <Trans>未提供</Trans>}
                    </div>
                    <div className="truncate text-xs">
                      <Trans>語言來源</Trans>：
                      {latest.languageSource === "specified" ? <Trans>指定語言</Trans> :
                        latest.languageSource === "detected" ? <Trans>模型偵測</Trans> : <Trans>未知來源</Trans>}
                    </div>
                    <div className="truncate text-xs">
                      <Trans>時間對齊</Trans>：
                      {latest.alignmentStatus === "forced" ? <Trans>精準對齊</Trans> :
                        latest.alignmentStatus === "partial" ? <Trans>部分對齊</Trans> : <Trans>估算時間</Trans>}
                      {typeof latest.alignmentCoverage === "number" ? ` · ${(Math.max(0, Math.min(1, latest.alignmentCoverage)) * 100).toFixed(0)}%` : null}
                    </div>
                    <div className="flex flex-col gap-0.5 truncate text-xs">
                      {latest.txtPath ? <span>TXT: {latest.txtPath}</span> : null}
                      {latest.srtPath ? <span>SRT: {latest.srtPath}</span> : null}
                      {latest.jsonPath ? <span>JSON: {latest.jsonPath}</span> : null}
                      {!latest.txtPath && !latest.srtPath && !latest.jsonPath ? (
                        <Trans>未輸出檔案</Trans>
                      ) : null}
                    </div>
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
                    <TableHead><Trans>時間</Trans></TableHead>
                    <TableHead><Trans>文字</Trans></TableHead>
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
                  <div className="text-sm font-medium"><Trans>批次結果</Trans></div>
                  {results.map((result) => (
                    <div key={result.audioPath} className="result-row">
                      <span className="truncate">
                        {basename(result.audioPath)}
                      </span>
                      <Badge variant={result.srtPath || result.txtPath || result.jsonPath ? "secondary" : "outline"}>
                        {result.srtPath || result.txtPath || result.jsonPath ? <Trans>已輸出</Trans> : <Trans>未輸出</Trans>}
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
