//! Read-only, bounded-work Markdown lexer. This is deliberately not a CommonMark
//! parser: inline constructs are single-line, containers are only lexical prefixes.
use std::ops::Range;

/// Long lines remain plain (fence state is still tracked). Inline lookahead has a
/// shared 4 * line-byte budget, preventing repeated unmatched delimiters from
/// causing quadratic scans. Neither limit truncates or edits source text.
pub const MAX_LINE_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Heading,
    Marker,
    Quote,
    Task,
    Strong,
    Emphasis,
    Code,
    Link,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub dark: bool,
    /// Document ID + native buffer generation, not the content/dirty checkpoint.
    pub buffer_key: (u64, u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fence {
    marker: u8,
    length: usize,
}

pub struct Lexer {
    settings: Settings,
    /// Fence state BEFORE each processed logical line; no retained text/spans.
    states: Vec<Option<Fence>>,
    next_line: usize,
}

impl Lexer {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            states: vec![None],
            next_line: 0,
        }
    }
    pub fn update(&mut self, settings: Settings) {
        if self.settings != settings {
            *self = Self::new(settings);
        }
    }
    pub fn current_line(&self) -> usize {
        self.next_line
    }
    pub fn change_line(&mut self, line: usize) {
        self.next_line = self.next_line.min(line);
        self.states.truncate(self.next_line + 1);
    }
    pub fn highlight_line(&mut self, line: &str) -> Vec<(Range<usize>, Kind)> {
        if !self.settings.enabled {
            self.next_line += 1;
            return Vec::new();
        }
        let (spans, next) = tokenize(line, self.states[self.next_line]);
        self.states.truncate(self.next_line + 1);
        self.states.push(next);
        self.next_line += 1;
        spans
    }
}

pub fn color(kind: Kind, dark: bool) -> [u8; 3] {
    // Color-only: preserve configured font, glyph metrics, and cursor behavior.
    match (kind, dark) {
        (Kind::Heading, true) => [203, 166, 247],
        (Kind::Heading, false) => [107, 33, 168],
        (Kind::Marker | Kind::Quote, true) => [166, 173, 200],
        (Kind::Marker | Kind::Quote, false) => [75, 85, 99],
        (Kind::Task, true) => [166, 227, 161],
        (Kind::Task, false) => [22, 101, 52],
        (Kind::Strong, true) => [250, 179, 135],
        (Kind::Strong, false) => [154, 52, 18],
        (Kind::Emphasis, true) => [245, 194, 231],
        (Kind::Emphasis, false) => [157, 23, 77],
        (Kind::Code, true) => [166, 227, 161],
        (Kind::Code, false) => [22, 101, 52],
        (Kind::Link, true) => [137, 180, 250],
        (Kind::Link, false) => [29, 78, 216],
    }
}

fn spaces(line: &str) -> Option<usize> {
    let n = line.bytes().take_while(|byte| *byte == b' ').count();
    (n <= 3 && line.as_bytes().get(n) != Some(&b'\t')).then_some(n)
}
fn blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}
fn run(bytes: &[u8], start: usize, marker: u8) -> usize {
    bytes[start..]
        .iter()
        .take_while(|byte| **byte == marker)
        .count()
}
fn fence(line: &str) -> Option<(usize, Fence, &str)> {
    let start = spaces(line)?;
    let marker = *line.as_bytes().get(start)?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = run(line.as_bytes(), start, marker);
    (length >= 3).then(|| (start, Fence { marker, length }, &line[start + length..]))
}

fn tokenize(line: &str, state: Option<Fence>) -> (Vec<(Range<usize>, Kind)>, Option<Fence>) {
    if let Some(open) = state {
        let closing = fence(line).is_some_and(|(_, close, tail)| {
            close.marker == open.marker && close.length >= open.length && tail.bytes().all(blank)
        });
        let spans = if line.is_empty() || line.len() > MAX_LINE_BYTES {
            Vec::new()
        } else {
            vec![(
                0..line.len(),
                if closing { Kind::Marker } else { Kind::Code },
            )]
        };
        return (spans, if closing { None } else { state });
    }
    if let Some((start, open, tail)) = fence(line) {
        if open.marker != b'`' || !tail.contains('`') {
            let spans = if line.len() > MAX_LINE_BYTES {
                Vec::new()
            } else {
                vec![(start..line.len(), Kind::Marker)]
            };
            return (spans, Some(open));
        }
    }
    if line.len() > MAX_LINE_BYTES {
        return (Vec::new(), None);
    }
    let Some(mut start) = spaces(line) else {
        return (Vec::new(), None);
    };
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    // Quote chains are prefixes only; quoted/list-nested fences do not enter the
    // top-level fence state machine.
    let quote_start = start;
    while bytes.get(start) == Some(&b'>') {
        start += 1;
        if bytes.get(start).is_some_and(|byte| blank(*byte)) {
            start += 1;
        }
    }
    if start > quote_start {
        spans.push((quote_start..start, Kind::Quote));
    }
    let body = &line[start..];
    if let Some(&marker) = body.as_bytes().first() {
        if matches!(marker, b'-' | b'_' | b'*')
            && body.bytes().all(|byte| byte == marker || blank(byte))
            && body.bytes().filter(|byte| *byte == marker).count() >= 3
        {
            spans.push((start..line.len(), Kind::Marker));
            return (spans, None);
        }
    }
    let digits = body.bytes().take_while(u8::is_ascii_digit).count();
    let marker_len = if matches!(bytes.get(start), Some(b'-' | b'+' | b'*')) {
        1
    } else if (1..=9).contains(&digits) && matches!(bytes.get(start + digits), Some(b'.' | b')')) {
        digits + 1
    } else {
        0
    };
    if marker_len > 0
        && bytes
            .get(start + marker_len)
            .is_none_or(|byte| blank(*byte))
    {
        let end = (start + marker_len + 1).min(line.len());
        spans.push((start..end, Kind::Marker));
        start = end;
        if matches!(
            bytes.get(start..start.saturating_add(3)),
            Some([b'[', b' ' | b'x' | b'X', b']'])
        ) && bytes.get(start + 3).is_none_or(|byte| blank(*byte))
        {
            spans.push((start..start + 3, Kind::Task));
            start += 3;
        }
    }
    let hashes = run(bytes, start, b'#');
    if (1..=6).contains(&hashes) && bytes.get(start + hashes).is_none_or(|byte| blank(*byte)) {
        spans.push((start..line.len(), Kind::Heading));
    } else {
        inline(line, start, &mut spans);
    }
    (spans, None)
}

fn escaped(bytes: &[u8], index: usize) -> bool {
    (bytes[..index]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        & 1)
        == 1
}
fn word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Scan matching delimiter runs, charging every scanned byte to one shared
/// per-line budget. Backslashes do NOT escape closing backticks inside code.
fn close_run(
    line: &str,
    from: usize,
    marker: u8,
    length: usize,
    budget: &mut usize,
) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = from;
    while i < bytes.len() && *budget > 0 {
        let start = i;
        if bytes[i] != marker {
            i += 1;
            *budget -= 1;
            continue;
        }
        while i < bytes.len() && bytes[i] == marker && *budget > 0 {
            i += 1;
            *budget -= 1;
        }
        if bytes.get(i) == Some(&marker) {
            return None;
        } // incomplete run at budget boundary
        if i - start == length {
            let valid = marker == b'`'
                || (!escaped(bytes, start)
                    && line[..start]
                        .chars()
                        .next_back()
                        .is_some_and(|ch| !ch.is_whitespace())
                    && (marker != b'_' || !line[i..].chars().next().is_some_and(word)));
            if valid {
                return Some(i);
            }
        }
    }
    None
}

fn link_end(line: &str, start: usize, budget: &mut usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = start + 1;
    let mut depth = 1;
    while i < bytes.len() && *budget > 0 {
        let byte = bytes[i];
        i += 1;
        *budget -= 1;
        if byte == b'\\' {
            if i < bytes.len() && *budget > 0 {
                i += 1;
                *budget -= 1;
            }
        } else if byte == b'[' {
            depth += 1;
        } else if byte == b']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
    }
    if depth != 0 || bytes.get(i) != Some(&b'(') || *budget == 0 {
        return None;
    }
    i += 1;
    *budget -= 1;
    depth = 1;
    while i < bytes.len() && *budget > 0 {
        let byte = bytes[i];
        i += 1;
        *budget -= 1;
        if byte == b'\\' {
            if i < bytes.len() && *budget > 0 {
                i += 1;
                *budget -= 1;
            }
        } else if byte == b'(' {
            depth += 1;
        } else if byte == b')' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

fn inline(line: &str, mut i: usize, spans: &mut Vec<(Range<usize>, Kind)>) {
    let bytes = line.as_bytes();
    let mut budget = line.len().saturating_mul(4);
    while i < bytes.len() && budget > 0 {
        let byte = bytes[i];
        if byte == b'\\' {
            i += 1;
            if i < bytes.len() {
                i += line[i..]
                    .chars()
                    .next()
                    .expect("nonempty suffix")
                    .len_utf8();
            }
            continue;
        }
        if matches!(byte, b'`' | b'*' | b'_') {
            let length = run(bytes, i, byte);
            let from = i + length;
            let eligible = byte == b'`'
                || (length <= 3
                    && line[from..]
                        .chars()
                        .next()
                        .is_some_and(|ch| !ch.is_whitespace())
                    && (byte != b'_' || !line[..i].chars().next_back().is_some_and(word)));
            if eligible {
                if let Some(end) = close_run(line, from, byte, length, &mut budget) {
                    let kind = if byte == b'`' {
                        Kind::Code
                    } else if length >= 2 {
                        Kind::Strong
                    } else {
                        Kind::Emphasis
                    };
                    spans.push((i..end, kind));
                    i = end;
                    continue;
                }
            }
            i = from;
            continue;
        }
        let image = byte == b'!' && bytes.get(i + 1) == Some(&b'[');
        if byte == b'[' || image {
            if let Some(end) = link_end(line, i + usize::from(image), &mut budget) {
                spans.push((i..end, Kind::Link));
                i = end;
                continue;
            }
        }
        i += line[i..]
            .chars()
            .next()
            .expect("nonempty suffix")
            .len_utf8();
    }
}

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod tests;
