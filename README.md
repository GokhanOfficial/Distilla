# Distilla

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/backend-Rust%20%2F%20Axum-orange.svg)
![Svelte](https://img.shields.io/badge/frontend-Svelte%205%20%2B%20TS-ff3e00.svg)

[Türkçe Dokümantasyon (README_TR.md)](README_TR.md)

**Distilla** is a high-efficiency, low-memory AI workspace that distills **PDF, Office documents, images, audio, and video** into clean, editable **Markdown**, summarizes long texts with live streaming, and exports publication-grade **A4 PDFs** with offline KaTeX math and syntax highlighting.

Built as a **single compiled Rust binary** (Axum + Tokio) embedding a **Virtual-DOM-free Svelte 5** frontend for minimal RAM usage and instant startup.

## Features

- **Single-File, Single-Step Streaming Pipeline:** Select a file locally, then upload with real-time progress (speed & ETA) directly to an isolated non-public temp directory. No fragile pre-upload cache, no orphan files — temporary directories are unconditionally cleaned up via RAII (`Drop`) on completion, error, or cancellation.
- **Broad Format Support:**
  - **Documents:** PDF, TXT, Markdown (`.md`)
  - **Office:** DOCX, DOC, PPTX, PPT, XLSX, XLS *(LibreOffice headless primary; optional Google Drive API fallback)*
  - **Images:** PNG, JPG/JPEG, WebP, GIF
  - **Audio:** MP3, WAV, OGG, M4A, AAC, OPUS, FLAC, OGA, WebM Audio
  - **Video:** MP4, MOV, AVI, MKV, WebM, 3GP, M4V, MPEG/MPG, WMV, FLV *(audio extracted and encoded via FFmpeg)*
- **Smart FFmpeg Media Compression:** Native MP3/WAV/OGG under 100 MB bypass FFmpeg completely. Videos and oversized audio files are automatically encoded to MP3 with dynamic bitrate calculation (32–320 kbps targeting 95 MB) and real-time progress + process-group cancellation.
- **Real-Time SSE Streaming & True Cancellation:** Live Server-Sent Events stream FFmpeg progress and AI Markdown deltas to the UI; cancelling aborts both child processes and the upstream OpenAI HTTP stream immediately.
- **AI Summarizer:** Turn notes or converted Markdown into structured summaries with live preview, cancellation, and auto-generated filenames.
- **MD → PDF Studio:** Split Markdown editor with synchronized scrolling, XSS-safe DOMPurify preview, AST-level KaTeX math (never corrupts code blocks), offline local KaTeX & Highlight.js assets, proper multi-page A4 margins, concurrency limiting, and UTF-16 PDF metadata.
- **Dynamic Configuration:** Configure AI models via `.env` (`AI_MODELS`), full **TR / EN** localization, and **Dark / Light** theme support.

## Quick Start

### Prerequisites
- **Rust** (`cargo`) and **Node.js 20+** (for local builds)
- **OpenAI-compatible API key** (OpenAI, OpenRouter, Gemini proxy, etc.)
- **FFmpeg & FFprobe** (for video/audio conversion)
- **Chromium / Google Chrome** (for Markdown → PDF export)
- *(Optional)* **LibreOffice (`soffice`)** or Google Drive OAuth credentials (for Office → PDF conversion)

### Local Build & Run

```bash
cp .env.example .env
# Edit .env with your OPENAI_API_KEY and preferred models

# 1. Build frontend & copy offline PDF assets
cd web && npm install && npm run build && cd ..

# 2. Build and run the single Rust binary
cargo run --release
```

Open `http://localhost:3000`.

### Docker Compose

```bash
cp .env.example .env
docker compose up --build -d
```

The final Docker image contains **only** the compiled Rust binary plus Chromium, FFmpeg, LibreOffice, and Noto fonts — no Node.js runtime in production.

## Environment Variables

See [`.env.example`](.env.example) for all options, including `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `AI_MODELS`, `AI_DEFAULT_MODEL`, `AI_FILENAME_MODEL`, `PDF_INPUT_MODE`, `MAX_UPLOAD_SIZE_MB`, `MEDIA_*`, `LIBREOFFICE_PATH`, `CHROMIUM_PATH`, and `PDF_MAX_CONCURRENCY`.

## License

MIT — see [LICENSE](LICENSE).
