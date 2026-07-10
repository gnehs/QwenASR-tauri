import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import type { FfmpegStatus } from "@/types/transcription";

export function RuntimePanel({ ffmpeg }: { ffmpeg: FfmpegStatus }) {
  return (
    <section className="settings-section" aria-labelledby="runtime-panel-title">
      <div className="settings-section-header">
        <h2 id="runtime-panel-title" className="settings-section-title">
          工具狀態
        </h2>
        <p className="settings-section-description">將音訊轉換為支援的格式。</p>
      </div>
      <div className="settings-section-content">
        <Card size="sm">
          <CardHeader>
            <CardTitle>FFmpeg</CardTitle>
            <CardDescription className="truncate">
              {ffmpeg.version ?? "未偵測到 ffmpeg"}
            </CardDescription>
            <CardAction>
              <Badge variant={ffmpeg.available ? "secondary" : "destructive"}>
                {ffmpeg.available ? "可用" : "缺少"}
              </Badge>
            </CardAction>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            <Separator />
            <p className="text-sm text-muted-foreground">
              非 WAV 音訊、影片檔或不符合 16 kHz mono 的 WAV 會透過 FFmpeg 正規化後再送入 QwenASR。
            </p>
            {!ffmpeg.available ? (
              <div className="tool-install-hint">
                <span>請在終端機安裝：</span>
                <code>brew install ffmpeg</code>
              </div>
            ) : null}
          </CardContent>
        </Card>
      </div>
    </section>
  );
}
