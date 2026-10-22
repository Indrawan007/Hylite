//! Narrow preview task edits: validated source bytes, never AST serialization.
use crate::document::Snapshot;
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub item: Range<usize>,
    pub marker: Range<usize>,
    pub checked: bool,
}

/// Parser event ranges may include up to three leading ASCII spaces. Only the
/// canonical three-byte markers are editable; e.g. a tab inside brackets stays
/// visible as a disabled task rather than being normalized or guessed.
pub fn marker_range(source: &str, event: Range<usize>, checked: bool) -> Option<Range<usize>> {
    let raw = source.get(event.clone())?;
    let prefix = raw.len().checked_sub(3)?;
    if prefix > 3 || !raw.as_bytes()[..prefix].iter().all(|byte| *byte == b' ') {
        return None;
    }
    let marker = raw.get(prefix..)?;
    if !matches!((marker, checked), ("[ ]", false) | ("[x]" | "[X]", true)) {
        return None;
    }
    Some(event.start + prefix..event.end)
}

/// Document must additionally authenticate this target against its warm model.
pub fn plan(source: &str, current: Snapshot, target: &Target) -> Option<TextChange> {
    if target.snapshot != current
        || !valid_range(source, &target.item)
        || target.item.start > target.marker.start
        || target.marker.end > target.item.end
        || marker_range(source, target.marker.clone(), target.checked)? != target.marker
    {
        return None;
    }
    let start = target.marker.start.checked_add(1)?;
    Some(TextChange {
        range: start..start + 1,
        replacement: if target.checked { " " } else { "x" }.into(),
        caret: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(source: &str, checked: bool) -> Target {
        let start = source.find('[').unwrap();
        Target {
            snapshot: Snapshot::default(),
            item: 0..source.len(),
            marker: start..start + 3,
            checked,
        }
    }
    fn apply(source: &str, target: &Target) -> String {
        let change = plan(source, Snapshot::default(), target).expect("valid plan");
        assert_eq!(change.range.len(), 1);
        assert_eq!(change.replacement.len(), 1);
        assert_eq!(change.caret, 1);
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        result
    }
    #[test]
    fn toggles_exactly_one_ascii_byte_without_normalizing_unicode_or_whitespace() {
        let source = "- [ ] café🙂\n\n\t";
        assert_eq!(apply(source, &target(source, false)), "- [x] café🙂\n\n\t");
    }
    #[test]
    fn checked_uppercase_and_lowercase_uncheck_without_touching_other_markers() {
        for marker in ["x", "X"] {
            let source = format!("- [{marker}] one\n- [X] keep\n");
            assert_eq!(
                apply(&source, &target(&source, true)),
                "- [ ] one\n- [X] keep\n"
            );
        }
    }
    #[test]
    fn canonical_marker_round_trip_keeps_source_exact() {
        let source = "  - [ ] café🙂  \n";
        let changed = apply(source, &target(source, false));
        assert_eq!(apply(&changed, &target(&changed, true)), source);
    }
    #[test]
    fn parser_prefix_spaces_are_bounded_and_do_not_become_the_replacement_range() {
        for prefix in 0..=3 {
            let source = format!("{}[x]", " ".repeat(prefix));
            assert_eq!(
                marker_range(&source, 0..source.len(), true),
                Some(prefix..source.len())
            );
        }
        assert_eq!(marker_range("    [x]", 0..7, true), None);
    }
    #[test]
    fn noncanonical_markers_tabs_and_state_mismatches_are_rejected() {
        for (source, checked) in [
            ("[\t]", false),
            ("[y]", true),
            ("[ ]", true),
            ("[X]", false),
            ("[x] ", true),
            ("\t[x]", true),
        ] {
            assert_eq!(marker_range(source, 0..source.len(), checked), None);
        }
    }
    #[test]
    fn malformed_utf8_bounds_overflow_and_empty_ranges_are_rejected() {
        let source = "é[x]";
        for range in [0..0, 1..4, 0..usize::MAX, Range { start: 4, end: 2 }] {
            assert_eq!(marker_range(source, range, true), None);
        }
    }
    #[test]
    fn all_snapshot_fields_and_expected_state_must_match() {
        let source = "- [ ] todo";
        let target = target(source, false);
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
            assert!(plan(source, current, &target).is_none());
        }
        assert!(plan("- [x] todo", Snapshot::default(), &target).is_none());
    }
    #[test]
    fn marker_must_be_contained_in_a_valid_item_range() {
        let source = "- [ ] todo";
        for item in [0..3, 4..source.len(), 1..1, 0..usize::MAX] {
            let mut target = target(source, false);
            target.item = item;
            assert!(plan(source, Snapshot::default(), &target).is_none());
        }
    }
    #[test]
    fn normalized_event_prefix_is_not_an_authenticated_marker_target() {
        let source = "-   [ ] todo";
        let mut target = target(source, false);
        target.marker = 2..7;
        assert!(plan(source, Snapshot::default(), &target).is_none());
    }
    #[test]
    fn planner_alone_cannot_authenticate_a_code_block_task() {
        let source = "```md\n- [ ] literal\n```\n";
        // Model membership is intentionally a separate Document responsibility.
        assert!(plan(source, Snapshot::default(), &target(source, false)).is_some());
    }
}
