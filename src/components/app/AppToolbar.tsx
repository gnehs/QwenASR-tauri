import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { SidebarTrigger } from "@/components/ui/sidebar";
import type { FfmpegStatus } from "@/types/transcription";

export function AppToolbar({
  ffmpeg,
  title,
  subtitle,
  actions,
}: {
  ffmpeg: FfmpegStatus;
  title: string;
  subtitle: string;
  actions?: ReactNode;
}) {
  return (
    <header className="window-toolbar">
      <SidebarTrigger />
      <div className="min-w-0">
        <h1 className="truncate font-heading text-base font-medium">{title}</h1>
        <p className="truncate text-sm text-muted-foreground">{subtitle}</p>
      </div>
      <div className="window-toolbar-actions">
        {actions}
        {!ffmpeg.available ? (
          <Badge variant="destructive">缺少 FFmpeg</Badge>
        ) : null}
      </div>
    </header>
  );
}
