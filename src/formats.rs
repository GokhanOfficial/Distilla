use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    Text,
    Image,
    Pdf,
    Office { google_mime: &'static str },
    DirectAudio { format: &'static str },
    EncodeAudio,
    EncodeVideo,
}

#[derive(Debug, Clone)]
pub struct FormatSpec {
    pub extensions: &'static [&'static str],
    pub primary_ext: &'static str,
    pub canonical_mime: &'static str,
    pub mimes: &'static [&'static str],
    pub name: &'static str,
    pub category: &'static str,
    pub kind: FormatKind,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicFormatInfo {
    pub ext: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub mime: &'static str,
}

pub static SUPPORTED_FORMATS: &[FormatSpec] = &[
    // Documents
    FormatSpec {
        extensions: &[".pdf"],
        primary_ext: ".pdf",
        canonical_mime: "application/pdf",
        mimes: &["application/pdf"],
        name: "PDF",
        category: "document",
        kind: FormatKind::Pdf,
    },
    FormatSpec {
        extensions: &[".txt"],
        primary_ext: ".txt",
        canonical_mime: "text/plain",
        mimes: &["text/plain"],
        name: "TXT",
        category: "document",
        kind: FormatKind::Text,
    },
    FormatSpec {
        extensions: &[".md", ".markdown"],
        primary_ext: ".md",
        canonical_mime: "text/markdown",
        mimes: &["text/markdown", "text/x-markdown"],
        name: "Markdown",
        category: "document",
        kind: FormatKind::Text,
    },
    // Office
    FormatSpec {
        extensions: &[".docx"],
        primary_ext: ".docx",
        canonical_mime: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        mimes: &["application/vnd.openxmlformats-officedocument.wordprocessingml.document"],
        name: "DOCX",
        category: "office",
        kind: FormatKind::Office {
            google_mime: "application/vnd.google-apps.document",
        },
    },
    FormatSpec {
        extensions: &[".doc"],
        primary_ext: ".doc",
        canonical_mime: "application/msword",
        mimes: &["application/msword"],
        name: "DOC",
        category: "office",
        kind: FormatKind::Office {
            google_mime: "application/vnd.google-apps.document",
        },
    },
    FormatSpec {
        extensions: &[".pptx"],
        primary_ext: ".pptx",
        canonical_mime: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        mimes: &["application/vnd.openxmlformats-officedocument.presentationml.presentation"],
        name: "PPTX",
        category: "office",
        kind: FormatKind::Office {
            google_mime: "application/vnd.google-apps.presentation",
        },
    },
    FormatSpec {
        extensions: &[".ppt"],
        primary_ext: ".ppt",
        canonical_mime: "application/vnd.ms-powerpoint",
        mimes: &["application/vnd.ms-powerpoint"],
        name: "PPT",
        category: "office",
        kind: FormatKind::Office {
            google_mime: "application/vnd.google-apps.presentation",
        },
    },
    FormatSpec {
        extensions: &[".xlsx"],
        primary_ext: ".xlsx",
        canonical_mime: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        mimes: &["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"],
        name: "XLSX",
        category: "office",
        kind: FormatKind::Office {
            google_mime: "application/vnd.google-apps.spreadsheet",
        },
    },
    FormatSpec {
        extensions: &[".xls"],
        primary_ext: ".xls",
        canonical_mime: "application/vnd.ms-excel",
        mimes: &["application/vnd.ms-excel"],
        name: "XLS",
        category: "office",
        kind: FormatKind::Office {
            google_mime: "application/vnd.google-apps.spreadsheet",
        },
    },
    // Images
    FormatSpec {
        extensions: &[".png"],
        primary_ext: ".png",
        canonical_mime: "image/png",
        mimes: &["image/png"],
        name: "PNG",
        category: "image",
        kind: FormatKind::Image,
    },
    FormatSpec {
        extensions: &[".jpg", ".jpeg"],
        primary_ext: ".jpg",
        canonical_mime: "image/jpeg",
        mimes: &["image/jpeg", "image/jpg"],
        name: "JPG",
        category: "image",
        kind: FormatKind::Image,
    },
    FormatSpec {
        extensions: &[".webp"],
        primary_ext: ".webp",
        canonical_mime: "image/webp",
        mimes: &["image/webp"],
        name: "WebP",
        category: "image",
        kind: FormatKind::Image,
    },
    FormatSpec {
        extensions: &[".gif"],
        primary_ext: ".gif",
        canonical_mime: "image/gif",
        mimes: &["image/gif"],
        name: "GIF",
        category: "image",
        kind: FormatKind::Image,
    },
    // Direct audio
    FormatSpec {
        extensions: &[".mp3"],
        primary_ext: ".mp3",
        canonical_mime: "audio/mpeg",
        mimes: &["audio/mpeg", "audio/mp3"],
        name: "MP3",
        category: "audio",
        kind: FormatKind::DirectAudio { format: "mp3" },
    },
    FormatSpec {
        extensions: &[".wav"],
        primary_ext: ".wav",
        canonical_mime: "audio/wav",
        mimes: &["audio/wav", "audio/x-wav", "audio/wave"],
        name: "WAV",
        category: "audio",
        kind: FormatKind::DirectAudio { format: "wav" },
    },
    FormatSpec {
        extensions: &[".ogg"],
        primary_ext: ".ogg",
        canonical_mime: "audio/ogg",
        mimes: &["audio/ogg"],
        name: "OGG",
        category: "audio",
        kind: FormatKind::DirectAudio { format: "ogg" },
    },
    // Encoded audio
    FormatSpec {
        extensions: &[".m4a"],
        primary_ext: ".m4a",
        canonical_mime: "audio/mp4",
        mimes: &["audio/mp4", "audio/x-m4a"],
        name: "M4A",
        category: "audio",
        kind: FormatKind::EncodeAudio,
    },
    FormatSpec {
        extensions: &[".aac"],
        primary_ext: ".aac",
        canonical_mime: "audio/aac",
        mimes: &["audio/aac", "audio/aacp", "audio/x-aac"],
        name: "AAC",
        category: "audio",
        kind: FormatKind::EncodeAudio,
    },
    FormatSpec {
        extensions: &[".opus"],
        primary_ext: ".opus",
        canonical_mime: "audio/opus",
        mimes: &["audio/opus", "audio/x-opus"],
        name: "OPUS",
        category: "audio",
        kind: FormatKind::EncodeAudio,
    },
    FormatSpec {
        extensions: &[".flac"],
        primary_ext: ".flac",
        canonical_mime: "audio/flac",
        mimes: &["audio/flac", "audio/x-flac"],
        name: "FLAC",
        category: "audio",
        kind: FormatKind::EncodeAudio,
    },
    FormatSpec {
        extensions: &[".oga"],
        primary_ext: ".oga",
        canonical_mime: "audio/oga",
        mimes: &["audio/oga"],
        name: "OGA",
        category: "audio",
        kind: FormatKind::EncodeAudio,
    },
    FormatSpec {
        extensions: &[".weba"],
        primary_ext: ".weba",
        canonical_mime: "audio/webm",
        mimes: &["audio/webm"],
        name: "WebM Audio",
        category: "audio",
        kind: FormatKind::EncodeAudio,
    },
    // Video -> audio extracted to MP3
    FormatSpec {
        extensions: &[".mp4"],
        primary_ext: ".mp4",
        canonical_mime: "video/mp4",
        mimes: &["video/mp4"],
        name: "MP4",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".mov"],
        primary_ext: ".mov",
        canonical_mime: "video/quicktime",
        mimes: &["video/quicktime"],
        name: "MOV",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".avi"],
        primary_ext: ".avi",
        canonical_mime: "video/x-msvideo",
        mimes: &["video/avi", "video/x-msvideo"],
        name: "AVI",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".mkv"],
        primary_ext: ".mkv",
        canonical_mime: "video/x-matroska",
        mimes: &["video/x-matroska"],
        name: "MKV",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".webm"],
        primary_ext: ".webm",
        canonical_mime: "video/webm",
        mimes: &["video/webm"],
        name: "WebM",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".3gp"],
        primary_ext: ".3gp",
        canonical_mime: "video/3gpp",
        mimes: &["video/3gpp"],
        name: "3GP",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".m4v"],
        primary_ext: ".m4v",
        canonical_mime: "video/x-m4v",
        mimes: &["video/x-m4v"],
        name: "M4V",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".mpeg", ".mpg"],
        primary_ext: ".mpeg",
        canonical_mime: "video/mpeg",
        mimes: &["video/mpeg"],
        name: "MPEG",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".wmv"],
        primary_ext: ".wmv",
        canonical_mime: "video/x-ms-wmv",
        mimes: &["video/x-ms-wmv"],
        name: "WMV",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
    FormatSpec {
        extensions: &[".flv"],
        primary_ext: ".flv",
        canonical_mime: "video/x-flv",
        mimes: &["video/x-flv"],
        name: "FLV",
        category: "video",
        kind: FormatKind::EncodeVideo,
    },
];

/// Detect format by MIME first (ignoring empty / octet-stream), then fall back to extension.
pub fn detect_format(filename: &str, mime_type: &str) -> Option<&'static FormatSpec> {
    let clean_mime = mime_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();

    // Extension takes priority for ambiguous generic MIME types (like text/plain for .md)
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()));

    if let Some(ref ext_str) = ext {
        if ext_str == ".md" || ext_str == ".markdown" {
            if let Some(spec) = SUPPORTED_FORMATS
                .iter()
                .find(|f| f.extensions.contains(&ext_str.as_str()))
            {
                return Some(spec);
            }
        }
    }

    if !clean_mime.is_empty() && clean_mime != "application/octet-stream" {
        if let Some(spec) = SUPPORTED_FORMATS
            .iter()
            .find(|f| f.mimes.contains(&clean_mime.as_str()))
        {
            return Some(spec);
        }
    }

    if let Some(ref ext_str) = ext {
        if let Some(spec) = SUPPORTED_FORMATS
            .iter()
            .find(|f| f.extensions.contains(&ext_str.as_str()))
        {
            return Some(spec);
        }
    }

    None
}

pub fn all_extensions() -> Vec<&'static str> {
    SUPPORTED_FORMATS
        .iter()
        .flat_map(|f| f.extensions.iter().copied())
        .collect()
}

pub fn public_formats() -> Vec<PublicFormatInfo> {
    SUPPORTED_FORMATS
        .iter()
        .map(|f| PublicFormatInfo {
            ext: f.primary_ext,
            name: f.name,
            category: f.category,
            mime: f.canonical_mime,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mime_and_extension_fallback() {
        assert_eq!(detect_format("notes.md", "").unwrap().kind, FormatKind::Text);
        assert_eq!(
            detect_format("notes.md", "text/plain").unwrap().name,
            "Markdown"
        );
        assert_eq!(
            detect_format("sheet.xls", "application/octet-stream")
                .unwrap()
                .name,
            "XLS"
        );
        assert_eq!(
            detect_format("clip.mkv", "").unwrap().kind,
            FormatKind::EncodeVideo
        );
        assert_eq!(
            detect_format("lecture.mp3", "audio/mpeg").unwrap().kind,
            FormatKind::DirectAudio { format: "mp3" }
        );
        assert!(detect_format("malware.exe", "application/x-msdownload").is_none());
    }
}
