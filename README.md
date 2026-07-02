# QwenASR Studio

macOS desktop app for local audio transcription with Tauri, React, Tailwind, shadcn/ui Base UI, and [`huanglizhuo/QwenASR`](https://github.com/huanglizhuo/QwenASR).

## Features

- Single-file transcription
- Batch transcription queue
- Built-in QwenASR model downloader
- Download progress, file progress, and current transfer speed
- SRT subtitle export from timestamped segments
- FFmpeg-based audio/video conversion to 16 kHz mono PCM
- Native file and folder picker through Tauri dialog plugin

## Requirements

- pnpm
- Rust toolchain
- macOS
- FFmpeg for non-WAV files and audio normalization

Install FFmpeg on macOS:

```bash
brew install ffmpeg
```

## Development

Install dependencies:

```bash
pnpm install
```

Run the Tauri app:

```bash
pnpm tauri dev
```

If Tauri cannot find `cargo`, make sure Rust is installed and the Cargo bin
directory is available:

```bash
source "$HOME/.cargo/env"
```

The project script also prepends `$HOME/.cargo/bin` when running Tauri so
`pnpm tauri dev` works in shells that do not source Cargo automatically.

Build the frontend:

```bash
pnpm build
```

Check the Rust backend:

```bash
cd src-tauri
cargo check
```

## Models

The app can download the supported QwenASR models from Hugging Face:

- `Qwen/Qwen3-ASR-0.6B`
- `Qwen/Qwen3-ASR-1.7B`

Models are stored in the application data directory, not in this repository.
