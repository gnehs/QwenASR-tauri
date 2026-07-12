import { Trans } from "@lingui/react/macro";
import { CoffeeIcon, ExternalLinkIcon, XIcon } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

const BUY_ME_A_COFFEE_URL = "https://www.buymeacoffee.com/gnehs";

async function openExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (error) {
    console.error("Unable to open external URL", error);
  }
}

export function SupportBanner({ onDismiss }: { onDismiss: () => void }) {
  return (
    <Alert
      className="flex w-full max-w-md items-center gap-2.5 border-primary/20 bg-primary/5 px-3 py-2.5 text-left"
      role="note"
    >
      <CoffeeIcon className="size-4 shrink-0 text-primary" aria-hidden="true" />
      <p className="min-w-0 flex-1 text-xs leading-relaxed text-foreground">
        <Trans>這個 APP 有幫到你嗎？考慮請我喝杯咖啡吧！</Trans>
      </p>
      <div className="flex shrink-0 items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2"
          onClick={() => void openExternalUrl(BUY_ME_A_COFFEE_URL)}
        >
          <Trans>請我喝杯咖啡</Trans>
          <ExternalLinkIcon data-icon="inline-end" />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close"
          onClick={onDismiss}
        >
          <XIcon />
          <span className="sr-only">Close</span>
        </Button>
      </div>
    </Alert>
  );
}
