import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { AboutPanel } from "@/components/transcription/AboutPanel";
import {
  dynamicActivate,
  getAboutWindowTitle,
  isLocale,
  type NativeMenuLabels,
} from "@/i18n";

type NativeMenuLabelsPayload = NativeMenuLabels & {
  locale: string;
};

export function AboutWindow() {
  useEffect(() => {
    let isUnmounted = false;
    let unlisten: (() => void) | undefined;

    void listen<NativeMenuLabelsPayload>(
      "native-menu-labels",
      ({ payload }) => {
        if (!isLocale(payload.locale)) {
          return;
        }

        void dynamicActivate(payload.locale)
          .then(() => getCurrentWindow().setTitle(getAboutWindowTitle()))
          .catch(() => {
            // Ignore failures when running outside the Tauri runtime.
          });
      },
    )
      .then((stopListening) => {
        if (isUnmounted) {
          stopListening();
        } else {
          unlisten = stopListening;
        }
      })
      .catch(() => {
        // Ignore failures when running outside the Tauri runtime.
      });

    return () => {
      isUnmounted = true;
      unlisten?.();
    };
  }, []);

  return (
    <main className="min-h-screen bg-background p-6 text-foreground">
      <div className="mx-auto w-full max-w-lg">
        <AboutPanel />
      </div>
    </main>
  );
}
