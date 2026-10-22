//! Go-to-line input and UTF-8 offsets, independent of the native widget.
//! Addresses are 1-based logical lines and Unicode scalar columns, not visual
//! soft-wrap rows, bytes, graphemes, or display-cell widths.
use std::fmt;

const MAX_QUERY_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    TooLong,
    InvalidAddress,
    LineOutOfRange,
    ColumnOutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLong => "Address is too long (maximum 64 bytes)",
            Self::InvalidAddress => "Use line or line:column with positive ASCII integers",
            Self::LineOutOfRange => "Line is outside the current document",
            Self::ColumnOutOfRange => "Column is outside the line (end-of-line is allowed)",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Destination {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

fn positive(value: &str) -> Option<usize> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse::<usize>().ok().filter(|value| *value > 0)
}

/// Resolve against the current raw buffer, never the serializer's synthetic LF.
/// Bad addresses are rejected, not silently clamped. Empty documents and a final
/// empty line after a trailing LF both have a valid column 1.
pub fn resolve(text: &str, query: &str) -> Result<Destination, Error> {
    if query.len() > MAX_QUERY_BYTES {
        return Err(Error::TooLong);
    }
    let query = query.trim();
    let (line, column) = query.split_once(':').unwrap_or((query, "1"));
    let line = positive(line).ok_or(Error::InvalidAddress)?;
    let column = positive(column).ok_or(Error::InvalidAddress)?;
    let mut offset = 0;
    for (index, content) in text.split('\n').enumerate() {
        if index == line - 1 {
            let byte = content
                .char_indices()
                .map(|(byte, _)| byte)
                .chain(std::iter::once(content.len()))
                .nth(column - 1)
                .ok_or(Error::ColumnOutOfRange)?;
            return Ok(Destination {
                offset: offset + byte,
                line,
                column,
            });
        }
        // Only advance past an LF when it exists, so even the final line's
        // offset arithmetic stays bounded by text.len().
        offset += content.len();
        if offset < text.len() {
            offset += 1;
        }
    }
    Err(Error::LineOutOfRange)
}

#[derive(Default)]
pub struct GoTo {
    pub open: bool,
    pub query: String,
    pub error: Option<Error>,
}

impl GoTo {
    pub fn show(&mut self, line: usize, column: usize) {
        self.open = true;
        self.query = format!("{line}:{column}");
        self.error = None;
    }

    pub fn change_query(&mut self, query: String) {
        self.query = query;
        self.error = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.error = None;
    }
}

#[cfg(test)]
#[path = "navigation_tests.rs"]
mod tests;
