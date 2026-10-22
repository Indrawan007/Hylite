//! Revision-stamped block/cell provenance navigation, not a glyph/caret map.
use crate::document::Snapshot;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Block,
    TableCell {
        table: Range<usize>,
        row: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub range: Range<usize>,
    pub origin: Origin,
}

impl Target {
    pub fn block(snapshot: Snapshot, range: Range<usize>) -> Self {
        Self {
            snapshot,
            range,
            origin: Origin::Block,
        }
    }

    pub fn cell(
        snapshot: Snapshot,
        table: Range<usize>,
        row: usize,
        column: usize,
        range: Range<usize>,
    ) -> Self {
        Self {
            snapshot,
            range,
            origin: Origin::TableCell { table, row, column },
        }
    }

    /// Validate revision and UTF-8 bounds only. The Document must additionally
    /// verify exact block/cell membership in its current cached projection.
    pub fn offset(&self, current: Snapshot, source: &str) -> Option<usize> {
        let within_origin = match &self.origin {
            Origin::Block => true,
            Origin::TableCell { table, .. } => {
                table.start <= self.range.start
                    && self.range.end <= table.end
                    && source.get(table.clone()).is_some()
            }
        };
        (within_origin
            && self.snapshot == current
            && self.range.start < self.range.end
            && source.get(self.range.clone()).is_some())
        .then_some(self.range.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(range: Range<usize>) -> Target {
        Target::block(Snapshot::default(), range)
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
    #[test]
    fn cell_scope_requires_a_valid_containing_table_range() {
        let source = "abcé";
        let snapshot = Snapshot::default();
        for table in [0..2, 2..5, 0..6, 0..4, Range { start: 5, end: 0 }] {
            let target = Target::cell(snapshot, table, 0, 1, 1..3);
            assert_eq!(target.offset(snapshot, source), None);
        }
        assert_eq!(
            Target::cell(snapshot, 0..5, 1, 1, 1..3).offset(snapshot, source),
            Some(1)
        );
    }
    #[test]
    fn cell_offset_is_raw_provenance_not_styled_display_text() {
        let source = "é🙂|**3**";
        let start = "é🙂|".len();
        let snapshot = Snapshot::default();
        let target = Target::cell(snapshot, 0..source.len(), 1, 1, start..source.len());
        assert_eq!(target.offset(snapshot, source), Some(7));
        assert_eq!(&source[target.range.clone()], "**3**");
    }
    #[test]
    fn empty_padding_and_stale_cell_snapshots_are_rejected() {
        let snapshot = Snapshot::default();
        assert_eq!(
            Target::cell(snapshot, 0..3, 1, 1, 3..3).offset(snapshot, "a|b"),
            None
        );
        let current = Snapshot {
            revision: 1,
            ..snapshot
        };
        assert_eq!(
            Target::cell(snapshot, 0..3, 1, 1, 2..3).offset(current, "a|b"),
            None
        );
    }
    #[test]
    fn origin_and_indices_are_identity_but_model_lookup_is_document_responsibility() {
        let snapshot = Snapshot::default();
        let cell = Target::cell(snapshot, 0..3, usize::MAX, usize::MAX, 0..1);
        assert_ne!(cell, Target::block(snapshot, 0..1));
        assert_eq!(cell.clone(), cell);
        // Bounds resolver alone does not authenticate row/column indices.
        assert_eq!(cell.offset(snapshot, "a|b"), Some(0));
    }
}
