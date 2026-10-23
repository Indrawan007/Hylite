//! Read-only image references for the Preview.
//!
//! Only local files are ever opened: a destination with a URL scheme (`http:`,
//! `https:`, `file:`, `data:`, `//host`) is rejected *before* any I/O, so the
//! preview can never fetch the network or leave the format string as an excuse.
//! Every limit is enforced from the file header, without a decoder dependency:
//! the byte size is checked with `metadata`, and the pixel dimensions are parsed
//! from the PNG / GIF / BMP / JPEG headers. Anything else stays an inert marker.
use std::path::{Path, PathBuf};

/// Largest file the Preview will hand to the renderer (bytes on disk).
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Largest width or height accepted, in pixels.
pub const MAX_DIMENSION: u32 = 8_192;
/// Largest width × height accepted. Guards decompression memory, not just disk.
pub const MAX_PIXELS: u64 = 40_000_000;
/// How much of the file header is read for the dimension check.
pub const HEADER_BYTES: usize = 64 * 1024;

/// One parsed `![alt](src)` destination. Built by the preview projection, never
/// by the renderer, so the renderer cannot invent a path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spec {
    pub src: String,
    pub alt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `![]()` with nothing usable.
    Empty,
    /// A URL (`http:`, `https:`, `file:`, `data:`, `//host`, or any `scheme:`).
    Remote,
    /// NUL bytes, broken percent-encoding, or a fragment-only destination.
    InvalidPath,
    /// A relative path while the document has never been saved.
    MissingBase,
    /// The path does not exist.
    NotFound,
    /// The path exists but is not a regular file (a folder, a socket, …).
    NotAFile,
    /// The file exists but could not be opened or read.
    Unreadable,
    /// Larger than `MAX_FILE_BYTES`.
    TooLarge,
    /// Wider/taller than `MAX_DIMENSION`, or more than `MAX_PIXELS` in total.
    TooManyPixels,
    /// Not PNG, GIF, BMP or JPEG, or a header too short to read.
    UnsupportedFormat,
}

impl Error {
    /// Stable, testable text used by the inert Preview marker.
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "empty destination",
            Self::Remote => "remote URL",
            Self::InvalidPath => "invalid path",
            Self::MissingBase => "save the document first",
            Self::NotFound => "file not found",
            Self::NotAFile => "not a file",
            Self::Unreadable => "cannot read file",
            Self::TooLarge => "file too large",
            Self::TooManyPixels => "image too large",
            Self::UnsupportedFormat => "unsupported format",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
}

/// Resolve a Markdown destination to a filesystem path without touching disk.
///
/// Relative paths are joined to the document's folder; without one they are
/// refused (`MissingBase`), because guessing a working directory is how a
/// preview silently shows the wrong picture.
pub fn resolve(src: &str, base: Option<&Path>) -> Result<PathBuf, Error> {
    let trimmed = src.trim();
    if trimmed.is_empty() {
        return Err(Error::Empty);
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("://") || lower.starts_with("//") || lower.starts_with("data:") {
        return Err(Error::Remote);
    }
    // Any other `scheme:` (mailto:, file:, …) is a URL too, never a filename.
    if let Some((scheme, _)) = lower.split_once(':') {
        let is_scheme = !scheme.is_empty()
            && scheme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if is_scheme {
            return Err(Error::Remote);
        }
    }
    if trimmed.starts_with('#') {
        return Err(Error::InvalidPath);
    }
    let decoded = decode_percent(trimmed).ok_or(Error::InvalidPath)?;
    if decoded.contains('\0') || decoded.is_empty() {
        return Err(Error::InvalidPath);
    }
    let path = PathBuf::from(decoded);
    if path.is_absolute() {
        return Ok(path);
    }
    let base = base.ok_or(Error::MissingBase)?;
    Ok(base.join(path))
}

/// Decode `%XX` escapes. Returns `None` for a broken escape or non-UTF-8 bytes,
/// so a malformed destination can never become a surprising path.
pub fn decode_percent(src: &str) -> Option<String> {
    let bytes = src.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let value = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
            out.push(value);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Size and dimension gate. Reads at most `HEADER_BYTES`, never the whole file.
pub fn inspect(path: &Path) -> Result<Dimensions, Error> {
    let metadata = std::fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => Error::NotFound,
        _ => Error::Unreadable,
    })?;
    if !metadata.is_file() {
        return Err(Error::NotAFile);
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(Error::TooLarge);
    }
    let mut file = std::fs::File::open(path).map_err(|_| Error::Unreadable)?;
    let mut header = vec![0_u8; HEADER_BYTES];
    let read = std::io::Read::read(&mut file, &mut header).map_err(|_| Error::Unreadable)?;
    header.truncate(read);
    let (width, height) = parse_dimensions(&header).ok_or(Error::UnsupportedFormat)?;
    if width == 0 || height == 0 {
        return Err(Error::UnsupportedFormat);
    }
    if width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err(Error::TooManyPixels);
    }
    Ok(Dimensions {
        width,
        height,
        bytes: metadata.len(),
    })
}

/// Resolve, then gate. The returned path is the only thing the renderer sees.
pub fn load(spec: &Spec, base: Option<&Path>) -> Result<(PathBuf, Dimensions), Error> {
    let path = resolve(&spec.src, base)?;
    let dimensions = inspect(&path)?;
    Ok((path, dimensions))
}

/// PNG, GIF, BMP and JPEG only. Everything else is `None` (→ unsupported).
pub fn parse_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return png(bytes);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return gif(bytes);
    }
    if bytes.starts_with(b"BM") {
        return bmp(bytes);
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return jpeg(bytes);
    }
    None
}

fn be32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?))
}

fn le16(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(..2)?.try_into().ok()?))
}

fn le32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?))
}

fn be16(bytes: &[u8]) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(..2)?.try_into().ok()?))
}

fn png(bytes: &[u8]) -> Option<(u32, u32)> {
    // 8-byte signature, 4-byte length, "IHDR", then width and height.
    if bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    Some((be32(bytes.get(16..)?)?, be32(bytes.get(20..)?)?))
}

fn gif(bytes: &[u8]) -> Option<(u32, u32)> {
    // Logical screen descriptor: little-endian width and height.
    Some((
        u32::from(le16(bytes.get(6..)?)?),
        u32::from(le16(bytes.get(8..)?)?),
    ))
}

fn bmp(bytes: &[u8]) -> Option<(u32, u32)> {
    let header_size = le32(bytes.get(14..)?)?;
    if header_size < 40 {
        return None;
    }
    let width = i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?);
    let height = i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?);
    Some((width.unsigned_abs(), height.unsigned_abs()))
}

fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut index = 2;
    while index + 4 <= bytes.len() {
        if bytes[index] != 0xFF {
            return None;
        }
        let marker = *bytes.get(index + 1)?;
        // Standalone markers carry no length.
        if marker == 0x01 || (0xD0..=0xD8).contains(&marker) {
            index += 2;
            continue;
        }
        let length = usize::from(be16(bytes.get(index + 2..)?)?);
        if length < 2 {
            return None;
        }
        let is_sof = matches!(
            marker,
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
        );
        if is_sof {
            // length(2) precision(1) height(2) width(2)
            let height = be16(bytes.get(index + 5..)?)?;
            let width = be16(bytes.get(index + 7..)?)?;
            return Some((u32::from(width), u32::from(height)));
        }
        index += 2 + length;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hylite-images-{tag}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create test folder");
            Self { path }
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            std::fs::write(&path, bytes).expect("write fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    fn gif_header(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = b"GIF89a".to_vec();
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&[0, 0, 0]);
        bytes
    }

    fn bmp_header(width: i32, height: i32) -> Vec<u8> {
        let mut bytes = b"BM".to_vec();
        bytes.extend_from_slice(&[0; 12]);
        bytes.extend_from_slice(&40_u32.to_le_bytes());
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes
    }

    fn jpeg_header(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        bytes.extend_from_slice(&16_u16.to_be_bytes());
        bytes.extend_from_slice(&[0; 14]);
        bytes.extend_from_slice(&[0xFF, 0xC0]);
        bytes.extend_from_slice(&17_u16.to_be_bytes());
        bytes.push(8);
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
        bytes
    }

    #[test]
    fn destinations_with_a_scheme_never_become_paths() {
        for src in [
            "https://example.invalid/x.png",
            "HTTP://EXAMPLE.INVALID/x.png",
            "file:///etc/passwd",
            "data:image/png;base64,AAAA",
            "//example.invalid/x.png",
            "mailto:someone@example.invalid",
            "ftp://example.invalid/x.png",
        ] {
            assert_eq!(
                resolve(src, Some(Path::new("/tmp"))),
                Err(Error::Remote),
                "{src}"
            );
        }
    }

    #[test]
    fn relative_paths_need_a_document_folder_and_absolute_paths_do_not() {
        let base = Path::new("/home/user/notes");
        assert_eq!(
            resolve("img.png", Some(base)),
            Ok(PathBuf::from("/home/user/notes/img.png"))
        );
        assert_eq!(
            resolve("../shared/img.png", Some(base)),
            Ok(PathBuf::from("/home/user/notes/../shared/img.png"))
        );
        assert_eq!(
            resolve("/srv/img.png", None),
            Ok(PathBuf::from("/srv/img.png"))
        );
        assert_eq!(resolve("img.png", None), Err(Error::MissingBase));
    }

    #[test]
    fn empty_fragment_and_broken_percent_destinations_are_refused() {
        let base = Some(Path::new("/tmp"));
        assert_eq!(resolve("   ", base), Err(Error::Empty));
        assert_eq!(resolve("#anchor", base), Err(Error::InvalidPath));
        assert_eq!(resolve("bad%2.png", base), Err(Error::InvalidPath));
        assert_eq!(resolve("bad%zz.png", base), Err(Error::InvalidPath));
        assert_eq!(resolve("nul\0byte.png", base), Err(Error::InvalidPath));
        // A percent-encoded space is a real filename, not a URL.
        assert_eq!(
            resolve("my%20image.png", base),
            Ok(PathBuf::from("/tmp/my image.png"))
        );
        assert_eq!(decode_percent("caf%C3%A9.png").as_deref(), Some("café.png"));
    }

    #[test]
    fn dimensions_come_from_the_header_of_each_supported_format() {
        assert_eq!(parse_dimensions(&png_header(320, 200)), Some((320, 200)));
        assert_eq!(parse_dimensions(&gif_header(640, 480)), Some((640, 480)));
        assert_eq!(parse_dimensions(&bmp_header(800, -600)), Some((800, 600)));
        assert_eq!(parse_dimensions(&jpeg_header(1024, 768)), Some((1024, 768)));
        assert_eq!(parse_dimensions(b"not an image"), None);
        assert_eq!(parse_dimensions(&[0xFF, 0xD8, 0xFF]), None);
        assert_eq!(parse_dimensions(&[]), None);
    }

    #[test]
    fn inspect_reports_missing_directories_and_truncated_headers() {
        let root = TempDir::new("inspect");
        assert_eq!(
            inspect(&root.path.join("missing.png")),
            Err(Error::NotFound)
        );
        assert_eq!(inspect(&root.path), Err(Error::NotAFile));
        let truncated = root.write("tiny.png", b"\x89PNG\r\n\x1a\n");
        assert_eq!(inspect(&truncated), Err(Error::UnsupportedFormat));
        let text = root.write("note.png", b"hello world");
        assert_eq!(inspect(&text), Err(Error::UnsupportedFormat));
    }

    #[test]
    fn inspect_accepts_a_small_png_and_refuses_oversized_files_and_pixels() {
        let root = TempDir::new("limits");
        let ok = root.write("ok.png", &png_header(320, 200));
        assert_eq!(
            inspect(&ok),
            Ok(Dimensions {
                width: 320,
                height: 200,
                bytes: png_header(320, 200).len() as u64,
            })
        );

        // Header claims a huge canvas while the file stays tiny: the pixel gate
        // must reject it from the header alone, without decoding.
        let huge = root.write("huge.png", &png_header(9_000, 200));
        assert_eq!(inspect(&huge), Err(Error::TooManyPixels));
        let many = root.write("many.png", &png_header(8_000, 8_000));
        assert_eq!(inspect(&many), Err(Error::TooManyPixels));

        let mut big = png_header(16, 16);
        big.resize(MAX_FILE_BYTES as usize + 1, 0);
        let big = root.write("big.png", &big);
        assert_eq!(inspect(&big), Err(Error::TooLarge));
    }

    #[test]
    fn load_resolves_and_gates_in_one_step() {
        let root = TempDir::new("load");
        root.write("pic.png", &png_header(64, 32));
        let spec = Spec {
            src: "pic.png".into(),
            alt: "a picture".into(),
        };
        let (path, dimensions) = load(&spec, Some(&root.path)).expect("local image loads");
        assert_eq!(path, root.path.join("pic.png"));
        assert_eq!((dimensions.width, dimensions.height), (64, 32));
        assert_eq!(dimensions.bytes, png_header(64, 32).len() as u64);

        let remote = Spec {
            src: "https://example.invalid/x.png".into(),
            ..Spec::default()
        };
        assert_eq!(load(&remote, Some(&root.path)), Err(Error::Remote));
        let missing = Spec {
            src: "missing.png".into(),
            ..Spec::default()
        };
        assert_eq!(load(&missing, Some(&root.path)), Err(Error::NotFound));
        let no_base = Spec {
            src: "pic.png".into(),
            ..Spec::default()
        };
        assert_eq!(load(&no_base, None), Err(Error::MissingBase));
    }

    #[test]
    fn every_error_has_a_stable_marker_label() {
        let labels = [
            (Error::Empty, "empty destination"),
            (Error::Remote, "remote URL"),
            (Error::InvalidPath, "invalid path"),
            (Error::MissingBase, "save the document first"),
            (Error::NotFound, "file not found"),
            (Error::NotAFile, "not a file"),
            (Error::Unreadable, "cannot read file"),
            (Error::TooLarge, "file too large"),
            (Error::TooManyPixels, "image too large"),
            (Error::UnsupportedFormat, "unsupported format"),
        ];
        for (error, label) in labels {
            assert_eq!(error.label(), label);
        }
    }
}
