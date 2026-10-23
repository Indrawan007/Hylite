pub mod pdf;
mod pdf_build;

use pulldown_cmark::{html, Options, Parser};

/// Convert markdown → HTML body
pub fn to_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

/// Export markdown → file HTML lengkap (Catppuccin styled)
pub fn export_html(markdown: &str, path: &std::path::Path) -> anyhow::Result<()> {
    let page = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Hylite Export</title>
<style>
  :root {{ color-scheme: dark; }}
  body {{
    font-family: "Inter", -apple-system, system-ui, sans-serif;
    max-width: 820px; margin: 48px auto; padding: 0 24px;
    line-height: 1.7; background: #1e1e2e; color: #cdd6f4;
  }}
  h1,h2,h3,h4 {{ color: #b4befe; line-height: 1.3; margin-top: 1.6em; }}
  a {{ color: #89b4fa; }}
  code {{
    background: #313244; padding: 2px 6px; border-radius: 4px;
    font-family: "JetBrains Mono", monospace; font-size: 0.9em;
  }}
  pre {{
    background: #313244; padding: 16px; border-radius: 8px;
    overflow-x: auto; border: 1px solid #45475a;
  }}
  pre code {{ background: none; padding: 0; }}
  blockquote {{
    border-left: 4px solid #89b4fa; margin: 0;
    padding: 4px 16px; color: #a6adc8; background: #272738;
  }}
  table {{ border-collapse: collapse; width: 100%; }}
  th, td {{ border: 1px solid #45475a; padding: 8px 12px; text-align: left; }}
  th {{ background: #313244; }}
  hr {{ border: none; border-top: 1px solid #45475a; margin: 2em 0; }}
  img {{ max-width: 100%; border-radius: 8px; }}
</style>
</head>
<body>
{}
</body>
</html>"#,
        to_html(markdown)
    );

    crate::files::atomic_write(path, page.as_bytes())?;
    Ok(())
}

/// Export markdown → PDF with the active theme's colours.
///
/// The layout engine is `pdf.rs`: base-14 fonts, WinAnsi text, tables and code
/// blocks rendered as vector content. No font embedding, no images, no
/// compression, and no Unicode outside WinAnsi (`?` fallback).
pub fn export_pdf(
    markdown: &str,
    palette: pdf::Palette,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let blocks = crate::document::preview_model::parse(markdown);
    let doc = pdf_build::document(&blocks);
    let bytes = pdf::render(&doc, &palette);
    crate::files::atomic_write(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::{export_html, export_pdf, pdf, to_html};

    #[test]
    fn empty_markdown_produces_empty_body() {
        assert!(to_html("").is_empty());
    }

    #[test]
    fn renders_headings_and_inline_formatting() {
        let html = to_html("# Hylite\n\n**Bold**, *italic*, and `code`.\n");

        assert!(html.contains("<h1>Hylite</h1>"));
        assert!(html.contains("<strong>Bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<code>code</code>"));
    }

    #[test]
    fn renders_tables_and_task_lists() {
        let html = to_html("| Name |\n| --- |\n| Hylite |\n\n- [x] Ready\n- [ ] Later\n");

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Name</th>"));
        assert!(html.contains("<td>Hylite</td>"));
        assert!(html.contains("type=\"checkbox\""));
        assert!(html.contains("checked=\"\""));
    }

    #[test]
    fn escapes_code_without_treating_it_as_html() {
        let html = to_html("```html\n<script>alert('test')</script>\n```\n");

        assert!(html.contains("class=\"language-html\""));
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn preserves_unicode_text() {
        let html = to_html("# Catatan ✨\n\nBahasa Indonesia — 日本語.\n");

        assert!(html.contains("Catatan ✨"));
        assert!(html.contains("Bahasa Indonesia — 日本語."));
    }

    #[test]
    fn raw_html_is_currently_preserved_not_sanitized() {
        // This documents the current trust boundary, not a security guarantee.
        let html = to_html("<script>alert('test')</script>\n");

        assert!(html.contains("<script>alert('test')</script>"));
    }

    #[test]
    fn writes_a_complete_html_document() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catatan.html");

        export_html("# Catatan\n\nIsi dokumen.\n", &path).unwrap();
        let page = std::fs::read_to_string(path).unwrap();

        assert!(page.starts_with("<!DOCTYPE html>"));
        assert!(page.contains("<meta charset=\"utf-8\">"));
        assert!(page.contains("<style>"));
        assert!(page.contains("<h1>Catatan</h1>"));
        assert!(page.contains("<p>Isi dokumen.</p>"));
        assert!(page.ends_with("</html>"));
    }

    #[test]
    fn propagates_write_errors() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing").join("catatan.html");

        assert!(export_html("# Catatan", &path).is_err());
    }

    #[test]
    fn writes_a_valid_pdf_with_headings_tables_and_code() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("catatan.pdf");
        let markdown =
            "# Catatan\n\nIsi **dokumen**.\n\n| a | b |\n| - | - |\n| 1 | 2 |\n\n```\nkode\n```\n";
        export_pdf(markdown, pdf::Palette::default(), &path).unwrap();

        let bytes = std::fs::read(path).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("(Catatan) Tj"));
        assert!(text.contains("(Isi ) Tj") && text.contains("(dokumen) Tj"));
        assert!(text.contains("(a) Tj") && text.contains("(2) Tj"));
        assert!(text.contains("(kode) Tj"));
        assert!(text.contains("/Count 1"));
    }

    #[test]
    fn an_empty_document_still_writes_one_page() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("kosong.pdf");
        export_pdf("", pdf::Palette::default(), &path).unwrap();
        let text = String::from_utf8_lossy(&std::fs::read(path).unwrap()).to_string();
        assert!(text.contains("/Count 1"));
    }

    #[test]
    fn pdf_export_propagates_write_errors() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing").join("catatan.pdf");
        assert!(export_pdf("# Catatan", pdf::Palette::default(), &path).is_err());
    }
}
