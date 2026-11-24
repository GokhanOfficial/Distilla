export type Lang = 'tr' | 'en';

const translations = {
  tr: {
    'app.tagline': 'AI Doküman & Medya → Markdown',
    'nav.converter': 'Metinleştirici',
    'nav.summarizer': 'Özetleyici',
    'nav.editor': 'MD → PDF',

    'converter.title': 'Dosyayı Markdown’a Dönüştür',
    'converter.subtitle': 'PDF, Office, görsel, ses veya video dosyanızı seçin; yapay zeka ile temiz Markdown’a dönüştürün.',
    'converter.dropTitle': 'Dosyanızı buraya sürükleyip bırakın',
    'converter.dropSubtitle': 'veya bilgisayarınızdan tek bir dosya seçin',
    'converter.maxSize': 'Maksimum boyut',
    'converter.changeFile': 'Değiştir',
    'converter.removeFile': 'Kaldır',
    'converter.advanced': 'Gelişmiş Seçenekler',
    'converter.modelLabel': 'Yapay Zeka Modeli',
    'converter.start': 'Dönüştürmeyi Başlat',
    'converter.uploading': 'Dosya Yükleniyor...',
    'converter.converting': 'Dönüştürülüyor...',
    'converter.cancel': 'İptal Et',
    'converter.cancelling': 'İptal Ediliyor...',
    'converter.uploaded': 'Yüklenen',
    'converter.speed': 'Hız',
    'converter.eta': 'Kalan',
    'converter.elapsed': 'Geçen',
    'converter.attempt': 'Deneme',
    'converter.duration': 'Süre',
    'converter.bitrate': 'Bitrate',
    'converter.aiLive': 'AI çıktısı akıyor',
    'converter.chars': 'karakter',
    'converter.words': 'kelime',
    'converter.editorPlaceholder': 'Dönüştürülen Markdown içeriği burada görünecek...',
    'converter.previewEmpty': 'Önizleme burada görünecek...',
    'converter.summarizeAction': 'Özetle',

    'progress.queued': 'Sırada bekleniyor...',
    'progress.preparing': 'Dosya hazırlanıyor...',
    'progress.officeConverting': 'Office belgesi PDF’e dönüştürülüyor...',
    'progress.mediaEncoding': 'FFmpeg ile ses işleniyor...',
    'progress.aiConverting': 'Yapay zeka ile metinleştiriliyor...',
    'progress.summarizing': 'Yapay zeka ile özetleniyor...',
    'progress.completed': 'Tamamlandı',
    'progress.failed': 'İşlem başarısız',
    'progress.cancelled': 'İşlem iptal edildi',

    'tabs.markdown': 'Markdown',
    'tabs.preview': 'Önizleme',
    'tabs.split': 'Bölünmüş',

    'actions.copy': 'Kopyala',
    'actions.downloadMd': '.md İndir',
    'actions.downloadPdf': '.pdf İndir',
    'actions.clear': 'Temizle',

    'summarizer.title': 'AI Metin Özetleyici',
    'summarizer.subtitle': 'Uzun metinleri veya Markdown belgelerini yapılandırılmış, akademik özetlere dönüştürün.',
    'summarizer.inputTitle': 'Kaynak Metin',
    'summarizer.outputTitle': 'Özet Çıktısı',
    'summarizer.inputPlaceholder': 'Özetlenecek metni buraya yapıştırın...',
    'summarizer.outputPlaceholder': 'Özet burada canlı olarak görünecek...',
    'summarizer.start': 'Özetle',
    'summarizer.summarizing': 'Özetleniyor...',

    'editor.title': 'MD → PDF Stüdyosu',
    'editor.subtitle': 'Markdown yazın, KaTeX matematik ve kod vurgulamasıyla canlı önizleyin ve A4 PDF olarak indirin.',
    'editor.markdownTitle': 'Markdown Editörü',
    'editor.previewTitle': 'Canlı Önizleme',
    'editor.placeholder': '# Başlık\n\nMarkdown içeriğinizi buraya yazın...\n\n## Matematik Örneği\n\n$$E = mc^2$$\n\n## Kod Örneği\n\n```ts\nconst greeting: string = "Merhaba Distilla";\nconsole.log(greeting);\n```',

    'toast.copied': 'Panoya kopyalandı',
    'toast.downloadStarted': 'İndirme başlatıldı',
    'toast.generatingTitle': 'Dosya adı üretiliyor...',
    'toast.generatingPdf': 'PDF oluşturuluyor...',
    'toast.pdfReady': 'PDF hazır ve indirildi',
    'toast.conversionComplete': 'Dönüştürme tamamlandı',
    'toast.conversionCancelled': 'İşlem iptal edildi',
    'toast.summaryComplete': 'Özetleme tamamlandı',
    'toast.summaryCancelled': 'Özetleme iptal edildi',
    'toast.unsupportedFormat': 'Desteklenmeyen dosya formatı',
    'toast.fileTooLarge': 'Dosya maksimum yükleme boyutunu aşıyor',
    'toast.noFile': 'Lütfen önce bir dosya seçin',
    'toast.emptyContent': 'İçerik boş',
    'toast.serverUnreachable': 'Sunucu bağlantısı kurulamadı',
    'toast.aiNotConfigured': 'Sunucuda OPENAI_API_KEY yapılandırılmamış',

    'theme.dark': 'Karanlık Tema',
    'theme.light': 'Aydınlık Tema',
  },
  en: {
    'app.tagline': 'AI Document & Media → Markdown',
    'nav.converter': 'Converter',
    'nav.summarizer': 'Summarizer',
    'nav.editor': 'MD → PDF',

    'converter.title': 'Convert File to Markdown',
    'converter.subtitle': 'Select a PDF, Office document, image, audio, or video file to distill into clean Markdown with AI.',
    'converter.dropTitle': 'Drag and drop your file here',
    'converter.dropSubtitle': 'or click to select a single file from your device',
    'converter.maxSize': 'Max size',
    'converter.changeFile': 'Change',
    'converter.removeFile': 'Remove',
    'converter.advanced': 'Advanced Options',
    'converter.modelLabel': 'AI Model',
    'converter.start': 'Start Conversion',
    'converter.uploading': 'Uploading File...',
    'converter.converting': 'Converting...',
    'converter.cancel': 'Cancel',
    'converter.cancelling': 'Cancelling...',
    'converter.uploaded': 'Uploaded',
    'converter.speed': 'Speed',
    'converter.eta': 'ETA',
    'converter.elapsed': 'Elapsed',
    'converter.attempt': 'Attempt',
    'converter.duration': 'Duration',
    'converter.bitrate': 'Bitrate',
    'converter.aiLive': 'Streaming AI output',
    'converter.chars': 'chars',
    'converter.words': 'words',
    'converter.editorPlaceholder': 'Converted Markdown content will appear here...',
    'converter.previewEmpty': 'Preview will appear here...',
    'converter.summarizeAction': 'Summarize',

    'progress.queued': 'Queued...',
    'progress.preparing': 'Preparing file...',
    'progress.officeConverting': 'Converting Office document to PDF...',
    'progress.mediaEncoding': 'Processing audio with FFmpeg...',
    'progress.aiConverting': 'Distilling content with AI...',
    'progress.summarizing': 'Summarizing with AI...',
    'progress.completed': 'Completed',
    'progress.failed': 'Failed',
    'progress.cancelled': 'Cancelled',

    'tabs.markdown': 'Markdown',
    'tabs.preview': 'Preview',
    'tabs.split': 'Split View',

    'actions.copy': 'Copy',
    'actions.downloadMd': 'Download .md',
    'actions.downloadPdf': 'Download .pdf',
    'actions.clear': 'Clear',

    'summarizer.title': 'AI Text Summarizer',
    'summarizer.subtitle': 'Turn long texts or Markdown notes into crisp, structured summaries.',
    'summarizer.inputTitle': 'Source Text',
    'summarizer.outputTitle': 'Summary Output',
    'summarizer.inputPlaceholder': 'Paste text to summarize here...',
    'summarizer.outputPlaceholder': 'Your live summary will appear here...',
    'summarizer.start': 'Summarize',
    'summarizer.summarizing': 'Summarizing...',

    'editor.title': 'MD → PDF Studio',
    'editor.subtitle': 'Write Markdown with live KaTeX math and syntax highlighting, then export a clean A4 PDF.',
    'editor.markdownTitle': 'Markdown Editor',
    'editor.previewTitle': 'Live Preview',
    'editor.placeholder': '# Heading\n\nWrite your Markdown content here...\n\n## Math Example\n\n$$E = mc^2$$\n\n## Code Example\n\n```ts\nconst greeting: string = "Hello Distilla";\nconsole.log(greeting);\n```',

    'toast.copied': 'Copied to clipboard',
    'toast.downloadStarted': 'Download started',
    'toast.generatingTitle': 'Generating filename...',
    'toast.generatingPdf': 'Generating PDF...',
    'toast.pdfReady': 'PDF ready and downloaded',
    'toast.conversionComplete': 'Conversion complete',
    'toast.conversionCancelled': 'Operation cancelled',
    'toast.summaryComplete': 'Summarization complete',
    'toast.summaryCancelled': 'Summarization cancelled',
    'toast.unsupportedFormat': 'Unsupported file format',
    'toast.fileTooLarge': 'File exceeds maximum upload size',
    'toast.noFile': 'Please select a file first',
    'toast.emptyContent': 'Content is empty',
    'toast.serverUnreachable': 'Could not connect to server',
    'toast.aiNotConfigured': 'OPENAI_API_KEY is not configured on the server',

    'theme.dark': 'Dark Theme',
    'theme.light': 'Light Theme',
  },
} as const;

export type TranslationKey = keyof (typeof translations)['tr'];

const STORAGE_KEY = 'distilla-lang';

function getInitialLang(): Lang {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'tr' || saved === 'en') return saved;
  }
  return 'tr';
}

class I18nStore {
  lang = $state<Lang>(getInitialLang());

  constructor() {
    if (typeof document !== 'undefined') {
      document.documentElement.lang = this.lang;
    }
  }

  setLang(next: Lang) {
    this.lang = next;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, next);
    }
    if (typeof document !== 'undefined') {
      document.documentElement.lang = next;
    }
  }

  t(key: TranslationKey | string): string {
    const dict = translations[this.lang] as Record<string, string>;
    const fallback = translations.tr as Record<string, string>;
    return dict[key] ?? fallback[key] ?? key;
  }
}

export const i18n = new I18nStore();
