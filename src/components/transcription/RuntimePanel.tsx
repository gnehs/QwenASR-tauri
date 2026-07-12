import { Badge } from "@/components/ui/badge";
import { Trans } from "@lingui/react/macro";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { SettingsSection } from "@/components/transcription/SettingsSection";
import type { FfmpegStatus } from "@/types/transcription";

export function RuntimePanel({ ffmpeg }: { ffmpeg: FfmpegStatus }) {
  return (
    <SettingsSection
      id="runtime-panel-title"
      title={<Trans>工具狀態</Trans>}
      description={<Trans>將音訊轉換為支援的格式。</Trans>}
    >
      <Card size="sm">
        <CardHeader>
          <CardTitle>FFmpeg</CardTitle>
          <CardDescription className="truncate">
            {ffmpeg.version ?? <Trans>未偵測到 ffmpeg</Trans>}
          </CardDescription>
          <CardAction>
            <Badge variant={ffmpeg.available ? "secondary" : "destructive"}>
              {ffmpeg.available ? <Trans>可用</Trans> : <Trans>缺少</Trans>}
            </Badge>
          </CardAction>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <Separator />
          <p className="text-sm text-muted-foreground">
            <Trans>
              非 WAV 音訊、影片檔或不符合 16 kHz mono 的 WAV 會透過 FFmpeg
              正規化後再送入 QwenASR。
            </Trans>
          </p>
          {!ffmpeg.available ? (
            <div className="flex min-w-0 flex-wrap items-center gap-2 rounded-md bg-muted p-2 text-sm text-muted-foreground">
              <span>
                <Trans>請在終端機安裝：</Trans>
              </span>
              <code className="rounded-sm bg-background px-1.5 py-0.5 font-mono text-xs text-foreground">
                brew install ffmpeg
              </code>
            </div>
          ) : null}
        </CardContent>
      </Card>
    </SettingsSection>
  );
}
