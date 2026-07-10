import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import type { FfmpegStatus } from "@/types/transcription";

export function AppToolbar({
  ffmpeg,
  title,
  subtitle,
  actions,
  utilities,
}: {
  ffmpeg: FfmpegStatus;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  utilities?: ReactNode;
}) {
  return (
    <header className="window-toolbar">
      <div className="window-toolbar-copy">
        <h1 className="truncate font-heading text-base font-medium">{title}</h1>
        {subtitle ? (
          <p className="truncate text-sm text-muted-foreground">{subtitle}</p>
        ) : null}
      </div>
      {actions || !ffmpeg.available ? (
        <div className="window-toolbar-actions">
          <div className="window-toolbar-primary-actions">{actions}</div>
          {!ffmpeg.available ? (
            <Badge variant="destructive">缺少 FFmpeg</Badge>
          ) : null}
        </div>
      ) : null}
      <div className="window-toolbar-utilities">{utilities}</div>
    </header>
  );
}
