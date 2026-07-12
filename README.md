# QwenASR Studio

QwenASR Studio is a local speech-to-text desktop app for macOS. Drag and drop audio or video, choose a language and model, and get transcripts. When needed, you can also export `.srt` subtitle files.

> README in Traditional Chinese: [README.zh.md](README.zh.md)

Audio, video, and transcription results are processed locally on your machine and are not uploaded to this project.

This app is built on [alan890104/qwen3-asr-rs](https://github.com/alan890104/qwen3-asr-rs) and wrapped as a desktop application.

## Requirements

- Apple Silicon Mac (M1 or newer) running macOS 14 or later
- Network connection (required when downloading models for the first time)
- [FFmpeg](https://ffmpeg.org/) for reading and preparing audio

The easiest way to install FFmpeg is with [Homebrew](https://brew.sh/). If Homebrew is already installed, run:

```bash
brew install ffmpeg
```

After installation, restart QwenASR Studio and check FFmpeg status in **Settings**.

> QwenASR Studio uses Apple Silicon MLX/Metal acceleration and does not support Intel Macs.

## Get QwenASR Studio

Download the latest release from GitHub Releases and install it directly:

1. Open this repository's [Releases page](https://github.com/gnehs/QwenASR-tauri/releases/latest).
2. Download `qwenasr-studio-macos-arm64-app.dmg` from the latest release assets.
3. Open the DMG and drag `QwenASR Studio.app` into your Applications folder, then open it.
4. If macOS blocks first launch, open Terminal and run:

```bash
xattr -cr "/Applications/QwenASR Studio.app"
```

Then reopen the app.

If you want to build the app yourself, see [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions.

## First transcription

1. Open the app and verify FFmpeg is working in **Settings**.
2. In **Models**, download `Qwen3-ASR 0.6B` (recommended starting model).
3. Go to **Transcription Tasks**, then drag-and-drop your audio/video file or click **Add Task**.
4. Choose a model and language. If unsure, keep **Auto detect** selected.
5. Check **Export SRT** when subtitle output is needed, and optionally choose an output folder.
6. Add to queue and wait for completion. Check transcript and SRT paths in task details.

## Supported files

Supported input formats:

```text
wav, mp3, m4a, aac, flac, ogg, mp4, mov, mkv, webm
```

The app first remuxes audio locally before transcription, and cleans temporary files after each job.

## Which model to use

| Model                    | Approx. download size | Suggested use                                                                  |
| ------------------------ | --------------------: | ------------------------------------------------------------------------------ |
| Qwen3-ASR 0.6B           |                1.9 GB | Recommended for first-time users; suitable for most recordings and batch jobs. |
| Qwen3-ASR 1.7B           |                4.7 GB | Better accuracy for noisy or complex audio.                                    |
| Qwen3 ForcedAligner 0.6B |                1.8 GB | Required only when exporting SRT; app will suggest when needed.                |

Downloaded models are stored in your Mac application data folder, not in the checked-out project directory. These are large files; ensure enough disk space and stable connectivity when downloading.

SRT timing precision comes from ForcedAligner and currently supports Chinese, English, Cantonese, French, German, Italian, Japanese, Korean, Portuguese, Russian, and Spanish. Other languages still support subtitle export, but timings are estimated from segment lengths.

## Troubleshooting

### FFmpeg not found

Install FFmpeg as described above, then fully quit and relaunch the app. If the issue remains, check status and error messages in **Settings**.

### Model download is slow or fails

Model files are large and download speed depends on your network. Confirm a stable connection and sufficient disk space, then retry.

### Where is the SRT file?

If you choose an output folder, SRT is saved there. Otherwise, it is placed in the same folder as the original audio/video file.

## Privacy

- Transcription happens locally. Audio/video, transcripts, and SRT files are not uploaded.
- Models are downloaded to your local machine directly from Hugging Face.
- Files may contain sensitive content; do not share raw files or full transcripts in issue reports.

## Contributing

Thank you for your interest. For bug reports, feature ideas, or code changes, check the [contributing guide](CONTRIBUTING.md) for instructions on cloning, setup, testing, and submitting updates.

## License and Credits

- This project is licensed under the [MIT License](LICENSE).
- QwenASR Studio is built on [alan890104/qwen3-asr-rs](https://github.com/alan890104/qwen3-asr-rs), which is also open source.
- The app uses models from the Qwen3-ASR family provided by QwenLM.

## Related Links

- [Qwen3-ASR GitHub](https://github.com/QwenLM/Qwen3-ASR)
- [Qwen3-ASR Hugging Face](https://huggingface.co/collections/Qwen/qwen3-asr)
- [FFmpeg](https://ffmpeg.org/)
