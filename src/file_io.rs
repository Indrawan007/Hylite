//! Native dialogs and blocking file work. Cancellation is not an I/O error.

use crate::{export, files};
use anyhow::Context;
use std::path::PathBuf;

pub type FileResult<T> = Result<Option<T>, String>;

async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| format!("File worker failed: {error}"))?
        .map_err(|error| format!("{error:#}"))
}

pub async fn open() -> FileResult<(PathBuf, String)> {
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_title("Open Markdown File")
        .add_filter("Markdown", &["md", "markdown", "mdx"])
        .add_filter("Text", &["txt"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    read_path(handle.path().to_path_buf()).await
}

/// Open a path that was chosen without a dialog (the file tree, or a launch
/// argument). Same UTF-8 and error contract as the dialog route.
pub async fn open_path(path: PathBuf) -> FileResult<(PathBuf, String)> {
    read_path(path).await
}

async fn read_path(path: PathBuf) -> FileResult<(PathBuf, String)> {
    blocking(move || {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("Cannot read {} as UTF-8", path.display()))?;
        Ok((path, text))
    })
    .await
    .map(Some)
}

pub async fn pick_folder() -> FileResult<PathBuf> {
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_title("Open Workspace Folder")
        .pick_folder()
        .await
    else {
        return Ok(None);
    };
    Ok(Some(handle.path().to_path_buf()))
}

/// One-shot workspace snapshot on the blocking pool: the scan never runs on the
/// UI thread and never follows symbolic links.
pub async fn scan_workspace(root: PathBuf) -> FileResult<crate::workspace::Tree> {
    blocking(move || crate::workspace::scan(&root).map_err(anyhow::Error::msg))
        .await
        .map(Some)
}

pub async fn save(
    current_path: Option<PathBuf>,
    text: String,
    save_as: bool,
) -> FileResult<PathBuf> {
    let path = match current_path.as_ref().filter(|_| !save_as) {
        Some(path) => path.clone(),
        None => {
            let filename = current_path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("untitled.md");
            let mut dialog = rfd::AsyncFileDialog::new()
                .set_title("Save Markdown As")
                .add_filter("Markdown", &["md"])
                .set_file_name(filename);
            if let Some(parent) = current_path.as_ref().and_then(|path| path.parent()) {
                dialog = dialog.set_directory(parent);
            }
            let Some(handle) = dialog.save_file().await else {
                return Ok(None);
            };
            handle.path().to_path_buf()
        }
    };
    blocking(move || {
        files::atomic_write(&path, text.as_bytes())
            .with_context(|| format!("Cannot save {}", path.display()))?;
        Ok(path)
    })
    .await
    .map(Some)
}

pub async fn export_html(markdown: String, source: Option<PathBuf>) -> FileResult<PathBuf> {
    let stem = source
        .as_ref()
        .and_then(|path| path.file_stem())
        .and_then(|name| name.to_str())
        .unwrap_or("untitled");
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_title("Export HTML")
        .add_filter("HTML", &["html"])
        .set_file_name(format!("{stem}.html"))
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let path = handle.path().to_path_buf();
    blocking(move || {
        if let Some(source) = source {
            files::ensure_distinct_destination(&source, &path)?;
        }
        export::export_html(&markdown, &path)
            .with_context(|| format!("Cannot export {}", path.display()))?;
        Ok(path)
    })
    .await
    .map(Some)
}

pub async fn export_pdf(
    markdown: String,
    palette: crate::export::pdf::Palette,
    source: Option<PathBuf>,
) -> FileResult<PathBuf> {
    let stem = source
        .as_ref()
        .and_then(|path| path.file_stem())
        .and_then(|name| name.to_str())
        .unwrap_or("untitled");
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_title("Export PDF")
        .add_filter("PDF", &["pdf"])
        .set_file_name(format!("{stem}.pdf"))
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let path = handle.path().to_path_buf();
    blocking(move || {
        if let Some(source) = source {
            files::ensure_distinct_destination(&source, &path)?;
        }
        crate::export::export_pdf(&markdown, palette, &path)
            .with_context(|| format!("Cannot export {}", path.display()))?;
        Ok(path)
    })
    .await
    .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_an_existing_path_does_not_need_a_dialog() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let result = runtime.block_on(save(Some(path.clone()), "Saved ✨".into(), false));
        assert_eq!(result.unwrap(), Some(path.clone()));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "Saved ✨");
    }

    #[test]
    fn worker_errors_retain_path_and_cause_instead_of_becoming_cancellation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing/note.md");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let error = runtime
            .block_on(save(Some(path.clone()), "Saved".into(), false))
            .unwrap_err();
        assert!(error.contains(&path.display().to_string()));
        assert!(error.contains("Cannot create temporary file"));
        assert!(!path.exists());
    }
}
