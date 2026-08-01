import { useEffect, useId, useState } from "react";
import { i18n } from "@lingui/core";
import { Trans, useLingui } from "@lingui/react/macro";
import { useTheme } from "next-themes";

import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldTitle,
} from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ModelPanel } from "@/components/transcription/ModelPanel";
import { RuntimePanel } from "@/components/transcription/RuntimePanel";
import { SettingsSection } from "@/components/transcription/SettingsSection";
import { Switch } from "@/components/ui/switch";
import { activateLocale, locales, type Locale } from "@/i18n";
import type {
  DownloadProgress,
  FfmpegStatus,
  ModelStatus,
} from "@/types/transcription";

const themePreferences = ["system", "light", "dark"] as const;
type ThemePreference = (typeof themePreferences)[number];

function isThemePreference(
  value: string | null | undefined,
): value is ThemePreference {
  return (
    value !== null &&
    value !== undefined &&
    themePreferences.includes(value as ThemePreference)
  );
}

export function SettingsPanel({
  models,
  downloadProgress,
  downloadMovingAverageSpeedBytesPerSec,
  isDownloading,
  deletingModelId,
  isTranscribing,
  autoSkipNonSpeech,
  onAutoSkipNonSpeechChange,
  ffmpeg,
  onDownload,
  onDeleteModel,
  onOpenModelFolder,
  onRedownload,
  onRefresh,
}: {
  models: ModelStatus[];
  downloadProgress: DownloadProgress | null;
  downloadMovingAverageSpeedBytesPerSec: number;
  isDownloading: boolean;
  deletingModelId: string | null;
  isTranscribing: boolean;
  autoSkipNonSpeech: boolean;
  onAutoSkipNonSpeechChange: (checked: boolean) => void;
  ffmpeg: FfmpegStatus;
  onDownload: (modelId?: string) => void;
  onDeleteModel: (modelId: string) => Promise<boolean>;
  onOpenModelFolder: (modelId: string) => Promise<void>;
  onRedownload: (modelId: string) => Promise<void>;
  onRefresh: () => void;
}) {
  const { t } = useLingui();
  const autoSkipNonSpeechId = useId();
  const [isChangingLocale, setIsChangingLocale] = useState(false);
  const [isThemeMounted, setIsThemeMounted] = useState(false);
  const { setTheme, theme } = useTheme();
  const activeLocale = (
    i18n.locale in locales ? i18n.locale : "zh-Hant"
  ) as Locale;
  const activeTheme = isThemePreference(theme) ? theme : "system";
  const themeLabels: Record<ThemePreference, string> = {
    system: t`跟隨系統`,
    light: t`亮色`,
    dark: t`暗色`,
  };

  useEffect(() => {
    setIsThemeMounted(true);
  }, []);

  async function handleLocaleChange(value: string | null) {
    if (!value || value === activeLocale) return;

    setIsChangingLocale(true);
    try {
      await activateLocale(value as Locale);
    } finally {
      setIsChangingLocale(false);
    }
  }

  return (
    <div className="flex min-h-0 flex-col gap-6">
      <SettingsSection
        id="interface-panel-title"
        title={<Trans>介面</Trans>}
        description={<Trans>選擇介面語言與主題配色。</Trans>}
      >
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="interface-language">
              <Trans>語言</Trans>
            </FieldLabel>
            <Select
              value={activeLocale}
              disabled={isChangingLocale}
              onValueChange={handleLocaleChange}
            >
              <SelectTrigger
                id="interface-language"
                className="w-full"
                aria-label={t`介面語言`}
              >
                <SelectValue>{locales[activeLocale]}</SelectValue>
              </SelectTrigger>
              <SelectContent alignItemWithTrigger={false}>
                <SelectGroup>
                  {Object.entries(locales).map(([value, label]) => (
                    <SelectItem key={value} value={value}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Field>
            <FieldLabel htmlFor="interface-theme">
              <Trans>主題配色</Trans>
            </FieldLabel>
            <Select
              value={isThemeMounted ? activeTheme : null}
              disabled={!isThemeMounted}
              onValueChange={(value) => {
                if (isThemePreference(value)) {
                  setTheme(value);
                }
              }}
            >
              <SelectTrigger
                id="interface-theme"
                className="w-full"
                aria-label={t`主題配色`}
              >
                <SelectValue placeholder={t`載入中…`}>
                  {isThemeMounted ? themeLabels[activeTheme] : undefined}
                </SelectValue>
              </SelectTrigger>
              <SelectContent alignItemWithTrigger={false}>
                <SelectGroup>
                  {themePreferences.map((value) => (
                    <SelectItem key={value} value={value}>
                      {themeLabels[value]}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
        </FieldGroup>
      </SettingsSection>
      <SettingsSection
        id="transcription-panel-title"
        title={<Trans>轉錄</Trans>}
        description={<Trans>調整音訊分析與語音轉錄流程。</Trans>}
      >
        <Field orientation="horizontal">
          <FieldContent>
            <FieldTitle id={`${autoSkipNonSpeechId}-label`}>
              <Trans>自動跳過非說話片段</Trans>
            </FieldTitle>
            <FieldDescription id={`${autoSkipNonSpeechId}-description`}>
              <Trans>
                可以自動跳過非說話片段，加速語音轉錄流程，但可能某些說話片段會被意外跳過。
              </Trans>
            </FieldDescription>
          </FieldContent>
          <Switch
            aria-labelledby={`${autoSkipNonSpeechId}-label`}
            aria-describedby={`${autoSkipNonSpeechId}-description`}
            checked={autoSkipNonSpeech}
            onCheckedChange={onAutoSkipNonSpeechChange}
          />
        </Field>
      </SettingsSection>
      <ModelPanel
        models={models}
        downloadProgress={downloadProgress}
        downloadMovingAverageSpeedBytesPerSec={
          downloadMovingAverageSpeedBytesPerSec
        }
        isDownloading={isDownloading}
        deletingModelId={deletingModelId}
        isTranscribing={isTranscribing}
        onDownload={onDownload}
        onDeleteModel={onDeleteModel}
        onOpenModelFolder={onOpenModelFolder}
        onRedownload={onRedownload}
        onRefresh={onRefresh}
      />
      <RuntimePanel ffmpeg={ffmpeg} />
    </div>
  );
}
