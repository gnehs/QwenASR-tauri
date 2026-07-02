import { RefreshCwIcon } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { SidebarTrigger } from "@/components/ui/sidebar";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { FfmpegStatus } from "@/types/transcription";

export function AppToolbar({
  ffmpeg,
  title,
  subtitle,
  modelReady,
  modelLabel,
  onOpenSettings,
  onRefresh,
}: {
  ffmpeg: FfmpegStatus;
  title: string;
  subtitle: string;
  modelReady: boolean;
  modelLabel: string;
  onOpenSettings: () => void;
  onRefresh: () => void;
}) {
  return (
    <header className="window-toolbar">
      <SidebarTrigger />
      <div className="min-w-0">
        <h1 className="truncate font-heading text-base font-medium">{title}</h1>
        <p className="truncate text-sm text-muted-foreground">{subtitle}</p>
      </div>
      <div className="ml-auto flex items-center gap-2">
        <Badge variant={modelReady ? "secondary" : "destructive"}>
          模型 {modelReady ? modelLabel : "未下載"}
        </Badge>
        <Badge variant={ffmpeg.available ? "secondary" : "destructive"}>
          FFmpeg {ffmpeg.available ? "Ready" : "Missing"}
        </Badge>
        {!modelReady ? (
          <Button variant="outline" size="sm" onClick={onOpenSettings}>
            下載模型
          </Button>
        ) : null}
        <Tooltip>
          <TooltipTrigger
            render={
              <Button variant="ghost" size="icon-sm" onClick={onRefresh} />
            }
          >
            <RefreshCwIcon data-icon="inline-start" />
            <span className="sr-only">重新整理</span>
          </TooltipTrigger>
          <TooltipContent>重新整理模型與工具狀態</TooltipContent>
        </Tooltip>
      </div>
    </header>
  );
}
