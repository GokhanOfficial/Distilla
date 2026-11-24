# syntax=docker/dockerfile:1

# Stage 1: Build Svelte 5 frontend + copy offline KaTeX/Highlight.js PDF assets
FROM node:22-bookworm-slim AS web-build
WORKDIR /app/web
COPY web/package*.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 2: Build single Rust binary with embedded frontend & PDF assets
FROM rust:1-bookworm AS rust-build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY --from=web-build /app/web/dist ./web/dist
COPY --from=web-build /app/assets/pdf ./assets/pdf
RUN cargo build --release --locked

# Stage 3: Minimal Debian runtime (no Node.js runtime, single compiled binary)
FROM debian:bookworm-slim AS runtime
WORKDIR /app

ENV PORT=3000 \
    APP_NAME=Distilla \
    RUST_LOG=distilla=info,tower_http=info \
    CHROMIUM_PATH=/usr/bin/chromium \
    FFMPEG_PATH=/usr/bin/ffmpeg \
    FFPROBE_PATH=/usr/bin/ffprobe \
    LIBREOFFICE_PATH=/usr/bin/soffice

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        chromium \
        ffmpeg \
        libreoffice-writer \
        libreoffice-calc \
        libreoffice-impress \
        fonts-liberation \
        fonts-noto \
        fonts-noto-color-emoji \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /usr/sbin/nologin distilla

COPY --from=rust-build /app/target/release/distilla /usr/local/bin/distilla

USER distilla

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -sf "http://127.0.0.1:${PORT:-3000}/api/health" > /dev/null || exit 1

ENTRYPOINT ["/usr/local/bin/distilla"]
