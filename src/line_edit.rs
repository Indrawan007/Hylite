//! Atomic operations on whole logical Source lines, not Markdown containers.
use crate::text_edit::{line_range, TextChange};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Duplicate,
    Delete,
    MoveUp,
    MoveDown,
}

/// A selection ending at column zero of the next line excludes that next line.
/// Duplicate/move keep the caret's byte column for an empty selection; a
/// nonempty selection puts the caret at the first copied/moved line's start.
/// Selection anchors are not part of TextChange and are not preserved.
pub fn edit(text: &str, selection: Range<usize>, action: Action) -> Option<TextChange> {
    let lines = line_range(text, selection.clone())?;
    let following_lf = text.as_bytes().get(lines.end) == Some(&b'\n');
    match action {
        Action::Duplicate => {
            let body = &text[lines.clone()];
            let column = if selection.is_empty() {
                selection.start - lines.start
            } else {
                0
            };
            let (at, replacement, caret) = if following_lf {
                (lines.end + 1, format!("{body}\n"), column)
            } else {
                // No terminal LF: separate the original and copy without adding
                // a trailing LF to a nonempty last line. An empty EOF line is
                // duplicated by adding one LF, including an empty document.
                (lines.end, format!("\n{body}"), 1 + column)
            };
            Some(TextChange {
                range: at..at,
                replacement,
                caret,
            })
        }
        Action::MoveUp | Action::MoveDown => {
            move_lines(text, selection, lines, action == Action::MoveUp)
        }
        Action::Delete => {
            if text.is_empty() {
                return None;
            }
            let range = if following_lf {
                // Keep the next line intact and put its start under the caret.
                lines.start..lines.end + 1
            } else if lines.start > 0 {
                // Last line (including the empty EOF line): consume the LF that
                // precedes it, rather than leaving an unintended blank line.
                lines.start - 1..lines.end
            } else {
                lines
            };
            Some(TextChange {
                range,
                replacement: String::new(),
                caret: 0,
            })
        }
    }
}

// Swap the touched block with exactly one adjacent content line. The final
// empty line after a terminal LF is a sentinel, not a movable line/target:
// this preserves whether the document ends in LF. Real blank lines before
// that sentinel remain movable. Preserve caret byte-column, not visual x.
fn move_lines(
    text: &str,
    selection: Range<usize>,
    lines: Range<usize>,
    up: bool,
) -> Option<TextChange> {
    if lines.start == text.len() {
        return None;
    }
    let column = if selection.is_empty() {
        selection.start - lines.start
    } else {
        0
    };
    let body = &text[lines.clone()];
    let (range, replacement, caret) = if up {
        let previous_end = lines.start.checked_sub(1)?;
        let previous_start = text[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous = &text[previous_start..previous_end];
        (
            previous_start..lines.end,
            format!("{body}\n{previous}"),
            column,
        )
    } else {
        if text.as_bytes().get(lines.end) != Some(&b'\n') {
            return None;
        }
        let next_start = lines.end + 1;
        if next_start == text.len() {
            return None;
        }
        let next_end = text[next_start..]
            .find('\n')
            .map_or(text.len(), |index| next_start + index);
        let next = &text[next_start..next_end];
        (
            lines.start..next_end,
            format!("{next}\n{body}"),
            next.len() + 1 + column,
        )
    };
    // A blank line moved to EOF in a document without terminal LF would turn
    // a separator into a new terminal LF. Refuse that edge move rather than
    // change the file's newline policy or drop an empty line.
    // Identical swaps are also genuine no-ops, including caret/history.
    if (range.end == text.len() && replacement.ends_with('\n') != text.ends_with('\n'))
        || text[range.clone()] == replacement
    {
        return None;
    }
    Some(TextChange {
        range,
        replacement,
        caret,
    })
}

#[cfg(test)]
#[path = "line_edit_tests.rs"]
mod tests;
