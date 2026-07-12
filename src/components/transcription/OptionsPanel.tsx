import { useId, useMemo } from "react";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react";
import { Trans } from "@lingui/react/macro";
import { SlidersHorizontalIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
  FieldTitle,
} from "@/components/ui/field";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Switch } from "@/components/ui/switch";
import { localizeLanguageGroups } from "@/lib/app-constants";
import type { OptionsState } from "@/types/transcription";

export function OptionsPanel({
  options,
  onChange,
}: {
  options: OptionsState;
  onChange: (next: OptionsState) => void;
}) {
  const { _ } = useLingui();
  const outputId = useId();
  const segmentId = useId();
  const languageGroups = useMemo(() => localizeLanguageGroups(_), [_]);
  const languageItems = useMemo(
    () => languageGroups.flatMap((group) => group.items),
    [languageGroups],
  );
  const language = languageItems.find(
    (item) => item.value === options.language,
  );
  const outputFormats = [
    options.writeTxt ? "TXT" : null,
    options.writeSrt ? "SRT" : null,
    options.writeJson ? "JSON" : null,
  ].filter(Boolean);
  const summaryItems = [
    language?.label ?? _(msg`自動偵測`),
    outputFormats.length > 0
      ? `${_(msg`輸出`)} ${outputFormats.join("、")}`
      : _(msg`不輸出檔案`),
    options.segmentByPunctuation ? _(msg`依標點斷句`) : _(msg`依音訊片段斷句`),
  ].filter(Boolean);

  return (
    <Sheet>
      <div className="flex min-w-0 items-center justify-between gap-3 rounded-lg border bg-muted/40 px-3 py-2.5">
        <div className="min-w-0">
          <div className="text-sm font-medium">
            <Trans>轉錄設定</Trans>
          </div>
          <div className="truncate text-sm text-muted-foreground">
            {summaryItems.join(" · ")}
          </div>
        </div>
        <SheetTrigger render={<Button variant="outline" size="sm" />}>
          <SlidersHorizontalIcon data-icon="inline-start" />
          <Trans>調整</Trans>
        </SheetTrigger>
      </div>
      <SheetContent className="w-[min(420px,100vw)]">
        <SheetHeader>
          <SheetTitle>
            <Trans>轉錄設定</Trans>
          </SheetTitle>
          <SheetDescription>
            <Trans>
              多數情況可以維持自動偵測，只在需要指定語言或輸出格式時修改。
            </Trans>
          </SheetDescription>
        </SheetHeader>
        <div className="min-h-0 overflow-auto px-4 pb-4">
          <FieldGroup>
            <Field>
              <FieldLabel>
                <Trans>輸出語言</Trans>
              </FieldLabel>
              <Select
                items={languageItems}
                value={options.language}
                onValueChange={(value) =>
                  onChange({ ...options, language: String(value) })
                }
              >
                <SelectTrigger className="w-full" aria-label={_(msg`輸出語言`)}>
                  <SelectValue>
                    {language?.label ?? <Trans>自動偵測</Trans>}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent alignItemWithTrigger={false}>
                  <SelectGroup>
                    {languageGroups.map((group) => (
                      <SelectGroup key={group.id}>
                        <SelectLabel>{group.label}</SelectLabel>
                        {group.items.map((item) => (
                          <SelectItem key={item.value} value={item.value}>
                            {item.label}
                          </SelectItem>
                        ))}
                      </SelectGroup>
                    ))}
                  </SelectGroup>
                </SelectContent>
              </Select>
            </Field>

            <FieldSet>
              <FieldLegend variant="label">
                <Trans>輸出格式</Trans>
              </FieldLegend>
              <FieldDescription>
                <Trans>可同時輸出多種格式至指定資料夾。</Trans>
              </FieldDescription>
              <FieldGroup className="gap-3">
                <Field orientation="horizontal">
                  <FieldLabel htmlFor={`${outputId}-txt`}>
                    <FieldContent>
                      <FieldTitle>
                        <Trans>TXT 文字</Trans>
                      </FieldTitle>
                      <FieldDescription id={`${outputId}-txt-description`}>
                        <Trans>輸出純文字逐字稿。</Trans>
                      </FieldDescription>
                    </FieldContent>
                  </FieldLabel>
                  <Checkbox
                    id={`${outputId}-txt`}
                    aria-describedby={`${outputId}-txt-description`}
                    checked={options.writeTxt}
                    onCheckedChange={(checked) =>
                      onChange({ ...options, writeTxt: checked === true })
                    }
                  />
                </Field>
                <Field orientation="horizontal">
                  <FieldLabel htmlFor={`${outputId}-srt`}>
                    <FieldContent>
                      <FieldTitle>
                        <Trans>SRT 字幕</Trans>
                      </FieldTitle>
                      <FieldDescription id={`${outputId}-srt-description`}>
                        <Trans>建立可隨音訊同步顯示的字幕檔。</Trans>
                      </FieldDescription>
                    </FieldContent>
                  </FieldLabel>
                  <Checkbox
                    id={`${outputId}-srt`}
                    aria-describedby={`${outputId}-srt-description`}
                    checked={options.writeSrt}
                    onCheckedChange={(checked) =>
                      onChange({ ...options, writeSrt: checked === true })
                    }
                  />
                </Field>
                <Field orientation="horizontal">
                  <FieldLabel htmlFor={`${outputId}-json`}>
                    <FieldContent>
                      <FieldTitle>
                        <Trans>JSON 資料</Trans>
                      </FieldTitle>
                      <FieldDescription id={`${outputId}-json-description`}>
                        <Trans>輸出含分段資料，支援語言會附逐字時間戳。</Trans>
                      </FieldDescription>
                    </FieldContent>
                  </FieldLabel>
                  <Checkbox
                    id={`${outputId}-json`}
                    aria-describedby={`${outputId}-json-description`}
                    checked={options.writeJson}
                    onCheckedChange={(checked) =>
                      onChange({ ...options, writeJson: checked === true })
                    }
                  />
                </Field>
              </FieldGroup>
            </FieldSet>

            <Field orientation="horizontal">
              <FieldContent>
                <FieldTitle id={`${segmentId}-label`}>
                  <Trans>依標點符號斷句</Trans>
                </FieldTitle>
                <FieldDescription id={`${segmentId}-description`}>
                  <Trans>關閉後會保留系統偵測出的自然音訊片段。</Trans>
                </FieldDescription>
              </FieldContent>
              <Switch
                aria-labelledby={`${segmentId}-label`}
                aria-describedby={`${segmentId}-description`}
                checked={options.segmentByPunctuation}
                onCheckedChange={(checked) =>
                  onChange({ ...options, segmentByPunctuation: checked })
                }
              />
            </Field>
          </FieldGroup>
        </div>
        <SheetFooter>
          <SheetClose render={<Button />}>
            <Trans>完成</Trans>
          </SheetClose>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
