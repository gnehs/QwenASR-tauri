import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";

import type { OptionsState, SelectOption } from "@/types/transcription";

export const languageItems: SelectOption[] = [
  { label: "自動偵測", value: "auto" },
  { label: "繁體中文 / Chinese", value: "Chinese" },
  { label: "簡體中文 / Simplified Chinese", value: "Chinese (Simplified)" },
  { label: "English", value: "English" },
  { label: "粵語 / Cantonese", value: "Cantonese" },
  { label: "阿拉伯文 / Arabic", value: "Arabic" },
  { label: "德文 / German", value: "German" },
  { label: "法文 / French", value: "French" },
  { label: "西班牙文 / Spanish", value: "Spanish" },
  { label: "葡萄牙文 / Portuguese", value: "Portuguese" },
  { label: "印尼文 / Indonesian", value: "Indonesian" },
  { label: "義大利文 / Italian", value: "Italian" },
  { label: "韓文 / Korean", value: "Korean" },
  { label: "俄文 / Russian", value: "Russian" },
  { label: "泰文 / Thai", value: "Thai" },
  { label: "越南文 / Vietnamese", value: "Vietnamese" },
  { label: "日文 / Japanese", value: "Japanese" },
  { label: "土耳其文 / Turkish", value: "Turkish" },
  { label: "印地文 / Hindi", value: "Hindi" },
  { label: "馬來文 / Malay", value: "Malay" },
  { label: "荷蘭文 / Dutch", value: "Dutch" },
  { label: "瑞典文 / Swedish", value: "Swedish" },
  { label: "丹麥文 / Danish", value: "Danish" },
  { label: "芬蘭文 / Finnish", value: "Finnish" },
  { label: "波蘭文 / Polish", value: "Polish" },
  { label: "捷克文 / Czech", value: "Czech" },
  { label: "菲律賓文 / Filipino", value: "Filipino" },
  { label: "波斯文 / Persian", value: "Persian" },
  { label: "希臘文 / Greek", value: "Greek" },
  { label: "匈牙利文 / Hungarian", value: "Hungarian" },
  { label: "馬其頓文 / Macedonian", value: "Macedonian" },
  { label: "羅馬尼亞文 / Romanian", value: "Romanian" },
  { label: "安徽方言 / Anhui", value: "Anhui" },
  { label: "東北方言 / Dongbei", value: "Dongbei" },
  { label: "福建方言 / Fujian", value: "Fujian" },
  { label: "甘肅方言 / Gansu", value: "Gansu" },
  { label: "貴州方言 / Guizhou", value: "Guizhou" },
  { label: "河北方言 / Hebei", value: "Hebei" },
  { label: "河南方言 / Henan", value: "Henan" },
  { label: "湖北方言 / Hubei", value: "Hubei" },
  { label: "湖南方言 / Hunan", value: "Hunan" },
  { label: "江西方言 / Jiangxi", value: "Jiangxi" },
  { label: "寧夏方言 / Ningxia", value: "Ningxia" },
  { label: "山東方言 / Shandong", value: "Shandong" },
  { label: "陝西方言 / Shaanxi", value: "Shaanxi" },
  { label: "山西方言 / Shanxi", value: "Shanxi" },
  { label: "四川方言 / Sichuan", value: "Sichuan" },
  { label: "天津方言 / Tianjin", value: "Tianjin" },
  { label: "雲南方言 / Yunnan", value: "Yunnan" },
  { label: "浙江方言 / Zhejiang", value: "Zhejiang" },
  {
    label: "粵語（香港口音） / Cantonese (Hong Kong accent)",
    value: "Cantonese (Hong Kong accent)",
  },
  {
    label: "粵語（廣東口音） / Cantonese (Guangdong accent)",
    value: "Cantonese (Guangdong accent)",
  },
  { label: "吳語 / Wu language", value: "Wu language" },
  { label: "閩南語 / Minnan language", value: "Minnan language" },
];

export const languageLabelMessages: Record<string, MessageDescriptor> = {
  auto: msg`自動偵測`,
  Chinese: msg`繁體中文 / Chinese`,
  "Chinese (Simplified)": msg`簡體中文 / Simplified Chinese`,
  English: msg`English`,
  Cantonese: msg`粵語 / Cantonese`,
  Arabic: msg`阿拉伯文 / Arabic`,
  German: msg`德文 / German`,
  French: msg`法文 / French`,
  Spanish: msg`西班牙文 / Spanish`,
  Portuguese: msg`葡萄牙文 / Portuguese`,
  Indonesian: msg`印尼文 / Indonesian`,
  Italian: msg`義大利文 / Italian`,
  Korean: msg`韓文 / Korean`,
  Russian: msg`俄文 / Russian`,
  Thai: msg`泰文 / Thai`,
  Vietnamese: msg`越南文 / Vietnamese`,
  Japanese: msg`日文 / Japanese`,
  Turkish: msg`土耳其文 / Turkish`,
  Hindi: msg`印地文 / Hindi`,
  Malay: msg`馬來文 / Malay`,
  Dutch: msg`荷蘭文 / Dutch`,
  Swedish: msg`瑞典文 / Swedish`,
  Danish: msg`丹麥文 / Danish`,
  Finnish: msg`芬蘭文 / Finnish`,
  Polish: msg`波蘭文 / Polish`,
  Czech: msg`捷克文 / Czech`,
  Filipino: msg`菲律賓文 / Filipino`,
  Persian: msg`波斯文 / Persian`,
  Greek: msg`希臘文 / Greek`,
  Hungarian: msg`匈牙利文 / Hungarian`,
  Macedonian: msg`馬其頓文 / Macedonian`,
  Romanian: msg`羅馬尼亞文 / Romanian`,
  Anhui: msg`安徽方言 / Anhui`,
  Dongbei: msg`東北方言 / Dongbei`,
  Fujian: msg`福建方言 / Fujian`,
  Gansu: msg`甘肅方言 / Gansu`,
  Guizhou: msg`貴州方言 / Guizhou`,
  Hebei: msg`河北方言 / Hebei`,
  Henan: msg`河南方言 / Henan`,
  Hubei: msg`湖北方言 / Hubei`,
  Hunan: msg`湖南方言 / Hunan`,
  Jiangxi: msg`江西方言 / Jiangxi`,
  Ningxia: msg`寧夏方言 / Ningxia`,
  Shandong: msg`山東方言 / Shandong`,
  Shaanxi: msg`陝西方言 / Shaanxi`,
  Shanxi: msg`山西方言 / Shanxi`,
  Sichuan: msg`四川方言 / Sichuan`,
  Tianjin: msg`天津方言 / Tianjin`,
  Yunnan: msg`雲南方言 / Yunnan`,
  Zhejiang: msg`浙江方言 / Zhejiang`,
  "Cantonese (Hong Kong accent)": msg`粵語（香港口音） / Cantonese (Hong Kong accent)`,
  "Cantonese (Guangdong accent)": msg`粵語（廣東口音） / Cantonese (Guangdong accent)`,
  "Wu language": msg`吳語 / Wu language`,
  "Minnan language": msg`閩南語 / Minnan language`,
};

const dialectLanguageValues = new Set([
  "Anhui",
  "Dongbei",
  "Fujian",
  "Gansu",
  "Guizhou",
  "Hebei",
  "Henan",
  "Hubei",
  "Hunan",
  "Jiangxi",
  "Ningxia",
  "Shandong",
  "Shaanxi",
  "Shanxi",
  "Sichuan",
  "Tianjin",
  "Yunnan",
  "Zhejiang",
  "Cantonese (Hong Kong accent)",
  "Cantonese (Guangdong accent)",
  "Wu language",
  "Minnan language",
]);

const languageCategoryMessages = {
  major: msg`主要語言`,
  dialect: msg`方言`,
};

export function localizeLanguageItems(
  translate: (message: MessageDescriptor) => string,
): SelectOption[] {
  return languageItems.map((item) => {
    const message = languageLabelMessages[item.value];
    return message ? { ...item, label: translate(message) } : item;
  });
}

export function localizeLanguageGroups(
  translate: (message: MessageDescriptor) => string,
) {
  const items = localizeLanguageItems(translate);

  return [
    {
      id: "major",
      label: translate(languageCategoryMessages.major),
      items: items.filter((item) => !dialectLanguageValues.has(item.value)),
    },
    {
      id: "dialect",
      label: translate(languageCategoryMessages.dialect),
      items: items.filter((item) => dialectLanguageValues.has(item.value)),
    },
  ] as const;
}

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
  segmentByPunctuation: true,
};
