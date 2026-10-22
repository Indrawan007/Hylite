//! Narrow root ATX heading edits. No inline/AST serialization or Setext rewrite.
use crate::document::Snapshot;
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Promote,
    Demote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub block: Range<usize>,
    pub marker: Range<usize>,
    pub level: u8,
}

/// Lexical validation only; Document additionally authenticates root membership.
/// The parser may omit indentation from its range. Neither that indentation nor
/// any separator, inline markup, closing hashes or terminal whitespace is edited.
pub fn marker_range(source: &str, block: &Range<usize>, level: u8) -> Option<Range<usize>> {
    if !(1..=6).contains(&level) || !valid_range(source, block) {
        return None;
    }
    let line_start = source[..block.start].rfind('\n').map_or(0, |at| at + 1);
    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |at| line_start + at);
    let indent = source[line_start..line_end]
        .bytes()
        .take_while(|byte| *byte == b' ')
        .count();
    if indent > 3 || (block.end != line_end && block.end != line_end.saturating_add(1)) {
        return None;
    }
    let start = line_start + indent;
    let end = start.checked_add(usize::from(level))?;
    if block.start > start || end > block.end || end > line_end {
        return None;
    }
    if !source.get(start..end)?.bytes().all(|byte| byte == b'#')
        || !matches!(
            source.as_bytes().get(end),
            None | Some(b' ' | b'\t' | b'\n')
        )
    {
        return None;
    }
    Some(start..end)
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
    match direction {
        Direction::Promote if target.level > 1 => Some(TextChange {
            range: target.marker.end - 1..target.marker.end,
            replacement: String::new(),
            caret: 0,
        }),
        Direction::Demote if target.level < 6 => Some(TextChange {
            range: target.marker.end..target.marker.end,
            replacement: "#".into(),
            caret: 1,
        }),
        _ => None,
    }
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
        let change = plan(
            source,
            Snapshot::default(),
            &target(source, level),
            direction,
        )
        .unwrap();
        assert!(change.range.len() <= 1 && change.replacement.len() <= 1);
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        result
    }
    #[test]
    fn edits_only_a_hash_preserving_inline_raw_unicode_closing_hashes_and_whitespace() {
        let source = "  ###\tcafé🙂 **raw** \\# &amp; ###  \t\n";
        assert_eq!(
            changed(source, 3, Direction::Promote),
            "  ##\tcafé🙂 **raw** \\# &amp; ###  \t\n"
        );
        assert_eq!(
            changed(source, 3, Direction::Demote),
            "  ####\tcafé🙂 **raw** \\# &amp; ###  \t\n"
        );
    }
    #[test]
    fn all_levels_respect_bounds_and_round_trip_exactly() {
        for level in 1..=6 {
            let source = format!("{} text  \n", "#".repeat(level as usize));
            for direction in [Direction::Promote, Direction::Demote] {
                let allowed = match direction {
                    Direction::Promote => level > 1,
                    Direction::Demote => level < 6,
                };
                assert_eq!(
                    plan(
                        &source,
                        Snapshot::default(),
                        &target(&source, level),
                        direction
                    )
                    .is_some(),
                    allowed
                );
                if level < 6 {
                    let demoted = changed(&source, level, Direction::Demote);
                    assert_eq!(changed(&demoted, level + 1, Direction::Promote), source);
                }
            }
        }
    }
    #[test]
    fn empty_heading_eof_tabs_and_bounded_indentation_are_supported() {
        for source in ["##", "##\n", "##\t\n", " ## hi\n", "   ## hi"] {
            assert!(marker_range(source, &(0..source.len()), 2).is_some());
        }
        assert_eq!(marker_range("   ## hi\n", &(3..9), 2), Some(3..5));
    }
    #[test]
    fn setext_container_prefixes_code_and_noncanonical_markers_are_refused() {
        for source in [
            "text\n---\n",
            "> ## text\n",
            "- ## text\n",
            "    ## text\n",
            "\t## text\n",
            "##text\n",
            "### text\n",
            "## text\nbody\n",
        ] {
            assert!(
                marker_range(source, &(0..source.len()), 2).is_none(),
                "{source:?}"
            );
        }
        assert!(marker_range("##\r\n", &(0..4), 2).is_none());
        for level in [0, 7, u8::MAX] {
            assert!(marker_range("## hi", &(0..5), level).is_none());
        }
    }
    #[test]
    fn invalid_utf8_overflow_reversed_empty_and_partial_block_ranges_are_refused() {
        let source = "é\n## hi\n";
        for block in [1..8, 0..usize::MAX, Range { start: 7, end: 3 }, 3..3, 3..6] {
            assert!(marker_range(source, &block, 2).is_none());
        }
        assert_eq!(marker_range(source, &(3..source.len()), 2), Some(3..5));
    }
    #[test]
    fn every_snapshot_field_expected_level_and_marker_must_match() {
        let source = "## hi\n";
        let target = target(source, 2);
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
        for (level, marker) in [(1, 0..2), (2, 1..2), (2, 0..usize::MAX)] {
            let mut bad = target.clone();
            bad.level = level;
            bad.marker = marker;
            assert!(plan(source, Snapshot::default(), &bad, Direction::Demote).is_none());
        }
    }
    #[test]
    fn lexical_planner_does_not_claim_model_membership_in_a_fence_or_nested_list() {
        for source in ["```md\n## text\n```\n", "- parent\n  ## text\n"] {
            let start = source.find("##").unwrap();
            let end = start + source[start..].find('\n').unwrap() + 1;
            assert!(marker_range(source, &(start..end), 2).is_some());
        }
    }
}
