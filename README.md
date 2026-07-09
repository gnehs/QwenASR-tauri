# QwenASR Studio

QwenASR Studio 是一個以 Tauri 2、React、Vite 與 Rust 打造的 macOS 桌面轉錄工具。它把音訊或影片檔案留在本機處理，透過 Qwen3-ASR 模型產生逐字稿，並可輸出 SRT 字幕檔。

## 功能

- 單檔與多檔任務佇列轉錄
- 拖放檔案或透過原生檔案選擇器新增任務
- 內建 Qwen3-ASR 與 Qwen3 ForcedAligner 模型下載、刪除與下載進度顯示
- FFmpeg 音訊正規化：轉為 16 kHz、mono、PCM WAV 後再送入 ASR
- 內建 FireRedVAD ONNX 語音活動偵測，轉錄時跳過較長靜音片段
- 支援自動語言偵測與多語言/方言提示
- 繁體中文輸出可透過 OpenCC 做簡轉繁
- 可透過 Qwen3-ForcedAligner-0.6B 產生精準時間戳與 SRT 字幕
- 任務進度、ETA、完成通知、失敗重試與清除已結束任務

## 技術棧

- Desktop: [Tauri 2](https://v2.tauri.app/)
- Frontend: React 19、TypeScript、Vite 7、Tailwind CSS v4
- UI: shadcn/ui、Base UI、lucide-react、sonner
- Backend: Rust、qwen3_asr、hf-hub、tract-onnx、opencc-rs、hound、rustfft
- Package manager: pnpm

## 前置需求

- macOS
- Node.js 與 pnpm
- Rust toolchain
- FFmpeg

所有輸入檔目前都會先經過 FFmpeg 正規化，所以即使是 WAV 檔也需要系統可以找到 `ffmpeg`。

macOS 可用 Homebrew 安裝 FFmpeg：

```bash
brew install ffmpeg
```

如果 Tauri 找不到 `cargo`，請確認 Rust 已安裝，並讓目前 shell 載入 Cargo 環境：

```bash
source "$HOME/.cargo/env"
```

專案的 Tauri scripts 也會在執行時把 `$HOME/.cargo/bin` 加到 `PATH` 前面，降低 GUI shell 或非互動 shell 找不到 Cargo 的機率。

## 開發

安裝依賴：

```bash
pnpm install
```

啟動 Tauri 開發模式：

```bash
pnpm tauri:dev
```

也可以直接把參數傳給 Tauri CLI：

```bash
pnpm tauri dev
```

只啟動 Vite 前端開發伺服器：

```bash
pnpm dev
```

Tauri 設定會在開發模式中啟動 `pnpm dev`，並連到 `http://127.0.0.1:1420`。

## 建置與檢查

建置前端：

```bash
pnpm build
```

預覽前端 production build：

```bash
pnpm preview
```

檢查 Rust 後端：

```bash
cd src-tauri
cargo check
```

執行 Rust 測試：

```bash
cd src-tauri
cargo test
```

打包 Tauri app：

```bash
pnpm tauri build
```

目前 `package.json` 沒有 `lint` 或 JavaScript `test` script；若要新增檢查流程，請先補上對應 script 再寫入 CI 或文件。

## 模型

目前支援的模型：

| ID | Hugging Face repo | 大小提示 | 說明 |
| --- | --- | --- | --- |
| `qwen3-asr-0.6b` | [`Qwen/Qwen3-ASR-0.6B`](https://huggingface.co/Qwen/Qwen3-ASR-0.6B) | ~1.2 GB | 預設推薦，適合大多數單次與批次轉錄 |
| `qwen3-asr-1.7b` | [`Qwen/Qwen3-ASR-1.7B`](https://huggingface.co/Qwen/Qwen3-ASR-1.7B) | ~3.5 GB | 較高準確度，適合重要錄音或較複雜環境 |
| `qwen3-forced-aligner-0.6b` | [`Qwen/Qwen3-ForcedAligner-0.6B`](https://huggingface.co/Qwen/Qwen3-ForcedAligner-0.6B) | ~1.8 GB | 將逐字稿與音訊對齊，產生字／詞級時間戳；不會出現在轉錄模型選單 |

模型會由 app 下載到系統應用程式資料目錄，不會存進 repository。macOS 通常會落在：

```text
~/Library/Application Support/QwenASR Studio/models/<model-id>
```

模型下載流程會從 Hugging Face 取得必要檔案，並在本機用 `vocab.json` 與 `merges.txt` 產生 `tokenizer.json`。

## 支援輸入與輸出

支援選取的輸入副檔名：

```text
wav, mp3, m4a, aac, flac, ogg, mp4, mov, mkv, webm
```

轉錄前會先輸出暫存 WAV 到系統暫存目錄下的 `qwenasr-tauri` 子目錄，處理完成後由程式清理。

若啟用 SRT 輸出：

- app 會自動確認並下載 `Qwen3 ForcedAligner 0.6B`
- 每個 FireRedVAD 片段完成 ASR 後，會以原始逐字稿執行 ForcedAligner，再依句子分組成可閱讀的字幕 cue
- ForcedAligner 支援 Chinese、English、Cantonese、French、German、Italian、Japanese、Korean、Portuguese、Russian、Spanish；其他語言仍會輸出以片段長度估算的時間軸
- 有指定輸出資料夾時，SRT 會寫入該資料夾
- 未指定輸出資料夾時，SRT 會寫在來源檔案同層

## 使用流程

1. 進入「設定」確認 FFmpeg 狀態。
2. 下載 `Qwen3-ASR 0.6B` 或 `Qwen3-ASR 1.7B`。
3. 回到「轉錄任務」，拖放檔案或點「新增任務」。
4. 選擇模型、語言、是否輸出 SRT，以及輸出資料夾。
5. 加入佇列後等待任務逐一完成。
6. 在任務詳細資訊中查看逐字稿、SRT 路徑與錯誤訊息。

## Rust examples

ASR smoke test：

```bash
cd src-tauri
cargo run --example asr_smoke -- [model_dir] <audio_path> [language]
```

如果省略 `model_dir`，example 會嘗試使用 app data 內的 `qwen3-asr-0.6b`。

VAD smoke test：

```bash
cd src-tauri
cargo run --release --example vad_smoke -- <audio_path>
```

ASR＋ForcedAligner smoke test：

```bash
cd src-tauri
cargo run --release --example forced_alignment_smoke -- \
  <asr_model_dir> \
  <forced_aligner_model_dir> \
  <audio_path> \
  Chinese
```

## 資料與隱私

- 音訊、影片、逐字稿與 SRT 可能包含敏感內容，請不要把真實使用者資料、帳務資料、地址或電子郵件提交到 repository。
- 模型與輸出檔案不應提交進 git；模型預設位於 app data 目錄。
- README 中的路徑均為通用範例，沒有包含個人資料。
- FFmpeg 錯誤摘要會遮蔽 input/output 路徑，但仍應避免在 issue 或 log 中貼上敏感檔名與內容。

## 參考文件

- [GitHub Docs: About READMEs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-readmes)
- [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri v2 CLI Reference](https://v2.tauri.app/reference/cli/)
- [pnpm install](https://pnpm.io/cli/install)
- [Vite: Building the App](https://vite.dev/guide/static-deploy.html#building-the-app)
- [Qwen3-ASR 官方 repository](https://github.com/QwenLM/Qwen3-ASR)
- [Qwen3-ForcedAligner-0.6B 官方模型卡](https://huggingface.co/Qwen/Qwen3-ForcedAligner-0.6B)
