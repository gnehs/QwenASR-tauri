import { useState } from "react";
import { i18n } from "@lingui/core";
import { Trans, useLingui } from "@lingui/react/macro";

import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
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
import { activateLocale, locales, type Locale } from "@/i18n";
import type {
  DownloadProgress,
  FfmpegStatus,
  ModelStatus,
} from "@/types/transcription";

export function SettingsPanel({
  models,
  downloadProgress,
  downloadMovingAverageSpeedBytesPerSec,
  isDownloading,
  deletingModelId,
  isTranscribing,
  ffmpeg,
  onDownload,
  onDeleteModel,
  onRefresh,
}: {
  models: ModelStatus[];
  downloadProgress: DownloadProgress | null;
  downloadMovingAverageSpeedBytesPerSec: number;
  isDownloading: boolean;
  deletingModelId: string | null;
  isTranscribing: boolean;
  ffmpeg: FfmpegStatus;
  onDownload: (modelId?: string) => void;
  onDeleteModel: (modelId: string) => Promise<boolean>;
  onRefresh: () => void;
}) {
  const { t } = useLingui();
  const [isChangingLocale, setIsChangingLocale] = useState(false);
  const activeLocale = (
    i18n.locale in locales ? i18n.locale : "zh-Hant"
  ) as Locale;

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
        id="language-panel-title"
        title={<Trans>介面語言</Trans>}
        description={<Trans>選擇 QwenASR Studio 使用的介面語言。</Trans>}
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
            <FieldDescription>
              <Trans>變更後會立即套用並自動保存。</Trans>
            </FieldDescription>
          </Field>
        </FieldGroup>
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
        onRefresh={onRefresh}
      />
      <RuntimePanel ffmpeg={ffmpeg} />
    </div>
  );
}
