import { CoffeeIcon, ExternalLinkIcon, GitForkIcon } from "lucide-react";
import { Trans } from "@lingui/react/macro";
import { openUrl } from "@tauri-apps/plugin-opener";

import { SettingsSection } from "@/components/transcription/SettingsSection";
import { Button } from "@/components/ui/button";

const GITHUB_URL = "https://github.com/gnehs/QwenASR-tauri";
const BUY_ME_A_COFFEE_URL = "https://www.buymeacoffee.com/gnehs";

async function openExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (error) {
    console.error("Unable to open external URL", error);
  }
}

export function AboutPanel() {
  return (
    <SettingsSection
      id="about-panel-title"
      title={<Trans>關於</Trans>}
      description={<Trans>查看專案連結與建置資訊。</Trans>}
    >
      <dl className="divide-y rounded-lg border">
        <div className="flex items-center justify-between gap-4 p-3">
          <dt className="text-sm font-medium">
            <Trans>版本</Trans>
          </dt>
          <dd className="font-mono text-sm text-muted-foreground">
            {__APP_VERSION__}
          </dd>
        </div>
        <div className="flex items-center justify-between gap-4 p-3">
          <dt className="text-sm font-medium">
            <Trans>Commit SHA</Trans>
          </dt>
          <dd className="font-mono text-sm text-muted-foreground">
            {__COMMIT_SHA__}
          </dd>
        </div>
      </dl>
      <div className="flex flex-col gap-3">
        <aside className="flex items-center justify-between gap-3 rounded-lg border border-primary/20 bg-primary/5 px-3 py-3">
          <div className="flex min-w-0 items-center gap-2.5">
            <div className="flex shrink-0 items-center justify-center text-primary">
              <CoffeeIcon aria-hidden="true" />
            </div>
            <p className="min-w-0 text-sm font-medium text-foreground">
              <Trans>這個 APP 有幫到你嗎？考慮請我喝杯咖啡吧！</Trans>
            </p>
          </div>
          <Button
            size="sm"
            className="shrink-0"
            onClick={() => void openExternalUrl(BUY_ME_A_COFFEE_URL)}
          >
            <Trans>請我喝杯咖啡</Trans>
            <ExternalLinkIcon data-icon="inline-end" />
          </Button>
        </aside>
        <Button
          variant="outline"
          size="sm"
          className="self-start"
          onClick={() => void openExternalUrl(GITHUB_URL)}
        >
          <GitForkIcon data-icon="inline-start" />
          <Trans>GitHub</Trans>
          <ExternalLinkIcon data-icon="inline-end" />
        </Button>
      </div>
    </SettingsSection>
  );
}
