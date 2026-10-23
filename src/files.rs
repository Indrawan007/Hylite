//! Filesystem operations shared by Markdown saves and HTML exports.

use anyhow::{bail, Context};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Replace a regular file atomically, without replacing a symlink itself.
///
/// Existing permissions are retained. New files use tempfile's private (0600)
/// permissions. Hard-linked files are refused: rename would silently detach the
/// selected name from the other links. ACLs/xattrs/ownership are not copied.
/// This does not provide multi-process conflict detection or eliminate TOCTOU
/// races with another process changing the destination concurrently.
pub fn atomic_write(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    atomic_replace(path, |file| file.write_all(contents))
}

fn atomic_replace(
    path: &Path,
    write: impl FnOnce(&mut File) -> std::io::Result<()>,
) -> anyhow::Result<()> {
    let destination = resolve_destination(path)?;
    let metadata = match fs::metadata(&destination) {
        Ok(metadata) => {
            if !metadata.is_file() {
                bail!("Destination is not a regular file: {}", path.display());
            }
            if metadata.nlink() > 1 {
                bail!("Refusing to replace a hard-linked file: {}", path.display());
            }
            if metadata.permissions().readonly() {
                bail!("Destination is read-only: {}", path.display());
            }
            // Rename only needs directory permissions. Explicitly check file
            // write access too, so atomic save does not bypass a file's ACL.
            File::options()
                .write(true)
                .open(&destination)
                .with_context(|| format!("Destination is not writable: {}", path.display()))?;
            Some(metadata)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error).context("Cannot inspect destination"),
    };
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".hylite-")
        .tempfile_in(parent)
        .with_context(|| format!("Cannot create temporary file in {}", parent.display()))?;

    write(temporary.as_file_mut()).context("Cannot write temporary file")?;
    if let Some(metadata) = metadata {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .context("Cannot preserve file permissions")?;
    }
    temporary
        .as_file()
        .sync_all()
        .context("Cannot synchronize temporary file")?;
    temporary
        .persist(&destination)
        .with_context(|| format!("Cannot replace {}", destination.display()))?;
    // If directory synchronization fails, the rename may already have happened.
    // Report the error conservatively; the editor must remain dirty.
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .context("File replaced, but directory synchronization failed; verify the saved file")?;
    Ok(())
}

fn resolve_destination(path: &Path) -> anyhow::Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::canonicalize(path).with_context(|| {
                format!(
                    "Cannot resolve symlink {} (possibly dangling)",
                    path.display()
                )
            })
        }
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error).with_context(|| format!("Cannot inspect {}", path.display())),
    }
}

/// Prevent HTML export from overwriting the active Markdown file, even via a
/// symlink or hard-link alias. Missing destinations are safe to create.
pub fn ensure_distinct_destination(source: &Path, destination: &Path) -> anyhow::Result<()> {
    let source_metadata = fs::metadata(source)
        .with_context(|| format!("Cannot inspect source {}", source.display()))?;
    match fs::metadata(destination) {
        Ok(metadata)
            if metadata.dev() == source_metadata.dev()
                && metadata.ino() == source_metadata.ino() =>
        {
            bail!("Export cannot overwrite the active Markdown file")
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Cannot inspect export destination"),
    }
}

#[cfg(test)]
mod tests {
    use super::{atomic_replace, atomic_write, ensure_distinct_destination};
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn creates_and_replaces_without_leaving_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, "second ✨".as_bytes()).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "second ✨");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn preserves_existing_permissions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        fs::write(&path, "before").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        atomic_write(&path, b"after").unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn new_files_are_private() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("private.md");
        atomic_write(&path, b"private").unwrap();
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn saves_through_a_relative_symlink_without_replacing_it() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.md");
        let link = directory.path().join("link.md");
        fs::write(&target, "before").unwrap();
        symlink("target.md", &link).unwrap();
        atomic_write(&link, b"after").unwrap();
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(target).unwrap(), "after");
    }

    #[test]
    fn refuses_dangling_symlinks() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("missing.md");
        let link = directory.path().join("link.md");
        symlink(&target, &link).unwrap();
        assert!(atomic_write(&link, b"new").is_err());
        assert!(!target.exists());
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    }

    #[test]
    fn refuses_hard_links_without_changing_either_name() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        let alias = directory.path().join("alias.md");
        fs::write(&path, "before").unwrap();
        fs::hard_link(&path, &alias).unwrap();
        assert!(atomic_write(&path, b"after").is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "before");
        assert_eq!(fs::read_to_string(alias).unwrap(), "before");
    }

    #[test]
    fn refuses_read_only_files_without_changing_contents() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        fs::write(&path, "before").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        assert!(atomic_write(&path, b"after").is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "before");
    }

    #[test]
    fn rejects_directories_and_missing_parents() {
        let directory = tempfile::tempdir().unwrap();
        assert!(atomic_write(directory.path(), b"after").is_err());
        assert!(atomic_write(&directory.path().join("missing/note.md"), b"after").is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn export_rejects_source_and_its_aliases() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("note.md");
        let link = directory.path().join("link.md");
        let hard_link = directory.path().join("hard.md");
        fs::write(&source, "source").unwrap();
        symlink(&source, &link).unwrap();
        fs::hard_link(&source, &hard_link).unwrap();
        for destination in [&source, &link, &hard_link] {
            assert!(ensure_distinct_destination(&source, destination).is_err());
        }
        assert!(ensure_distinct_destination(&source, &directory.path().join("new.html")).is_ok());
        assert_eq!(fs::read_to_string(source).unwrap(), "source");
    }

    #[test]
    fn partial_write_failure_preserves_original_and_cleans_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("note.md");
        fs::write(&path, "original").unwrap();
        let result = atomic_replace(&path, |file| {
            file.write_all(b"partial")?;
            Err(std::io::Error::other("simulated disk full"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "original");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
