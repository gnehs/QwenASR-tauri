# QwenASR Studio

QwenASR Studio 是在 Mac 上執行的本機語音轉文字工具。把音訊或影片拖進程式，選好語言與模型後，就能取得逐字稿；需要字幕時，也可以輸出 `.srt` 檔。

音訊、影片與轉錄結果都留在你的電腦上處理，不會上傳到本專案。

本程式基於 [alan890104/qwen3-asr-rs](https://github.com/alan890104/qwen3-asr-rs) 開發，整合成桌面應用介面。

## 你需要準備什麼

- 搭載 Apple Silicon（M1 或更新）並執行 macOS 14 或更新版本的 Mac
- 網路連線（第一次下載語音辨識模型時需要）
- [FFmpeg](https://ffmpeg.org/)：用來讀取與整理音訊

安裝 FFmpeg 最簡單的方式是使用 [Homebrew](https://brew.sh/)。如果你已經安裝 Homebrew，請開啟「終端機」並輸入：

```bash
brew install ffmpeg
```

完成後關閉並重新開啟 QwenASR Studio；在程式的「設定」頁面可以確認 FFmpeg 是否已經可用。

> QwenASR Studio 使用 Apple Silicon 的 MLX／Metal 加速，因此不支援 Intel 處理器的 Mac。

## 取得 QwenASR Studio

如果你已經安裝 Homebrew，可以在終端機執行以下指令安裝 QwenASR Studio：

```bash
brew install --cask gnehs/tap/qwenasr-studio
```

安裝完成後，就能從「應用程式」資料夾開啟 QwenASR Studio。

你也可以直接到 GitHub Releases 下載最新正式版：

1. 前往本專案的 [Releases 頁面](https://github.com/gnehs/QwenASR-tauri/releases/latest)。
2. 下載最新版中的 `qwenasr-studio-macos-arm64-app.dmg`。
3. 開啟 DMG，將裡面的 `QwenASR Studio.app` 拖到「應用程式」資料夾，然後開啟它。
4. 如果第一次開啟被系統擋住，請在終端機執行：

```bash
xattr -cr "/Applications/QwenASR Studio.app"
```

再重新開啟程式。

如果你想自行編譯，可參考 [CONTRIBUTING.md](CONTRIBUTING.md) 進行原始碼建置。

## 第一次轉錄

1. 開啟程式後，先到「設定」確認 FFmpeg 狀態正常。
2. 到「模型」下載 `Qwen3-ASR 0.6B`。這是一般情況下最適合開始使用的模型。
3. 回到「轉錄任務」，把音訊或影片拖進視窗，或按「新增任務」選取檔案。
4. 選擇模型與語言；不確定語言時可保留「自動偵測」。
5. 若要字幕，勾選 SRT 輸出；可另外選擇輸出資料夾。
6. 加入佇列，等待任務完成後在詳細資訊中查看逐字稿與 SRT 檔案位置。

## 可以處理的檔案

支援的輸入格式：

```text
wav, mp3, m4a, aac, flac, ogg, mp4, mov, mkv, webm
```

程式會先在本機暫時整理音訊，再進行辨識；處理結束後會清除暫存檔。

## 模型怎麼選

| 模型                     | 下載大小約 | 適合情況                                         |
| ------------------------ | ---------: | ------------------------------------------------ |
| Qwen3-ASR 0.6B           |     1.9 GB | 建議先從這個模型開始；適合大多數錄音與批次轉錄。 |
| Qwen3-ASR 1.7B           |     4.7 GB | 需要較高準確度，或錄音環境較複雜時使用。         |
| Qwen3 ForcedAligner 0.6B |     1.8 GB | 只在輸出 SRT 字幕時需要；會由程式自動提示下載。  |

模型會下載到 Mac 的應用程式資料夾，不會放進你從 GitHub 下載的專案資料夾。模型較大，請預留足夠磁碟空間並在下載時保持網路連線。

SRT 的精準時間軸由 ForcedAligner 產生，目前支援中文、英文、粵語、法文、德文、義大利文、日文、韓文、葡萄牙文、俄文與西班牙文；其他語言仍可輸出字幕，但時間軸是依片段長度估算。

## 常見問題

### 程式說找不到 FFmpeg

請依上方指示安裝 FFmpeg，然後完全結束並重新開啟程式。如果仍無法使用，請回到「設定」查看狀態與錯誤訊息。

### 模型下載很久或失敗

模型檔案很大，下載時間取決於網路速度。請確認網路連線穩定與磁碟空間足夠，再重新嘗試下載。

### SRT 檔在哪裡

有選擇輸出資料夾時，SRT 會儲存在該資料夾；未選擇時，會放在原始音訊或影片的同一個資料夾。

## 隱私與資料

- 轉錄在本機完成；音訊、影片、逐字稿與 SRT 不會由程式上傳到本專案。
- 模型由程式從 Hugging Face 下載到本機。
- 音訊和逐字稿可能含有敏感內容；回報問題時，請不要貼上真實檔案、完整逐字稿或可識別個人的檔名。

## 想協助改進？

謝謝！如果你想回報問題、提出功能建議，或修改程式，請閱讀 [貢獻指南](CONTRIBUTING.md)。其中包含從 GitHub 下載原始碼、設定開發環境、測試與送出變更的說明。

## 授權與致謝

- 本專案採用 [MIT License](LICENSE) 授權。
- QwenASR Studio 基於 [alan890104/qwen3-asr-rs](https://github.com/alan890104/qwen3-asr-rs) 開發，該專案亦為開源授權。
- 本程式所使用的模型來自 QwenLM 所提供的 Qwen3-ASR 家族。

## 相關連結

- [Qwen3-ASR GitHub](https://github.com/QwenLM/Qwen3-ASR)
- [Qwen3-ASR Hugging Face](https://huggingface.co/collections/Qwen/qwen3-asr)
- [FFmpeg](https://ffmpeg.org/)
