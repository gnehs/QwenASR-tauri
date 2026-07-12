import { i18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { emit } from "@tauri-apps/api/event";

export type Locale = "zh-Hant" | "zh-Hans" | "en" | "ja";

export const locales: Record<Locale, string> = {
  "zh-Hant": "繁體中文",
  "zh-Hans": "简体中文",
  en: "English",
  ja: "日本語",
};

export const defaultLocale: Locale = "zh-Hant";
export const localeStorageKey = "qwenasr.locale";
const aboutWindowTitleMessage = msg`關於 QwenASR Studio`;
const settingsMenuMessage = msg`設定`;
const fileMenuMessage = msg`檔案`;
const newTaskMenuMessage = msg`新增任務`;
const editMenuMessage = msg`編輯`;
const windowMenuMessage = msg`視窗`;
const helpMenuMessage = msg`說明`;
const githubMenuMessage = msg`GitHub`;

export type NativeMenuLabels = {
  about: string;
  settings: string;
  file: string;
  new_task: string;
  edit: string;
  window: string;
  help: string;
  github: string;
};

export function isLocale(value: string | null | undefined): value is Locale {
  return (
    value !== null &&
    value !== undefined &&
    Object.prototype.hasOwnProperty.call(locales, value)
  );
}

export function getStoredLocale(): Locale {
  if (typeof window === "undefined") {
    return defaultLocale;
  }

  try {
    const storedLocale = window.localStorage.getItem(localeStorageKey);
    return isLocale(storedLocale) ? storedLocale : defaultLocale;
  } catch {
    return defaultLocale;
  }
}

export function getInitialLocale(): Locale {
  return getStoredLocale();
}

export function saveLocale(locale: Locale): void {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.localStorage.setItem(localeStorageKey, locale);
  } catch {
    // Ignore storage failures so locale activation still succeeds.
  }
}

export function getAboutWindowTitle(): string {
  return i18n._(aboutWindowTitleMessage);
}

export function getNativeMenuLabels(): NativeMenuLabels {
  return {
    about: getAboutWindowTitle(),
    settings: i18n._(settingsMenuMessage),
    file: i18n._(fileMenuMessage),
    new_task: i18n._(newTaskMenuMessage),
    edit: i18n._(editMenuMessage),
    window: i18n._(windowMenuMessage),
    help: i18n._(helpMenuMessage),
    github: i18n._(githubMenuMessage),
  };
}

export async function syncNativeMenuLabels(locale: Locale): Promise<void> {
  try {
    await emit("native-menu-labels", {
      locale,
      ...getNativeMenuLabels(),
    });
  } catch {
    // Ignore failures when running outside the Tauri runtime.
  }
}

function applyLocaleToDocument(locale: Locale): void {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.lang = locale;
  document.documentElement.dataset.locale = locale;
}

export async function dynamicActivate(locale: Locale): Promise<void> {
  const { messages } = await import(`./locales/${locale}/messages.po`);

  i18n.load(locale, messages);
  i18n.activate(locale);
  applyLocaleToDocument(locale);
}

export async function activateLocale(locale: Locale): Promise<void> {
  await dynamicActivate(locale);
  saveLocale(locale);
  await syncNativeMenuLabels(locale);
}

export async function activateStoredLocale(): Promise<Locale> {
  const locale = getStoredLocale();
  await activateLocale(locale);
  return locale;
}

export { i18n };
