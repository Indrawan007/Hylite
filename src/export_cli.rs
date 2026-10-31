//! `hylite export-pdf <input.md> <output.pdf>`: the same renderer the toolbar
//! uses, without a window and without a save dialog, so a script can produce a
//! PDF. `main` passes the palette derived from the saved theme, so the CLI and
//! the GUI produce the same page for the same configuration.
use crate::export::pdf;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const USAGE: &str = "usage: hylite export-pdf <input.md> <output.pdf>";

/// Exit code for a refused or failed export.
pub const FAILURE: i32 = 1;
/// Exit code for a malformed command line.
pub const USAGE_ERROR: i32 = 2;

pub fn is_export_pdf(command: Option<&OsString>) -> bool {
    command.and_then(|command| command.to_str()) == Some("export-pdf")
}

pub fn run(args: Vec<OsString>, palette: pdf::Palette) -> i32 {
    match export(&args, palette) {
        Ok(()) => 0,
        Err((code, message)) => {
            eprintln!("{message}");
            code
        }
    }
}

fn export(args: &[OsString], palette: pdf::Palette) -> Result<(), (i32, String)> {
    // args[0] is the `export-pdf` word itself.
    if args.len() != 3 {
        return Err((USAGE_ERROR, format!("{USAGE}\n")));
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let markdown = std::fs::read_to_string(&input).map_err(|error| {
        (
            FAILURE,
            format!("Cannot read {}: {error}\n", input.display()),
        )
    })?;
    // The dialog path refuses to overwrite the open Markdown file; the CLI
    // refuses before touching the disk, so a typo cannot destroy the source.
    crate::files::ensure_distinct_destination(&input, &output)
        .map_err(|error| (FAILURE, format!("{error:#}\n")))?;
    write(&markdown, palette, &output)
}

fn write(markdown: &str, palette: pdf::Palette, output: &Path) -> Result<(), (i32, String)> {
    crate::export::export_pdf(markdown, palette, output).map_err(|error| {
        (
            FAILURE,
            format!("Cannot write {}: {error:#}\n", output.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    /// `export-pdf` plus file names resolved inside the temporary folder.
    fn inside(directory: &Path, names: &[&str]) -> Vec<OsString> {
        let mut args = vec![OsString::from("export-pdf")];
        args.extend(
            names
                .iter()
                .map(|name| OsString::from(directory.join(name).as_os_str())),
        );
        args
    }

    #[test]
    fn recognises_its_own_subcommand_only() {
        assert!(is_export_pdf(Some(&OsString::from("export-pdf"))));
        assert!(!is_export_pdf(Some(&OsString::from("export"))));
        assert!(!is_export_pdf(Some(&OsString::from("catatan.md"))));
        assert!(!is_export_pdf(None));
    }

    #[test]
    fn the_wrong_number_of_arguments_is_a_usage_error() {
        let light = pdf::Palette::default();
        assert_eq!(run(plain(&["export-pdf"]), light), USAGE_ERROR);
        assert_eq!(run(plain(&["export-pdf", "a.md"]), light), USAGE_ERROR);
        assert_eq!(
            run(plain(&["export-pdf", "a.md", "a.pdf", "extra"]), light),
            USAGE_ERROR
        );
    }

    #[test]
    fn writes_the_pdf_and_leaves_the_markdown_untouched() {
        let directory = tempfile::tempdir().unwrap();
        let markdown = "# Catatan\n\nIsi **tebal**.\n";
        std::fs::write(directory.path().join("catatan.md"), markdown).unwrap();

        let code = run(
            inside(directory.path(), &["catatan.md", "catatan.pdf"]),
            pdf::Palette::default(),
        );
        assert_eq!(code, 0);
        let bytes = std::fs::read(directory.path().join("catatan.pdf")).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("catatan.md")).unwrap(),
            markdown
        );
    }

    #[test]
    fn a_missing_input_fails_without_creating_an_output() {
        let directory = tempfile::tempdir().unwrap();
        let code = run(
            inside(directory.path(), &["hilang.md", "hilang.pdf"]),
            pdf::Palette::default(),
        );
        assert_eq!(code, FAILURE);
        assert!(!directory.path().join("hilang.pdf").exists());
    }

    #[test]
    fn refuses_to_overwrite_the_markdown_source() {
        let directory = tempfile::tempdir().unwrap();
        let markdown = "# Catatan\n";
        std::fs::write(directory.path().join("catatan.md"), markdown).unwrap();
        let code = run(
            inside(directory.path(), &["catatan.md", "catatan.md"]),
            pdf::Palette::default(),
        );
        assert_eq!(code, FAILURE);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("catatan.md")).unwrap(),
            markdown
        );
    }
}
