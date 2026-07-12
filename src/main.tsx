import React from "react";
import ReactDOM from "react-dom/client";
import { i18n } from "@lingui/core";
import { I18nProvider } from "@lingui/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { AboutWindow } from "./components/transcription/AboutWindow";
import { ThemeProvider } from "./components/theme-provider";
import {
  dynamicActivate,
  getAboutWindowTitle,
  getInitialLocale,
  syncNativeMenuLabels,
} from "./i18n";
import "./index.css";

const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement,
);
const isAboutWindow =
  new URLSearchParams(window.location.search).get("window") === "about";

async function bootstrap() {
  const initialLocale = getInitialLocale();
  await dynamicActivate(initialLocale);

  if (isAboutWindow) {
    try {
      await getCurrentWindow().setTitle(getAboutWindowTitle());
    } catch {
      // Ignore failures when running outside the Tauri runtime.
    }
  } else {
    await syncNativeMenuLabels(initialLocale);
  }

  root.render(
    <React.StrictMode>
      <ThemeProvider>
        <I18nProvider i18n={i18n}>
          {isAboutWindow ? <AboutWindow /> : <App />}
        </I18nProvider>
      </ThemeProvider>
    </React.StrictMode>,
  );
}

void bootstrap();
