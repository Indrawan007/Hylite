//! Lexical Markdown source actions, not an AST formatter. All edits are UTF-8
//! transactions; only the requested line-prefix family is normalized.

use crate::text_edit::{line_range, valid_range, TextChange};
use std::{fmt, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockStyle {
    Heading(u8),
    BulletList,
    OrderedList,
    TaskList,
    Quote,
}

impl BlockStyle {
    pub const ALL: [Self; 10] = [
        Self::Heading(1),
        Self::Heading(2),
        Self::Heading(3),
        Self::Heading(4),
        Self::Heading(5),
        Self::Heading(6),
        Self::BulletList,
        Self::OrderedList,
        Self::TaskList,
        Self::Quote,
    ];
}

impl fmt::Display for BlockStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Heading(level) => write!(f, "Heading {level}"),
            Self::BulletList => f.write_str("Bullet list"),
            Self::OrderedList => f.write_str("Numbered list"),
            Self::TaskList => f.write_str("Task list"),
            Self::Quote => f.write_str("Block quote"),
        }
    }
}

struct Line<'a> {
    indent: &'a str,
    body: &'a str,
    style: Option<BlockStyle>,
    checked: bool,
}

fn blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

// Consume one separator only: additional spaces belong to the body and must
// survive conversions. EOL is also accepted for an empty heading/list item.
fn separator_end(text: &str, marker_end: usize) -> Option<usize> {
    match text.as_bytes().get(marker_end) {
        None => Some(marker_end),
        Some(byte) if blank(*byte) => Some(marker_end + 1),
        _ => None,
    }
}

fn parse_line(line: &str, target: BlockStyle) -> Line<'_> {
    let indent_end = line.bytes().take_while(|byte| blank(*byte)).count();
    let body = &line[indent_end..];
    let mut parsed = Line {
        indent: &line[..indent_end],
        body,
        style: None,
        checked: false,
    };
    match target {
        BlockStyle::Heading(_) => {
            let hashes = body.bytes().take_while(|byte| *byte == b'#').count();
            if (1..=6).contains(&hashes) {
                if let Some(end) = separator_end(body, hashes) {
                    parsed.style = Some(BlockStyle::Heading(hashes as u8));
                    parsed.body = &body[end..];
                }
            }
        }
        BlockStyle::Quote => {
            if let Some(rest) = body.strip_prefix('>') {
                parsed.style = Some(BlockStyle::Quote);
                parsed.body = rest.strip_prefix([' ', '\t']).unwrap_or(rest);
            }
        }
        BlockStyle::BulletList | BlockStyle::OrderedList | BlockStyle::TaskList => {
            let digits = body.bytes().take_while(u8::is_ascii_digit).count();
            let marker = if matches!(body.as_bytes().first(), Some(b'-' | b'+' | b'*')) {
                Some((1, BlockStyle::BulletList))
            } else if (1..=9).contains(&digits)
                && matches!(body.as_bytes().get(digits), Some(b'.' | b')'))
            {
                Some((digits + 1, BlockStyle::OrderedList))
            } else {
                None
            };
            if let Some((end, style)) = marker {
                if let Some(end) = separator_end(body, end) {
                    parsed.style = Some(style);
                    parsed.body = &body[end..];
                    let bytes = parsed.body.as_bytes();
                    if bytes.len() >= 3
                        && bytes[0] == b'['
                        && bytes[2] == b']'
                        && matches!(bytes[1], b' ' | b'x' | b'X')
                    {
                        if let Some(end) = separator_end(parsed.body, 3) {
                            parsed.style = Some(BlockStyle::TaskList);
                            parsed.checked = bytes[1] != b' ';
                            parsed.body = &parsed.body[end..];
                        }
                    }
                }
            }
        }
    }
    parsed
}

/// Apply to all touched logical lines, excluding a next line selected only at
/// column zero. If all nonblank lines already have the requested prefix, remove
/// it; otherwise normalize this prefix family. Blank lines are preserved.
pub fn block(text: &str, selection: Range<usize>, style: BlockStyle) -> Option<TextChange> {
    if matches!(style, BlockStyle::Heading(level) if !(1..=6).contains(&level)) {
        return None;
    }
    let range = line_range(text, selection)?;
    let source = &text[range.clone()];
    let lines: Vec<_> = source
        .split('\n')
        .map(|line| parse_line(line, style))
        .collect();
    let nonblank: Vec<_> = lines
        .iter()
        .filter(|line| !line.body.is_empty() || line.style.is_some())
        .collect();
    // Empty/whitespace-only lines should be usable as a new block when no other
    // content is selected; an all-blank selection formats only its first line.
    let remove = !nonblank.is_empty() && nonblank.iter().all(|line| line.style == Some(style));
    let all_blank = nonblank.is_empty();
    let mut number = 0;
    let mut result = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        if line.body.is_empty() && line.style.is_none() && !(all_blank && index == 0) {
            result.push(line.indent.to_owned());
            continue;
        }
        number += 1;
        let prefix = if remove {
            String::new()
        } else {
            match style {
                BlockStyle::Heading(level) => format!("{} ", "#".repeat(usize::from(level))),
                BlockStyle::BulletList => "- ".into(),
                BlockStyle::OrderedList => format!("{number}. "),
                BlockStyle::TaskList => if line.checked { "- [x] " } else { "- [ ] " }.into(),
                BlockStyle::Quote => "> ".into(),
            }
        };
        result.push(format!("{}{prefix}{}", line.indent, line.body));
    }
    let replacement = result.join("\n");
    Some(TextChange {
        range,
        caret: replacement.len(),
        replacement,
    })
}

/// Insert a source link using a single-line label and an editable URL stub.
/// Escaping label brackets/backslashes avoids accidentally creating nested links.
pub fn link(text: &str, selection: Range<usize>) -> Option<TextChange> {
    if !valid_range(text, &selection) {
        return None;
    }
    let selected = &text[selection.clone()];
    if selected.chars().any(char::is_control) {
        return None;
    }
    let label = if selected.is_empty() {
        "text"
    } else {
        selected
    };
    let mut replacement = String::from("[");
    for ch in label.chars() {
        if matches!(ch, '\\' | '[' | ']') {
            replacement.push('\\');
        }
        replacement.push(ch);
    }
    replacement.push_str("](https://");
    let caret = replacement.len();
    replacement.push(')');
    Some(TextChange {
        range: selection,
        replacement,
        caret,
    })
}

/// Wrap touched whole lines in a fenced code block. A fence longer than every
/// backtick run in the body prevents payload text from closing it prematurely.
/// This is insertion, not toggle/unwrapping or language inference.
pub fn code_block(text: &str, selection: Range<usize>) -> Option<TextChange> {
    let range = line_range(text, selection.clone())?;
    let body = &text[range.clone()];
    let longest = body.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.saturating_add(1).max(3));
    let caret = fence.len() + 1 + (selection.end - range.start).min(body.len());
    let replacement = format!("{fence}\n{body}\n{fence}");
    Some(TextChange {
        range,
        replacement,
        caret,
    })
}

#[cfg(test)]
#[path = "block_edit_tests.rs"]
mod tests;
