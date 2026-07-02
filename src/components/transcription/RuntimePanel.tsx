import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import type { FfmpegStatus } from "@/types/transcription";

export function RuntimePanel({ ffmpeg }: { ffmpeg: FfmpegStatus }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>工具狀態</CardTitle>
        <CardDescription>轉檔與音訊正規化能力。</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="text-sm font-medium">FFmpeg</div>
            <p className="truncate text-sm text-muted-foreground">
              {ffmpeg.version ?? "未偵測到 ffmpeg"}
            </p>
          </div>
          <Badge variant={ffmpeg.available ? "secondary" : "destructive"}>
            {ffmpeg.available ? "可用" : "缺少"}
          </Badge>
        </div>
        <Separator />
        <p className="text-sm text-muted-foreground">
          非 WAV 音訊、影片檔或不符合 16 kHz mono 的 WAV 會透過 FFmpeg 正規化後再送入 QwenASR。
        </p>
      </CardContent>
    </Card>
  );
}
