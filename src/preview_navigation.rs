//! Revision-stamped block provenance navigation, not a glyph/caret map.
use crate::document::Snapshot;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub range: Range<usize>,
}

impl Target {
    /// Validate revision and UTF-8 bounds only. The Document must additionally
    /// verify membership in its current rendered leaf-block projection.
    pub fn offset(&self, current: Snapshot, source: &str) -> Option<usize> {
        (self.snapshot == current
            && self.range.start < self.range.end
            && source.get(self.range.clone()).is_some())
        .then_some(self.range.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(range: Range<usize>) -> Target {
        Target {
            snapshot: Snapshot::default(),
            range,
        }
    }

    #[test]
    fn block_start_zero_is_valid() {
        assert_eq!(target(0..4).offset(Snapshot::default(), "# é"), Some(0));
    }
    #[test]
    fn offset_is_utf8_bytes_not_unicode_scalar_column() {
        let source = "é🙂\n# next";
        let start = "é🙂\n".len();
        assert_eq!(
            target(start..source.len()).offset(Snapshot::default(), source),
            Some(7)
        );
    }
    #[test]
    fn interior_utf8_endpoints_are_rejected() {
        for range in [1..2, 0..1, 2..3, 3..6] {
            assert_eq!(target(range).offset(Snapshot::default(), "é🙂"), None);
        }
    }
    #[test]
    fn empty_reversed_and_out_of_bounds_ranges_are_rejected() {
        for range in [
            0..0,
            3..3,
            Range { start: 3, end: 2 },
            0..4,
            usize::MAX..usize::MAX,
        ] {
            assert_eq!(target(range).offset(Snapshot::default(), "abc"), None);
        }
    }
    #[test]
    fn all_snapshot_components_must_match() {
        for snapshot in [
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
            assert_eq!(target(0..1).offset(snapshot, "x"), None);
        }
    }
    #[test]
    fn eof_is_a_valid_exclusive_end_but_not_an_empty_target() {
        assert_eq!(target(1..3).offset(Snapshot::default(), "abc"), Some(1));
        assert_eq!(target(3..3).offset(Snapshot::default(), "abc"), None);
    }
    #[test]
    fn raw_source_and_target_are_not_modified() {
        let source = "# café🙂\n\t";
        let before = source.to_owned();
        let target = target(0..source.len());
        let copy = target.clone();
        assert_eq!(target.offset(Snapshot::default(), source), Some(0));
        assert_eq!(target, copy);
        assert_eq!(source, before);
    }
    #[test]
    fn a_new_revision_is_stale_even_if_source_bytes_are_identical() {
        let current = Snapshot {
            revision: 2,
            ..Snapshot::default()
        };
        assert_eq!(target(0..3).offset(current, "abc"), None);
    }
}
