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

    std::fs::write(path, page)?;
    Ok(())
}
