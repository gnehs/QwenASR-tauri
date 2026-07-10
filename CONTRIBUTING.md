# 貢獻指南

感謝你想協助 QwenASR Studio。本文件說明如何在本機從原始碼執行、檢查與提交變更。

## 專案概況

- 套件管理器：`pnpm`（請勿混用 npm、yarn 或 bun 來修改依賴鎖定檔）
- 桌面框架：Tauri 2
- 前端：React、TypeScript、Vite、Tailwind CSS
- 後端：Rust
- 支援平台：搭載 Apple Silicon（M1 或更新）的 macOS 14 以上；Intel Mac 不支援

## 開始前

請先準備下列工具：

- Node.js LTS 與 `pnpm`
- Rust stable toolchain
- Xcode Command Line Tools
- FFmpeg
- Apple Silicon Mac（MLX backend 使用 Metal GPU）

Tauri 官方文件列出了 macOS 的開發前置需求；桌面目標只需要安裝 Xcode Command Line Tools：

```bash
xcode-select --install
```

如果尚未啟用 pnpm，可執行：

```bash
corepack enable
```

安裝 FFmpeg：

```bash
brew install ffmpeg
```

詳細安裝方式請參考 [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)。

## 從 GitHub 下載原始碼

不熟悉 Git 也沒關係：到專案首頁按 **Code**，選擇 **Download ZIP**，解壓縮後在終端機進入該資料夾。

若已經使用 Git：

```bash
git clone https://github.com/gnehs/QwenASR-tauri.git
cd QwenASR-tauri
```

安裝 JavaScript 依賴：

```bash
pnpm install
```

## 從原始碼執行

啟動完整桌面程式：

```bash
pnpm tauri:dev
```

若 shell 找不到 Cargo，請先載入 Rust 環境後再重試：

```bash
source "$HOME/.cargo/env"
```

只啟動前端開發伺服器：

```bash
pnpm dev
```

## 驗證變更

提交前，依變更範圍執行適合的檢查：

```bash
pnpm build
pnpm test:prepare-mlx
cd src-tauri && cargo test
```

也可先快速檢查 Rust 後端：

```bash
cd src-tauri && cargo check
```

目前專案沒有 JavaScript lint 或一般前端測試 script。新增這類流程時，請同時提供對應 script 與文件。

## 打包

建立 macOS 應用程式安裝檔：

```bash
pnpm tauri build
```

打包前會自動準備 MLX Metal library。產物會由 Tauri 產生於 `src-tauri/target/release/bundle/`。

## Rust smoke tests

需要實際音訊與已下載模型的手動驗證，可使用下列 examples：

```bash
cd src-tauri
cargo run --example asr_smoke -- [model_dir] <audio_path> [language]
cargo run --release --example vad_smoke -- <audio_path>
cargo run --release --example forced_alignment_smoke -- \
  <asr_model_dir> \
  <forced_aligner_model_dir> \
  <audio_path> \
  Chinese
```

省略 `asr_smoke` 的 `model_dir` 時，程式會嘗試使用應用程式資料目錄中的 `qwen3-asr-0.6b`。

## 提交變更

1. 先確認修改範圍與測試結果。
2. 一項變更聚焦一個目的，避免混入無關格式調整。
3. 提交訊息請使用英文 [Conventional Commits](https://www.conventionalcommits.org/) 格式，例如：`docs: rewrite user guide`。
4. 開啟 Pull Request 時，說明變更目的、驗證方式，以及使用者可見的影響。

請勿將模型、轉錄結果、影音檔、帳務資料、地址、電子郵件或其他敏感資訊提交到 repository。回報問題時，也請以可安全公開的範例與遮蔽後的紀錄說明。

## 參考文件

- [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri v2 CLI Reference](https://v2.tauri.app/reference/cli/)
- [pnpm install](https://pnpm.io/cli/install)
- [Vite: Building the App](https://vite.dev/guide/static-deploy.html#building-the-app)
