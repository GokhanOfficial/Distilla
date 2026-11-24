use crate::config::{find_in_path, PdfConfig};
use anyhow::{anyhow, Context, Result};
use lopdf::{Dictionary, Document, Object, StringFormat};
use pulldown_cmark::{html, Event, Options, Parser};
use rust_embed::Embed;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Embed)]
#[folder = "assets/pdf/"]
struct PdfAssets;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PdfMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Clone)]
pub struct PdfService {
    config: PdfConfig,
    app_name: String,
    app_url: Option<String>,
    temp_root: PathBuf,
    semaphore: Arc<Semaphore>,
}

impl PdfService {
    pub fn new(
        config: PdfConfig,
        app_name: String,
        app_url: Option<String>,
        temp_root: PathBuf,
    ) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        Self {
            config,
            app_name,
            app_url,
            temp_root,
            semaphore,
        }
    }

    pub fn is_available(&self) -> bool {
        self.config
            .chromium_path
            .as_deref()
            .is_some_and(find_in_path)
    }

    pub async fn generate_pdf(&self, markdown: &str) -> Result<Vec<u8>> {
        let chromium = self
            .config
            .chromium_path
            .as_deref()
            .filter(|p| find_in_path(p))
            .ok_or_else(|| {
                anyhow!("Chromium/Chrome binary not found. Install chromium or set CHROMIUM_PATH.")
            })?;

        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| anyhow!("PDF queue closed"))?;

        let render_dir = self.temp_root.join(format!("pdf-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&render_dir).await?;

        let result = self.render_in_dir(chromium, markdown, &render_dir).await;
        let _ = tokio::fs::remove_dir_all(&render_dir).await;
        result
    }

    async fn render_in_dir(
        &self,
        chromium: &str,
        markdown: &str,
        render_dir: &Path,
    ) -> Result<Vec<u8>> {
        // Materialize embedded offline KaTeX + Highlight.js assets
        for file_path in PdfAssets::iter() {
            if let Some(file) = PdfAssets::get(file_path.as_ref()) {
                let target = render_dir.join(file_path.as_ref());
                if let Some(parent) = target.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&target, file.data.as_ref()).await?;
            }
        }

        let body_html = render_markdown_to_html(markdown);
        let full_html = build_pdf_html(&body_html);

        let html_path = render_dir.join("document.html");
        let pdf_path = render_dir.join("document.pdf");
        tokio::fs::write(&html_path, full_html).await?;

        let file_url = format!("file://{}", html_path.display());

        let mut cmd = Command::new(chromium);
        cmd.args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--disable-setuid-sandbox",
            "--disable-dev-shm-usage",
            "--disable-extensions",
            "--no-pdf-header-footer",
            "--virtual-time-budget=5000",
            &format!("--print-to-pdf={}", pdf_path.display()),
            &file_url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

        let output = tokio::time::timeout(Duration::from_secs(45), cmd.output())
            .await
            .map_err(|_| anyhow!("PDF generation timed out"))?
            .context("Failed to execute Chromium for PDF generation")?;

        if !output.status.success() || !pdf_path.exists() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Chromium PDF generation failed: {}",
                stderr.chars().take(400).collect::<String>()
            ));
        }

        let raw_pdf = tokio::fs::read(&pdf_path)
            .await
            .context("Failed to read generated PDF")?;

        let metadata = extract_metadata(markdown);
        match apply_pdf_metadata(&raw_pdf, &metadata, &self.app_name, self.app_url.as_deref()) {
            Ok(updated) => Ok(updated),
            Err(err) => {
                tracing::warn!("Could not update PDF metadata, returning raw PDF: {}", err);
                Ok(raw_pdf)
            }
        }
    }
}

/// Converts Markdown to HTML using pulldown-cmark's AST with built-in math parsing.
/// Code blocks (` ``` `) and inline code (`` ` ``) are NEVER touched by math replacement.
pub fn render_markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(markdown, options).map(|event| match event {
        Event::InlineMath(tex) => {
            let escaped = escape_html(&tex);
            Event::Html(format!(r#"<span class="math-inline">\({}\)</span>"#, escaped).into())
        }
        Event::DisplayMath(tex) => {
            let escaped = escape_html(&tex);
            Event::Html(format!(r#"<div class="math-display">\[{}\]</div>"#, escaped).into())
        }
        other => other,
    });

    let mut html_output = String::with_capacity(markdown.len() * 2);
    html::push_html(&mut html_output, parser);
    html_output
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn build_pdf_html(body_html: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="tr">
<head>
<meta charset="UTF-8">
<link rel="stylesheet" href="katex.min.css">
<link rel="stylesheet" href="github.min.css">
<style>
@page {{
  size: A4;
  margin: 18mm 16mm;
}}
* {{
  box-sizing: border-box;
}}
body {{
  font-family: 'Inter', 'Noto Sans', 'Liberation Sans', 'Segoe UI', system-ui, -apple-system, sans-serif;
  font-size: 11pt;
  line-height: 1.65;
  color: #1e293b;
  margin: 0;
  padding: 0;
}}
h1, h2, h3, h4 {{
  break-after: avoid;
  page-break-after: avoid;
  color: #0f172a;
}}
h1 {{
  font-size: 1.9em;
  border-bottom: 2px solid #e2e8f0;
  padding-bottom: 0.35rem;
  margin: 0 0 1rem 0;
}}
h2 {{
  font-size: 1.4em;
  margin: 1.4rem 0 0.65rem 0;
}}
h3 {{
  font-size: 1.15em;
  margin: 1.1rem 0 0.45rem 0;
}}
p {{
  margin: 0 0 0.85rem 0;
}}
pre {{
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  padding: 0.85em 1em;
  margin: 0.9em 0;
  overflow-x: hidden;
  white-space: pre-wrap;
  word-wrap: break-word;
  break-inside: avoid;
}}
code {{
  font-family: 'JetBrains Mono', 'Fira Code', 'Liberation Mono', Consolas, monospace;
  font-size: 0.88em;
}}
:not(pre) > code {{
  background: #f1f5f9;
  color: #be185d;
  padding: 2px 5px;
  border-radius: 4px;
}}
table {{
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
  font-size: 0.95em;
}}
tr {{
  break-inside: avoid;
}}
th, td {{
  border: 1px solid #cbd5e1;
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
}}
th {{
  background-color: #f1f5f9;
  font-weight: 600;
  color: #0f172a;
}}
blockquote {{
  border-left: 4px solid #6366f1;
  background: #f8fafc;
  padding: 0.65em 1em;
  margin: 0.9em 0;
  color: #475569;
  border-radius: 0 6px 6px 0;
}}
ul, ol {{
  padding-left: 1.6em;
  margin: 0.4em 0 0.9em 0;
}}
li {{
  margin: 0.25em 0;
}}
img {{
  max-width: 100%;
  height: auto;
  border-radius: 4px;
}}
a {{
  color: #4f46e5;
  text-decoration: none;
}}
hr {{
  border: none;
  border-top: 1px solid #e2e8f0;
  margin: 1.5rem 0;
}}
.math-display {{
  margin: 1em 0;
  text-align: center;
  overflow-x: auto;
}}
</style>
</head>
<body>
<article id="doc-root">{body}</article>
<script src="highlight.min.js"></script>
<script src="katex.min.js"></script>
<script src="auto-render.min.js"></script>
<script>
(function() {{
  try {{
    if (window.hljs) {{
      document.querySelectorAll('pre code').forEach(function(block) {{
        window.hljs.highlightElement(block);
      }});
    }}
    if (window.renderMathInElement) {{
      window.renderMathInElement(document.getElementById('doc-root'), {{
        delimiters: [
          {{ left: '$$', right: '$$', display: true }},
          {{ left: '\\[', right: '\\]', display: true }},
          {{ left: '\\(', right: '\\)', display: false }}
        ],
        ignoredTags: ['script', 'noscript', 'style', 'textarea', 'pre', 'code'],
        throwOnError: false,
        strict: false
      }});
    }}
  }} catch (e) {{
    console.error(e);
  }}
}})();
</script>
</body>
</html>"#,
        body = body_html
    )
}

pub fn extract_metadata(markdown: &str) -> PdfMetadata {
    let mut meta = PdfMetadata::default();
    let mut found_h1 = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if !found_h1 {
            if let Some(title) = trimmed.strip_prefix("# ") {
                let t = title.trim();
                if !t.is_empty() {
                    meta.title = Some(t.chars().take(150).collect());
                    found_h1 = true;
                    continue;
                }
            }
        } else if meta.subject.is_none()
            && !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("```")
        {
            meta.subject = Some(trimmed.chars().take(200).collect());
        }

        if let Some(kw) = trimmed
            .strip_prefix("## ")
            .or_else(|| trimmed.strip_prefix("### "))
        {
            let kw_clean = kw.trim().to_string();
            if !kw_clean.is_empty()
                && meta.keywords.len() < 10
                && !meta.keywords.contains(&kw_clean)
            {
                meta.keywords.push(kw_clean);
            }
        }
    }

    meta
}

fn apply_pdf_metadata(
    pdf_bytes: &[u8],
    metadata: &PdfMetadata,
    app_name: &str,
    app_url: Option<&str>,
) -> Result<Vec<u8>> {
    let mut doc = Document::load_mem(pdf_bytes).context("lopdf failed to load PDF")?;

    let info_id = if let Ok(Object::Reference(id)) = doc.trailer.get(b"Info") {
        *id
    } else {
        let id = doc.add_object(Dictionary::new());
        doc.trailer.set("Info", Object::Reference(id));
        id
    };

    if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(info_id) {
        if let Some(ref title) = metadata.title {
            dict.set("Title", pdf_utf16_string(title));
        }
        if let Some(ref subject) = metadata.subject {
            dict.set("Subject", pdf_utf16_string(subject));
        }
        if !metadata.keywords.is_empty() {
            dict.set("Keywords", pdf_utf16_string(&metadata.keywords.join(", ")));
        }
        dict.set("Producer", pdf_utf16_string(app_name));
        dict.set("Creator", pdf_utf16_string(app_name));
        if let Some(url) = app_url {
            dict.set("Website", pdf_utf16_string(url));
        }
    }

    let mut out = Vec::with_capacity(pdf_bytes.len() + 512);
    doc.save_to(&mut out).context("lopdf failed to save PDF")?;
    Ok(out)
}

/// Encodes text as UTF-16BE with BOM so Turkish/Unicode metadata renders properly in all PDF readers.
fn pdf_utf16_string(text: &str) -> Object {
    let mut bytes = vec![0xFE, 0xFF];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::String(bytes, StringFormat::Hexadecimal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_code_blocks_while_rendering_math() {
        let md = "# Title\n\nInline $x_1 < 5$ math.\n\n```bash\necho $HOME\nif [ $a < 10 ]; then echo ok; fi\n```\n\n$$E = mc^2$$\n";
        let html = render_markdown_to_html(md);
        assert!(html.contains(r#"<span class="math-inline">\(x_1 &lt; 5\)</span>"#));
        assert!(html.contains(r#"<div class="math-display">\[E = mc^2\]</div>"#));
        assert!(html.contains("echo $HOME"));
        assert!(!html.contains(r#"\(HOME"#));
    }

    #[test]
    fn extracts_markdown_metadata() {
        let md = "# Makine Öğrenmesi\n\nBu belge temel kavramları açıklar.\n\n## Regresyon\n### Lineer Regresyon\n";
        let meta = extract_metadata(md);
        assert_eq!(meta.title.as_deref(), Some("Makine Öğrenmesi"));
        assert_eq!(
            meta.subject.as_deref(),
            Some("Bu belge temel kavramları açıklar.")
        );
        assert_eq!(meta.keywords, vec!["Regresyon", "Lineer Regresyon"]);
    }
}
