//! Conservative Enter transactions for simple, unquoted Markdown lists.
//! No AST parsing or sibling renumbering. Ambiguous contexts use native Enter.
use crate::text_edit::TextChange;

const MAX_LINE_BYTES: usize = 16 * 1024;

fn blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}
fn whitespace(text: &str, mut column: usize) -> (usize, usize) {
    let start = column;
    let mut bytes = 0;
    for byte in text.bytes().take_while(|byte| blank(*byte)) {
        column += if byte == b'\t' { 4 - column % 4 } else { 1 };
        bytes += 1;
    }
    (bytes, column - start)
}

struct Item<'a> {
    indent: &'a str,
    marker: &'a str,
    separator: &'a str,
    number: Option<u32>,
    /// Beginning of list content BEFORE any task checkbox.
    content: usize,
    body: usize,
    task_separator: Option<&'a str>,
}

fn item(line: &str) -> Option<Item<'_>> {
    parse_item(line, true)
}

fn parse_item(line: &str, strict_gap: bool) -> Option<Item<'_>> {
    let bytes = line.as_bytes();
    let (indent, indent_column) = whitespace(line, 0);
    let digits = bytes[indent..]
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let marker_len = if matches!(bytes.get(indent), Some(b'-' | b'+' | b'*')) {
        1
    } else if (1..=9).contains(&digits) && matches!(bytes.get(indent + digits), Some(b'.' | b')')) {
        digits + 1
    } else {
        return None;
    };
    let marker_end = indent + marker_len;
    let (gap, gap_columns) = whitespace(&line[marker_end..], indent_column + marker_len);
    if (gap == 0 && marker_end < line.len()) || (strict_gap && gap_columns > 4) {
        return None;
    }
    let content = marker_end + gap;
    let mut body = content;
    let mut task_separator = None;
    if matches!(
        bytes.get(body..body + 3),
        Some([b'[', b' ' | b'x' | b'X', b']'])
    ) && bytes.get(body + 3).is_none_or(|byte| blank(*byte))
    {
        body += 3;
        let gap = bytes[body..]
            .iter()
            .take_while(|byte| blank(**byte))
            .count();
        task_separator = Some(&line[body..body + gap]);
        body += gap;
    }
    Some(Item {
        indent: &line[..indent],
        marker: &line[indent..marker_end],
        separator: &line[marker_end..content],
        number: if digits > 0 {
            Some(line[indent..indent + digits].parse().ok()?)
        } else {
            None
        },
        content,
        body,
        task_separator,
    })
}

fn rule(line: &str) -> bool {
    let Some(first) = line.as_bytes().first().copied() else {
        return false;
    };
    matches!(first, b'-' | b'*' | b'_')
        && line.bytes().all(|byte| byte == first || blank(byte))
        && line.bytes().filter(|byte| *byte == first).count() >= 3
}

fn quotes(mut line: &str) -> (usize, &str) {
    let mut depth = 0;
    loop {
        let indent = line.bytes().take_while(|byte| *byte == b' ').count();
        if indent > 3 || line.as_bytes().get(indent) != Some(&b'>') {
            break;
        }
        line = &line[indent + 1..];
        if line.as_bytes().first().is_some_and(|byte| blank(*byte)) {
            line = &line[1..];
        }
        depth += 1;
    }
    (depth, line)
}

#[derive(Clone, Copy)]
struct Fence {
    marker: u8,
    length: usize,
    quotes: usize,
    base_indent: usize,
}
fn fence_marker(line: &str) -> Option<(u8, usize, &str)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some((marker, length, &line[length..]))
}

fn opening(line: &str, depth: usize) -> Option<Fence> {
    let mut body = line;
    let mut list = false;
    // Guard-only parsing is deliberately permissive about separator widths.
    // This also sees nested `- - ````, without treating payload list markers as
    // closers. Advance slices, then calculate the visual base exactly once.
    while let Some(item) = parse_item(body, false) {
        body = &body[item.content..];
        list = true;
    }
    let base_indent = if list {
        line[..line.len() - body.len()]
            .bytes()
            .fold(0, |column, byte| {
                column + if byte == b'\t' { 4 - column % 4 } else { 1 }
            })
    } else {
        let (indent, columns) = whitespace(line, 0);
        body = &line[indent..];
        if columns <= 3 {
            0
        } else {
            columns
        }
    };
    let (marker, length, tail) = fence_marker(body)?;
    (marker != b'`' || !tail.contains('`')).then_some(Fence {
        marker,
        length,
        quotes: depth,
        base_indent,
    })
}

// Unlike syntax coloring, this guard also recognizes common quote/list fence
// openers. Closers NEVER strip a list marker from payload text. Ambiguous
// indented/container cases can stay suppressed longer rather than edit code.
fn fenced(prefix: &str) -> bool {
    let mut open: Option<Fence> = None;
    for line in prefix.split_terminator('\n') {
        let (depth, line) = quotes(line);
        let (indent, columns) = whitespace(line, 0);
        if let Some(state) = open {
            if depth == state.quotes
                && columns >= state.base_indent
                && columns - state.base_indent <= 3
            {
                if let Some((marker, length, tail)) = fence_marker(&line[indent..]) {
                    if marker == state.marker && length >= state.length && tail.bytes().all(blank) {
                        open = None;
                    }
                }
            }
            continue;
        }
        open = opening(line, depth);
    }
    open.is_some()
}

/// Return a complete atomic transaction, or None for unchanged native Enter.
/// Empty items exit in place (keep indent, remove marker/checkbox, no extra LF).
/// Nonempty items split at the caret and preserve all text outside the insertion.
pub fn enter(text: &str, caret: usize) -> Option<TextChange> {
    if !text.is_char_boundary(caret) {
        return None;
    }
    let start = text[..caret].rfind('\n').map_or(0, |index| index + 1);
    let end = text[caret..]
        .find('\n')
        .map_or(text.len(), |index| caret + index);
    let line = &text[start..end];
    if line.len() > MAX_LINE_BYTES {
        return None;
    }
    let parsed = item(line)?;
    // Four-space/tab indentation is ambiguous with indented code. Nested/quoted
    // list editing is intentionally left to native Enter rather than guessing.
    if parsed.indent.len() > 3
        || parsed.indent.contains('\t')
        || rule(&line[parsed.indent.len()..])
        || caret - start < parsed.body
        || fenced(&text[..start])
    {
        return None;
    }
    // Enter on a fence-opening list item must not generate another list marker.
    if opening(line, 0).is_some() {
        return None;
    }
    if line[parsed.body..].bytes().all(blank) {
        return Some(TextChange {
            range: start..end,
            replacement: parsed.indent.into(),
            caret: parsed.indent.len(),
        });
    }
    let marker = if let Some(number) = parsed.number {
        let next = number.checked_add(1)?;
        if next > 999_999_999 {
            return None;
        }
        format!(
            "{next:0width$}{}",
            parsed.marker.chars().next_back()?,
            width = parsed.marker.len() - 1
        )
    } else {
        parsed.marker.to_owned()
    };
    let mut replacement = format!("\n{}{marker}{}", parsed.indent, parsed.separator);
    if let Some(gap) = parsed.task_separator {
        replacement.push_str("[ ]");
        replacement.push_str(gap);
    }
    Some(TextChange {
        range: caret..caret,
        caret: replacement.len(),
        replacement,
    })
}

#[cfg(test)]
#[path = "continuation_tests.rs"]
mod tests;
