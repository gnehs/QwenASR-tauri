import { SlidersHorizontalIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
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
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
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
import { Textarea } from "@/components/ui/textarea";
import { languageItems } from "@/lib/app-constants";
import { toNumber } from "@/lib/format";
import type { OptionsState } from "@/types/transcription";

export function OptionsPanel({
  options,
  onChange,
}: {
  options: OptionsState;
  onChange: (next: OptionsState) => void;
}) {
  const language = languageItems.find((item) => item.value === options.language);
  const summaryItems = [
    language?.label ?? "自動偵測",
    options.writeSrt ? "輸出 SRT" : "只輸出文字",
    options.convertWithFfmpeg ? "自動轉檔" : "不轉檔",
    options.prompt.trim() ? "含提示詞" : null,
  ].filter(Boolean);

  return (
    <Sheet>
      <div className="options-strip">
        <div className="min-w-0">
          <div className="text-sm font-medium">轉錄設定</div>
          <div className="truncate text-sm text-muted-foreground">
            {summaryItems.join(" · ")}
          </div>
        </div>
        <SheetTrigger render={<Button variant="outline" size="sm" />}>
          <SlidersHorizontalIcon data-icon="inline-start" />
          調整
        </SheetTrigger>
      </div>
      <SheetContent className="options-sheet">
        <SheetHeader>
          <SheetTitle>轉錄設定</SheetTitle>
          <SheetDescription>
            多數情況可以維持預設值，只在需要指定語言或調整時間軸時修改。
          </SheetDescription>
        </SheetHeader>
        <div className="options-sheet-body">
          <FieldGroup>
          <Field>
            <FieldLabel>輸出語言</FieldLabel>
            <Select
              items={languageItems}
              value={options.language}
              onValueChange={(value) =>
                onChange({ ...options, language: String(value) })
              }
            >
              <SelectTrigger className="w-full" aria-label="輸出語言">
                <SelectValue>{language?.label ?? "自動偵測"}</SelectValue>
              </SelectTrigger>
              <SelectContent alignItemWithTrigger={false}>
                <SelectGroup>
                  {languageItems.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>

          <Field>
            <FieldLabel htmlFor="transcription-prompt">提示詞</FieldLabel>
            <Textarea
              id="transcription-prompt"
              value={options.prompt}
              placeholder="可留空"
              onChange={(event) =>
                onChange({ ...options, prompt: event.currentTarget.value })
              }
              />
            </Field>

          <FieldSet>
            <Field orientation="horizontal">
              <FieldContent>
                <FieldTitle id="write-srt-label">輸出 SRT 字幕</FieldTitle>
                <FieldDescription>使用分段時間戳產生字幕檔。</FieldDescription>
              </FieldContent>
              <Switch
                aria-labelledby="write-srt-label"
                checked={options.writeSrt}
                onCheckedChange={(checked) =>
                  onChange({ ...options, writeSrt: checked })
                }
              />
            </Field>
          </FieldSet>

          <details className="advanced-options">
            <summary>進階設定</summary>
            <div className="advanced-options-content">
              <FieldGroup className="grid gap-4 sm:grid-cols-3">
                <Field>
                  <FieldLabel htmlFor="segment-seconds">切段秒數</FieldLabel>
                  <Input
                    id="segment-seconds"
                    type="number"
                    min={1}
                    value={options.segmentSeconds}
                    onChange={(event) =>
                      onChange({
                        ...options,
                        segmentSeconds: toNumber(event.currentTarget.value, 30),
                      })
                    }
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="search-seconds">搜尋秒數</FieldLabel>
                  <Input
                    id="search-seconds"
                    type="number"
                    min={0.25}
                    step={0.25}
                    value={options.searchSeconds}
                    onChange={(event) =>
                      onChange({
                        ...options,
                        searchSeconds: toNumber(event.currentTarget.value, 3),
                      })
                    }
                  />
                </Field>
                <Field>
                  <FieldLabel htmlFor="worker-threads">執行緒</FieldLabel>
                  <Input
                    id="worker-threads"
                    type="number"
                    min={0}
                    value={options.threads}
                    onChange={(event) =>
                      onChange({
                        ...options,
                        threads: toNumber(event.currentTarget.value, 0),
                      })
                    }
                  />
                  <FieldDescription>0 代表自動</FieldDescription>
                </Field>
              </FieldGroup>

              <FieldSet>
                <FieldLegend variant="label">處理方式</FieldLegend>
                <Field orientation="horizontal">
                  <FieldContent>
                    <FieldTitle id="ffmpeg-convert-label">
                      使用 FFmpeg 轉檔
                    </FieldTitle>
                    <FieldDescription>
                      支援 mp3、m4a、影片與非標準 WAV。
                    </FieldDescription>
                  </FieldContent>
                  <Switch
                    aria-labelledby="ffmpeg-convert-label"
                    checked={options.convertWithFfmpeg}
                    onCheckedChange={(checked) =>
                      onChange({ ...options, convertWithFfmpeg: checked })
                    }
                  />
                </Field>
                <Field orientation="horizontal">
                  <FieldContent>
                    <FieldTitle id="skip-silence-label">跳過長靜音</FieldTitle>
                    <FieldDescription>
                      適合只需要文字、不重視原始時間軸時。
                    </FieldDescription>
                  </FieldContent>
                  <Switch
                    aria-labelledby="skip-silence-label"
                    checked={options.skipSilence}
                    onCheckedChange={(checked) =>
                      onChange({ ...options, skipSilence: checked })
                    }
                  />
                </Field>
                <Field orientation="horizontal">
                  <FieldContent>
                    <FieldTitle id="past-text-label">使用前文脈絡</FieldTitle>
                    <FieldDescription>
                      長音訊切段時讓前段文字參與解碼。
                    </FieldDescription>
                  </FieldContent>
                  <Switch
                    aria-labelledby="past-text-label"
                    checked={options.pastText}
                    onCheckedChange={(checked) =>
                      onChange({ ...options, pastText: checked })
                    }
                  />
                </Field>
              </FieldSet>
            </div>
          </details>
        </FieldGroup>
        </div>
        <SheetFooter>
          <SheetClose render={<Button />}>完成</SheetClose>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
