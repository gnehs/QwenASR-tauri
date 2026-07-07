import type { OptionsState, SelectOption } from "@/types/transcription";

export const languageItems: SelectOption[] = [
  { label: "自動偵測", value: "auto" },
  { label: "繁體中文 / Chinese", value: "Chinese" },
  { label: "English", value: "English" },
  { label: "Japanese", value: "Japanese" },
  { label: "Korean", value: "Korean" },
];

export const audioFilters = [
  {
    name: "Audio and video",
    extensions: [
      "wav",
      "mp3",
      "m4a",
      "aac",
      "flac",
      "ogg",
      "mp4",
      "mov",
      "mkv",
      "webm",
    ],
  },
];

export const defaultOptions: OptionsState = {
  language: "auto",
  writeSrt: true,
};
