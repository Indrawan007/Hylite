//! UTF-8 byte-range text transactions, independent of native widget navigation.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineStyle {
    Bold,
    Italic,
    Code,
}

pub struct TextChange {
    pub range: Range<usize>,
    pub replacement: String,
    /// UTF-8 byte offset relative to the beginning of the replacement.
    pub caret: usize,
}

pub fn valid_range(text: &str, range: &Range<usize>) -> bool {
    range.start <= range.end
        && range.end <= text.len()
        && text.is_char_boundary(range.start)
        && text.is_char_boundary(range.end)
}

pub fn inline(text: &str, range: Range<usize>, style: InlineStyle) -> Option<TextChange> {
    if !valid_range(text, &range) {
        return None;
    }
    let selected = &text[range.clone()];
    let marker = match style {
        InlineStyle::Bold => "**",
        InlineStyle::Italic => "*",
        InlineStyle::Code => "`",
    };
    // This is a source shortcut, not an AST-aware Markdown formatter. For inline
    // code containing backticks/newlines, refuse rather than create broken code.
    if style == InlineStyle::Code
        && (selected.contains('\n')
            || (selected.contains('`')
                && !(selected.len() > 2
                    && selected.starts_with('`')
                    && selected.ends_with('`')
                    && !selected[1..selected.len() - 1].contains('`'))))
    {
        return None;
    }
    let (replacement, caret) = if selected.len() > marker.len() * 2
        && selected.starts_with(marker)
        && selected.ends_with(marker)
        && (style != InlineStyle::Italic
            || ((selected.bytes().take_while(|byte| *byte == b'*').count() & 1) == 1
                && (selected
                    .bytes()
                    .rev()
                    .take_while(|byte| *byte == b'*')
                    .count()
                    & 1)
                    == 1))
    {
        let inner = selected[marker.len()..selected.len() - marker.len()].to_owned();
        let caret = inner.len();
        (inner, caret)
    } else {
        let inner = if selected.is_empty() {
            "text"
        } else {
            selected
        };
        (
            format!("{marker}{inner}{marker}"),
            marker.len() + inner.len(),
        )
    };
    Some(TextChange {
        range,
        replacement,
        caret,
    })
}

/// Exact logical-line extent shared by indentation and block formatting.
pub(crate) fn line_range(text: &str, selection: Range<usize>) -> Option<Range<usize>> {
    if !valid_range(text, &selection) {
        return None;
    }
    let start = text[..selection.start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let end_position =
        if !selection.is_empty() && text.as_bytes().get(selection.end - 1) == Some(&b'\n') {
            selection.end - 1
        } else {
            selection.end
        };
    let end = text[end_position..]
        .find('\n')
        .map_or(text.len(), |index| end_position + index);
    Some(start..end)
}

/// Indent/unindent whole logical lines touched by a selection. A selection
/// ending at the next line's column zero does not include that next line.
pub fn indent(text: &str, selection: Range<usize>, width: u8, remove: bool) -> Option<TextChange> {
    let range = line_range(text, selection)?;
    let spaces = " ".repeat(usize::from(width.max(1)));
    let replacement = text[range.clone()]
        .split('\n')
        .map(|line| {
            if remove {
                if let Some(line) = line.strip_prefix('\t') {
                    return line.to_owned();
                }
                let count = line
                    .bytes()
                    .take(usize::from(width.max(1)))
                    .take_while(|byte| *byte == b' ')
                    .count();
                line[count..].to_owned()
            } else {
                format!("{spaces}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let caret = replacement.len();
    Some(TextChange {
        range,
        replacement,
        caret,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(text: &str, change: TextChange) -> String {
        assert!(change.replacement.is_char_boundary(change.caret));
        format!(
            "{}{}{}",
            &text[..change.range.start],
            change.replacement,
            &text[change.range.end..]
        )
    }

    #[test]
    fn inline_formatting_preserves_unicode_and_toggles_selected_markers() {
        let text = "é✨";
        assert_eq!(
            apply(
                text,
                inline(text, 0..text.len(), InlineStyle::Bold).unwrap()
            ),
            "**é✨**"
        );
        assert_eq!(
            apply(
                "**é✨**",
                inline("**é✨**", 0..9, InlineStyle::Bold).unwrap()
            ),
            "é✨"
        );
        assert_eq!(
            apply("", inline("", 0..0, InlineStyle::Italic).unwrap()),
            "*text*"
        );
    }

    #[test]
    fn unsafe_byte_ranges_and_unsupported_inline_code_are_refused() {
        assert!(inline("é", 1..2, InlineStyle::Bold).is_none());
        assert!(inline("abc", Range { start: 3, end: 1 }, InlineStyle::Bold).is_none());
        assert!(inline("a`b", 0..3, InlineStyle::Code).is_none());
        assert!(inline("a\nb", 0..3, InlineStyle::Code).is_none());
    }

    #[test]
    fn indent_touches_whole_lines_but_excludes_the_next_line_at_column_zero() {
        let text = "alpha\né✨\nomega";
        let change = indent(text, 2..12, 2, false).unwrap();
        assert_eq!(apply(text, change), "  alpha\n  é✨\nomega");
    }

    #[test]
    fn unindent_removes_one_tab_or_up_to_the_configured_spaces() {
        let text = "\talpha\n  beta\n gamma\nz";
        let change = indent(text, 0..text.len(), 2, true).unwrap();
        assert_eq!(apply(text, change), "alpha\nbeta\ngamma\nz");
    }

    #[test]
    fn collapsed_indent_uses_the_current_line_and_keeps_newlines() {
        assert_eq!(
            apply("a\nb\n", indent("a\nb\n", 2..2, 4, false).unwrap()),
            "a\n    b\n"
        );
    }

    #[test]
    fn italic_wraps_bold_and_can_toggle_outer_italic_back_off() {
        let bold = "**é✨**";
        let combined = apply(
            bold,
            inline(bold, 0..bold.len(), InlineStyle::Italic).unwrap(),
        );
        assert_eq!(combined, "***é✨***");
        assert_eq!(
            apply(
                &combined,
                inline(&combined, 0..combined.len(), InlineStyle::Italic).unwrap()
            ),
            bold
        );
    }
}
