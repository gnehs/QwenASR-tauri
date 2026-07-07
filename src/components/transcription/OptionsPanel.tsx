import { SlidersHorizontalIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSet,
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
import { languageItems } from "@/lib/app-constants";
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
            多數情況可以維持自動偵測，只在需要指定語言或輸出字幕時修改。
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

            <FieldSet>
              <Field orientation="horizontal">
                <FieldContent>
                  <FieldTitle id="write-srt-label">輸出 SRT 字幕</FieldTitle>
                  <FieldDescription>
                    字幕會依文字切段，並使用近似時間軸輸出。
                  </FieldDescription>
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
          </FieldGroup>
        </div>
        <SheetFooter>
          <SheetClose render={<Button />}>完成</SheetClose>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}
