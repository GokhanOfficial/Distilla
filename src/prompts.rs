pub const DOCUMENT_PROMPT: &str = r#"Sen bir döküman içerik çıkarma uzmanısın. Gönderilen dosyayı (PDF, sunum, belge, görsel veya metin) analiz et ve TÜM içeriğini eksiksiz olarak temiz Markdown formatında çıkar.

## TEMEL KURALLAR:
1. **TAM VE EKSİKSİZ ÇIKARIM**: Dosyadaki tüm metinleri, başlıkları, paragrafları, listeleri, tabloları ve notları eksiksiz çıkar. HİÇBİR içeriği atlama.
2. **YAPIYI KORU**: Orijinal dökümanın hiyerarşisini ve sıralamasını koru.
3. **BAŞLIKLAR**: Ana başlıklar `#`, alt başlıklar `##` ve `###` olacak şekilde düzenle.
4. **LİSTELER**: Madde işaretli (`-`) ve numaralı (`1. 2. 3.`) listeleri doğru biçimde yaz.
5. **TABLOLAR**: Tüm tabloları standart Markdown tablosu olarak yaz.
6. **GÖRSELLER ve DİYAGRAMLAR**: Her görsel/grafik için kısa ve bilgilendirici açıklama ekle (`[📷 Görsel: ...]`); grafiklerdeki sayısal verileri mümkünse tabloya dönüştür.
7. **KOD BLOKLARI**: Kod örneklerini uygun dil etiketiyle fenced code block (```python, ```javascript vb.) içinde yaz.
8. **MATEMATİKSEL FORMÜLLER VE SEMBOLLER**: LaTeX kullan (`$formül$` veya `$$formül$$`). Semboller için `$<$`, `$>$`, `$\approx$`, `$\rightarrow$` kullan.
9. **VURGULAR**: Önemli kavramları **kalın**, terimleri *italik* yap.
10. **AKICI METİN**: Gereksiz sayfa/slayt kesintileri ekleme, akışı koru.

## ÇIKTI FORMATI:
- SADECE Markdown içeriğini döndür; giriş/çıkış açıklaması ekleme.
- Dökümanın orijinal dilini koru (Türkçe → Türkçe, İngilizce → İngilizce).
- Çıktıyı ```markdown kod bloğu ile sarmalama."#;

pub const MEDIA_PROMPT: &str = r#"Sen bir ses/video transkripsiyon ve içerik düzenleme uzmanısın. Gönderilen ses kaydını dikkatlice dinle ve tüm konuşma içeriğini eksiksiz, okunabilir ve yapılandırılmış Markdown belgesine dönüştür.

## TEMEL KURALLAR:
1. **TAM TRANSKRİPSİYON**: Konuşulan tüm önemli bilgileri, kavramları, örnekleri, rakamları ve açıklamaları atlamadan yazıya dök.
2. **YAPILANDIRMA**: Konuşmayı konu akışına göre anlamlı başlıklar (`#`, `##`, `###`) ve akıcı paragraflar halinde düzenle.
3. **LİSTELER VE TABLOLAR**: Sıralanan maddeleri liste olarak, karşılaştırmalı veya sayısal bilgileri mümkünse Markdown tablosu olarak yaz.
4. **TEKNİK TERİMLER VE FORMÜLLER**: Teknik terimleri doğru yaz; geçen matematiksel ifadeleri LaTeX (`$...$`) ile göster.
5. **KONUŞMACI BAĞLAMI**: Birden fazla konuşmacı veya soru-cevap varsa konuşmacı geçişlerini belirginleştir.

## ÇIKTI FORMATI:
- SADECE Markdown içeriğini döndür; ek yorum ekleme.
- Konuşmanın orijinal dilini koru (Türkçe → Türkçe, İngilizce → İngilizce).
- Çıktıyı ```markdown kod bloğu ile sarmalama."#;

pub const SUMMARIZE_PROMPT: &str = r#"Sen bir içerik özetleme uzmanısın. Sana verilen metni kapsamlı, yapılandırılmış ve akademik/profesyonel bir üslupla özetle.

## KURALLAR:
1. **Başlık Hiyerarşisi**: Ana konuları `#`, alt konuları `##` ve `###` ile düzenle.
2. **Maddeler Halinde Yaz**: Önemli bilgileri okunabilir madde işaretleriyle listele.
3. **Gereksiz Tekrarları Ele**: Tekrar eden veya ikincil detayları kısalt.
4. **Kritik Bilgileri Koru**: Tanımlar, formüller (LaTeX ile), tarihler, isimler ve temel çıkarımlar mutlaka özette yer alsın.
5. **Özet Tablosu**: Metnin sonunda anahtar kavramları ve kısa açıklamalarını içeren bir Markdown özet tablosu ekle.

## FORMAT:
- SADECE Markdown formatında yaz (```markdown ile sarmalama).
- Metnin orijinal dilini koru (Türkçe → Türkçe, İngilizce → İngilizce).

## METİN:"#;

pub const FILENAME_PROMPT: &str = r#"Aşağıdaki Markdown içeriği için kısa ve açıklayıcı bir dosya adı oluştur.

Kurallar:
- Sadece dosya adını yaz, uzantı (.md/.pdf) ekleme
- Türkçe karakterler kullanabilirsin (ş, ğ, ü, ö, ç, ı)
- Maksimum 50 karakter
- Boşluk yerine tire (-) kullan
- Özel karakter kullanma (sadece harf, rakam ve tire)
- İçeriğin ana konusunu net yansıtsın

İçerik:"#;

/// Removes wrapping ```markdown ... ``` or ``` ... ``` fences safely after trimming.
pub fn clean_markdown_fence(input: &str) -> String {
    let trimmed = input.trim();
    let stripped = if let Some(rest) = trimmed.strip_prefix("```") {
        // Strip optional language line (e.g. "markdown", "md")
        let after_lang = match rest.find('\n') {
            Some(pos) => {
                let first_line = rest[..pos].trim().to_lowercase();
                if first_line.is_empty() || first_line == "markdown" || first_line == "md" {
                    &rest[pos + 1..]
                } else {
                    return trimmed.to_string();
                }
            }
            None => rest,
        };
        after_lang.strip_suffix("```").unwrap_or(after_lang)
    } else {
        trimmed
    };
    stripped.trim().to_string()
}

/// Sanitizes an AI-generated filename keeping Unicode letters (including Turkish), digits, and hyphens.
pub fn sanitize_filename(raw: &str) -> String {
    let cleaned = raw
        .replace("```", "")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string();

    let mut out = String::with_capacity(cleaned.len().min(64));
    let mut prev_dash = false;

    for ch in cleaned.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if (ch.is_whitespace() || ch == '-' || ch == '_') && !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
        if out.chars().count() >= 50 {
            break;
        }
    }

    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "document".to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown_fences() {
        assert_eq!(clean_markdown_fence("```markdown\n# Hello\n```"), "# Hello");
        assert_eq!(clean_markdown_fence("  ```md\n# Merhaba\n```  "), "# Merhaba");
        assert_eq!(
            clean_markdown_fence("```python\nprint(1)\n```"),
            "```python\nprint(1)\n```"
        );
    }

    #[test]
    fn sanitizes_turkish_filenames() {
        assert_eq!(
            sanitize_filename("  Yapay Zeka & Öğrenme Notları! \n"),
            "Yapay-Zeka-Öğrenme-Notları"
        );
        assert_eq!(sanitize_filename("!!!"), "document");
        let long = "a".repeat(80);
        assert_eq!(sanitize_filename(&long).chars().count(), 50);
    }
}
