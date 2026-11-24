# Distilla

![Lisans](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/backend-Rust%20%2F%20Axum-orange.svg)
![Svelte](https://img.shields.io/badge/frontend-Svelte%205%20%2B%20TS-ff3e00.svg)

[English Documentation (README.md)](README.md)

**Distilla**; **PDF, Office belgeleri, görseller, ses ve video** dosyalarını yapay zeka ile temiz, düzenlenebilir **Markdown** formatına dönüştüren, uzun metinleri canlı akışla (streaming) özetleyen ve çevrimdışı KaTeX matematik + sözdizimi vurgulamasıyla **A4 PDF** üreten yüksek verimli bir web uygulamasıdır.

Minimum RAM tüketimi ve anında açılış için **tek derlenmiş Rust binary'si** (Axum + Tokio) içine gömülü **Virtual-DOM'suz Svelte 5** arayüzü ile geliştirilmiştir.

## Özellikler

- **Tek Dosya, Tek Aşamalı Streaming Akışı:** Dosya yerel olarak seçilir; "Dönüştürmeyi Başlat" ile canlı hız/ETA takibi eşliğinde sunucudaki public olmayan izole geçici klasöre stream edilir. Kırılgan ön-yükleme önbelleği yoktur; geçici dosyalar işlem bitiminde, hatada veya iptalde RAII (`Drop`) ile kesin olarak silinir.
- **Geniş Format Desteği:**
  - **Belgeler:** PDF, TXT, Markdown (`.md`)
  - **Office:** DOCX, DOC, PPTX, PPT, XLSX, XLS *(Birincil: LibreOffice headless; isteğe bağlı Google Drive API yedeği)*
  - **Görseller:** PNG, JPG/JPEG, WebP, GIF
  - **Ses:** MP3, WAV, OGG, M4A, AAC, OPUS, FLAC, OGA, WebM Audio
  - **Video:** MP4, MOV, AVI, MKV, WebM, 3GP, M4V, MPEG/MPG, WMV, FLV *(FFmpeg ile ses çıkarılıp MP3'e dönüştürülür)*
- **Akıllı FFmpeg Medya Sıkıştırma:** 100 MB altındaki native MP3/WAV/OGG dosyaları FFmpeg gerektirmeden doğrudan işlenir. Video ve 100 MB üzeri ses dosyaları dinamik bitrate hesabıyla (32–320 kbps, hedef 95 MB) MP3'e kodlanır.
- **Gerçek Zamanlı SSE Akışı ve Gerçek İptal:** FFmpeg ilerlemesi ve AI Markdown çıktısı Server-Sent Events ile anlık akar. İptal edildiğinde hem alt süreçler (FFmpeg/LibreOffice) hem de OpenAI HTTP akışı anında durdurulur.
- **AI Özetleyici:** Metinleri veya dönüştürülen belgeleri canlı önizlemeyle özetler, otomatik açıklayıcı dosya adı üretir.
- **MD → PDF Stüdyosu:** Çift yönlü senkronize kaydırmalı editör, DOMPurify XSS korumalı önizleme, kod bloklarını asla bozmayan AST tabanlı KaTeX matematik desteği, dış CDN'siz yerel assetler, gerçek çok sayfalı A4 kenar boşlukları ve UTF-16 PDF metadata.
- **Dinamik Yapılandırma:** `.env` üzerinden yapılandırılabilir AI model listesi (`AI_MODELS`), eksiksiz **TR / EN** dil desteği ve **Karanlık / Aydınlık** tema.

## Hızlı Başlangıç

### Yerel Derleme ve Çalıştırma

```bash
cp .env.example .env
# .env dosyasına OPENAI_API_KEY ve modellerinizi girin

# 1. Frontend'i derleyin ve çevrimdışı PDF assetlerini hazırlayın
cd web && npm install && npm run build && cd ..

# 2. Tek Rust binary'sini derleyip çalıştırın
cargo run --release
```

Tarayıcıda `http://localhost:3000` adresini açın.

### Docker Compose

```bash
cp .env.example .env
docker compose up --build -d
```

Final Docker imajında Node.js runtime'ı yer almaz; yalnızca derlenmiş Rust binary'si ile Chromium, FFmpeg, LibreOffice ve Noto fontları bulunur.

## Lisans

MIT — detaylar için [LICENSE](LICENSE) dosyasına bakın.
