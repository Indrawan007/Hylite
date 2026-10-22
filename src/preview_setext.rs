//! Root Setext heading level changes: only underline bytes, no ATX conversion.
use crate::document::Snapshot;
use crate::preview_heading::Direction;
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub block: Range<usize>,
    pub marker: Range<usize>,
    pub level: u8,
}

/// Lexical validation, not authentication. Document also requires exact root
/// membership in its warm model. Paragraph body may span multiple physical lines.
pub fn marker_range(source: &str, block: &Range<usize>, level: u8) -> Option<Range<usize>> {
    if !matches!(level, 1 | 2) || !valid_range(source, block) || block.is_empty() {
        return None;
    }
    let first_line = source[..block.start].rfind('\n').map_or(0, |at| at + 1);
    let prefix = source.get(first_line..block.start)?;
    if prefix.len() > 3 || !prefix.bytes().all(|byte| byte == b' ') {
        return None;
    }
    // Require the whole final physical line, allowing the parser's final LF.
    if block.end < source.len()
        && source.as_bytes()[block.end - 1] != b'\n'
        && source.as_bytes()[block.end] != b'\n'
    {
        return None;
    }
    let end = block.end - usize::from(source.as_bytes()[block.end - 1] == b'\n');
    let line_start = source[..end].rfind('\n')? + 1;
    if line_start <= block.start
        || !source[block.start..line_start]
            .bytes()
            .any(|byte| !matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
    {
        return None;
    }
    let line = source.get(line_start..end)?;
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let expected = if level == 1 { b'=' } else { b'-' };
    let count = line.as_bytes()[indent..]
        .iter()
        .take_while(|byte| **byte == expected)
        .count();
    if count == 0
        || !line.as_bytes()[indent + count..]
            .iter()
            .all(|byte| matches!(byte, b' ' | b'\t'))
    {
        return None;
    }
    Some(line_start + indent..line_start + indent + count)
}

pub fn plan(
    source: &str,
    current: Snapshot,
    target: &Target,
    direction: Direction,
) -> Option<TextChange> {
    if current != target.snapshot
        || marker_range(source, &target.block, target.level)? != target.marker
    {
        return None;
    }
    let replacement = match (target.level, direction) {
        (1, Direction::Demote) => "-",
        (2, Direction::Promote) => "=",
        _ => return None,
    };
    Some(TextChange {
        range: target.marker.clone(),
        replacement: replacement.repeat(target.marker.len()),
        caret: target.marker.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(source: &str, level: u8) -> Target {
        let block = 0..source.len();
        Target {
            snapshot: Snapshot::default(),
            marker: marker_range(source, &block, level).unwrap(),
            block,
            level,
        }
    }
    fn changed(source: &str, level: u8, direction: Direction) -> String {
        let target = target(source, level);
        let change = plan(source, Snapshot::default(), &target, direction).unwrap();
        assert_eq!(change.range, target.marker);
        assert_eq!(change.replacement.len(), target.marker.len());
        assert_eq!(change.caret, target.marker.len());
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        result
    }
    #[test]
    fn preserves_multiline_raw_body_indent_marker_count_and_terminal_whitespace() {
        let source = "  café🙂 **raw** &amp;  \ncontinued `code`\n   =====  \t\n";
        let expected = "  café🙂 **raw** &amp;  \ncontinued `code`\n   -----  \t\n";
        assert_eq!(changed(source, 1, Direction::Demote), expected);
        assert_eq!(changed(expected, 2, Direction::Promote), source);
    }
    #[test]
    fn one_to_many_markers_eof_and_indentation_are_not_normalized() {
        for count in [1, 2, 3, 17, 512] {
            for indent in 0..=3 {
                for suffix in ["", "\n", "  \t\n"] {
                    let source =
                        format!("title\n{}{}{suffix}", " ".repeat(indent), "=".repeat(count));
                    let expected =
                        format!("title\n{}{}{suffix}", " ".repeat(indent), "-".repeat(count));
                    assert_eq!(changed(&source, 1, Direction::Demote), expected);
                    assert_eq!(changed(&expected, 2, Direction::Promote), source);
                }
            }
        }
    }
    #[test]
    fn directions_stop_at_setext_levels_one_and_two_without_converting_to_atx() {
        for (source, level, direction) in [
            ("title\n===\n", 1, Direction::Promote),
            ("title\n---\n", 2, Direction::Demote),
        ] {
            assert!(plan(
                source,
                Snapshot::default(),
                &target(source, level),
                direction
            )
            .is_none());
        }
    }
    #[test]
    fn rejects_noncanonical_underlines_blank_body_container_prefix_and_invalid_level() {
        for source in [
            "title\n= =\n",
            "title\n==--\n",
            "title\n    ===\n",
            "title\n\t===\n",
            "title\n===x\n",
            "title\n===\r\n",
            "\n===\n",
            "  \n===\n",
            "> title\n> ===\n",
            "title\n---\n",
        ] {
            assert!(
                marker_range(source, &(0..source.len()), 1).is_none(),
                "{source:?}"
            );
        }
        for level in [0, 3, 6, u8::MAX] {
            assert!(marker_range("a\n===\n", &(0..6), level).is_none());
        }
    }
    #[test]
    fn invalid_bounds_utf8_partial_lines_and_expected_marker_are_rejected() {
        let source = "é\n===\n";
        for block in [1..7, 0..usize::MAX, Range { start: 6, end: 2 }, 0..0, 0..4] {
            assert!(marker_range(source, &block, 1).is_none());
        }
        let mut target = target(source, 1);
        target.marker = 3..5;
        assert!(plan(source, Snapshot::default(), &target, Direction::Demote).is_none());
    }
    #[test]
    fn full_snapshot_and_expected_level_must_match() {
        let source = "title\n===\n";
        let target = target(source, 1);
        for current in [
            Snapshot {
                document_id: 1,
                ..Snapshot::default()
            },
            Snapshot {
                revision: 1,
                ..Snapshot::default()
            },
            Snapshot {
                state_id: 1,
                ..Snapshot::default()
            },
        ] {
            assert!(plan(source, current, &target, Direction::Demote).is_none());
        }
        let mut bad = target;
        bad.level = 2;
        assert!(plan(source, Snapshot::default(), &bad, Direction::Promote).is_none());
    }
    #[test]
    fn lexical_validation_alone_does_not_authenticate_fenced_or_nested_headings() {
        // An underline scan cannot decide that this body belongs to a list.
        let list = "- title\n  ===\n";
        assert!(marker_range(list, &(0..list.len()), 1).is_some());
        for source in ["```md\ntitle\n===\n```\n", "- parent\n  title\n  ===\n"] {
            let start = source.find("title").unwrap();
            let end = source.find("===").unwrap() + 4;
            assert!(marker_range(source, &(start..end), 1).is_some());
        }
    }
}
