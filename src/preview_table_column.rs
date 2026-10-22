//! Explicit column insertion/deletion; retained raw segments stay byte-for-byte.
//! Lexical checks are not provenance: Document authenticates and preflights roots.
use crate::document::Snapshot;
use crate::preview_cell;
use crate::preview_table_edit::{self, MAX_COLUMNS};
use crate::preview_table_row::{MAX_CELLS, MAX_ROWS};
use crate::text_edit::TextChange;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    InsertBefore,
    InsertAfter,
    Delete,
}
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InsertBefore => "Insert before",
            Self::InsertAfter => "Insert after",
            Self::Delete => "Delete",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub table: Range<usize>,
    pub column: usize,
    pub columns: usize,
    pub rows: usize,
    pub header: Range<usize>,
    pub marker: Range<usize>,
}

pub fn can_insert(rows: usize, columns: usize) -> bool {
    (1..=MAX_ROWS).contains(&rows)
        && (1..MAX_COLUMNS).contains(&columns)
        && rows * (columns + 1) <= MAX_CELLS
}

/// Deletion can free capacity at growth caps, but cannot remove the last column.
pub fn can_delete(rows: usize, columns: usize) -> bool {
    (1..=MAX_ROWS).contains(&rows)
        && (2..=MAX_COLUMNS).contains(&columns)
        && rows * columns <= MAX_CELLS
}

/// Full-line canonical separators, ignoring escaped pipes even in raw code text.
fn pipes(line: &str, columns: usize) -> Option<Vec<usize>> {
    let raw = line.strip_suffix('\n')?;
    let indent = raw.bytes().take_while(|b| *b == b' ').count();
    let end = raw.trim_end_matches([' ', '\t']).len();
    if indent > 3
        || raw.as_bytes().get(indent) != Some(&b'|')
        || end <= indent + 1
        || raw.as_bytes().get(end - 1) != Some(&b'|')
    {
        return None;
    }
    let mut result = Vec::new();
    let mut escaped = false;
    for (index, byte) in raw.bytes().enumerate().take(end).skip(indent) {
        if byte == b'|' && !escaped {
            if result.len() == columns + 1 {
                return None;
            }
            result.push(index);
        }
        escaped = byte == b'\\' && !escaped;
    }
    (result.len() == columns + 1 && result.last() == Some(&(end - 1))).then_some(result)
}

/// All physical lines, including delimiter, must have outer pipes and LF.
/// This is bounded lexical eligibility, never root/snapshot authentication.
pub fn full_lines(source: &str, table: &Range<usize>, columns: usize, rows: usize) -> bool {
    if !(1..=MAX_COLUMNS).contains(&columns) || !(1..=MAX_ROWS).contains(&rows) {
        return false;
    }
    let Some(raw) = source.get(table.clone()) else {
        return false;
    };
    let mut count = 0;
    for line in raw.split_inclusive('\n') {
        if count > rows || pipes(line, columns).is_none() {
            return false;
        }
        count += 1;
    }
    count == rows + 1
}

pub fn plan(
    source: &str,
    current: Snapshot,
    target: &Target,
    action: Action,
) -> Option<TextChange> {
    if current != target.snapshot
        || !match action {
            Action::Delete => can_delete(target.rows, target.columns),
            Action::InsertBefore | Action::InsertAfter => can_insert(target.rows, target.columns),
        }
        || target.column >= target.columns
    {
        return None;
    }
    let cells = preview_cell::cells(source, &target.table, target.columns, target.rows)?;
    if cells.first()?.get(target.column) != Some(&target.header) {
        return None;
    }
    let delimiters = preview_table_edit::delimiters(source, &target.table, target.columns)?;
    if delimiters.get(target.column)?.source != target.marker {
        return None;
    }
    let mut replacement = String::new();
    let mut caret = 0;
    let mut count = 0;
    for (index, line) in source
        .get(target.table.clone())?
        .split_inclusive('\n')
        .enumerate()
    {
        let separators = pipes(line, target.columns)?;
        if action == Action::Delete {
            // Keep the left separator, delete only this raw segment and its
            // right separator. Existing neighbors and trailing bytes stay raw.
            let start = separators.get(target.column)? + 1;
            let end = separators.get(target.column + 1)? + 1;
            replacement.push_str(&line[..start]);
            replacement.push_str(&line[end..]);
        } else {
            let boundary = target.column + usize::from(action == Action::InsertAfter);
            let at = separators.get(boundary)? + 1;
            if index == 0 {
                caret = at + 1; // Between the two spaces of the new header cell.
            }
            replacement.push_str(&line[..at]);
            replacement.push_str(if index == 1 { " --- |" } else { "  |" });
            replacement.push_str(&line[at..]);
        }
        count += 1;
    }
    if count != target.rows + 1 {
        return None;
    }
    if action == Action::Delete {
        let remaining = preview_cell::cells(
            &replacement,
            &(0..replacement.len()),
            target.columns - 1,
            target.rows,
        )?;
        let neighbor = target.column.min(target.columns - 2);
        caret = remaining.first()?.get(neighbor)?.start;
    }
    Some(TextChange {
        range: target.table.clone(),
        replacement,
        caret,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(source: &str, columns: usize, rows: usize, column: usize) -> Target {
        let table = 0..source.len();
        Target {
            snapshot: Snapshot::default(),
            column,
            columns,
            rows,
            header: preview_cell::cells(source, &table, columns, rows).unwrap()[0][column].clone(),
            marker: preview_table_edit::delimiters(source, &table, columns).unwrap()[column]
                .source
                .clone(),
            table,
        }
    }
    fn apply(source: &str, target: &Target, action: Action) -> String {
        let change = plan(source, Snapshot::default(), target, action).unwrap();
        let mut output = source.to_owned();
        output.replace_range(change.range, &change.replacement);
        output
    }
    #[test]
    fn before_and_after_preserve_unicode_raw_segments_spacing_and_old_alignment() {
        let source = "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n";
        assert_eq!(
            apply(source, &target(source, 2, 2, 0), Action::InsertBefore),
            "|  | Name | Count |\n  | --- | ---  | --: |\n |  | café🙂 **raw** | a\\|b &amp; |\n"
        );
        assert_eq!(
            apply(source, &target(source, 2, 2, 1), Action::InsertAfter),
            "| Name | Count |  |\n  | ---  | --: | --- |\n | café🙂 **raw** | a\\|b &amp; |  |\n"
        );
    }
    #[test]
    fn middle_insertion_ignores_escaped_pipes_and_preserves_trailing_tabs() {
        let source = "| a\\|é | `b` |\t\n| :-- | --: |\t\n   | c\\\\\\|d | **🙂** |\t\n";
        let expected =
            "| a\\|é |  | `b` |\t\n| :-- | --- | --: |\t\n   | c\\\\\\|d |  | **🙂** |\t\n";
        assert_eq!(
            apply(source, &target(source, 2, 2, 0), Action::InsertAfter),
            expected
        );
        assert_eq!(
            apply(source, &target(source, 2, 2, 1), Action::InsertBefore),
            expected
        );
    }
    #[test]
    fn header_only_and_empty_cells_are_supported_and_caret_is_inside_new_header() {
        for source in ["|a|\n|-|\n", "|  |\n|---|\n|  |\n"] {
            let rows = if source.lines().count() == 2 { 1 } else { 2 };
            let t = target(source, 1, rows, 0);
            for (action, caret) in [
                (Action::InsertBefore, 2),
                (Action::InsertAfter, source.find('\n').unwrap() + 1),
            ] {
                let change = plan(source, Snapshot::default(), &t, action).unwrap();
                assert_eq!(change.caret, caret);
                let new = apply(source, &t, action);
                assert!(preview_cell::cells(&new, &(0..new.len()), 2, rows).is_some());
            }
        }
    }
    #[test]
    fn only_table_range_changes_even_with_indented_header_prefix_and_suffix() {
        let raw = "|a|b|\n  |---|--:|\n  |é|🙂|\n";
        let source = format!("before\n\n   {raw}\nafter\n\t");
        let mut t = target(raw, 2, 2, 1);
        let offset = 11;
        t.table = offset..offset + raw.len();
        t.header = t.header.start + offset..t.header.end + offset;
        t.marker = t.marker.start + offset..t.marker.end + offset;
        assert_eq!(
            apply(&source, &t, Action::InsertBefore),
            "before\n\n   |a|  |b|\n  |---| --- |--:|\n  |é|  |🙂|\n\nafter\n\t"
        );
    }
    #[test]
    fn stale_forged_dimensions_header_marker_and_ranges_are_rejected() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for action in [Action::InsertBefore, Action::InsertAfter] {
            for case in 0..12 {
                let mut t = target(source, 2, 2, 0);
                match case {
                    0 => t.snapshot.document_id += 1,
                    1 => t.snapshot.revision += 1,
                    2 => t.snapshot.state_id += 1,
                    3 => t.table.end -= 1,
                    4 => t.column = usize::MAX,
                    5 => t.columns = usize::MAX,
                    6 => t.rows = usize::MAX,
                    7 => t.header.start += 1,
                    8 => t.header.end += 1,
                    9 => t.marker.start += 1,
                    10 => t.marker.end += 1,
                    _ => t.table.start = usize::MAX,
                }
                assert!(plan(source, Snapshot::default(), &t, action).is_none());
            }
        }
    }
    #[test]
    fn growth_caps_check_column_row_cell_counts_before_allocation() {
        for (rows, columns, allowed) in [
            (1, 1, true),
            (128, 3, true),
            (128, 4, false),
            (42, 11, true),
            (47, 10, false),
            (1, 12, false),
            (0, 2, false),
            (129, 1, false),
            (usize::MAX, usize::MAX, false),
        ] {
            assert_eq!(can_insert(rows, columns), allowed);
        }
        let source = format!(
            "|{}\n|{}\n|{}\n",
            "h|".repeat(12),
            "---|".repeat(12),
            "x|".repeat(12)
        );
        let t = target(&source, 12, 2, 0);
        assert!(plan(&source, Snapshot::default(), &t, Action::InsertBefore).is_none());
    }
    #[test]
    fn ragged_non_pipe_delimiter_non_lf_and_unescaped_extra_cells_are_rejected() {
        for source in [
            "|a|b|\n|-|-|\n|x|\n",
            "a|b\n-|-\nx|y\n",
            "|a|b|\n-|-\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "|a|b|\n|-|-|\n|x|y|z|\n",
        ] {
            let t = Target {
                snapshot: Snapshot::default(),
                table: 0..source.len(),
                column: 0,
                columns: 2,
                rows: 2,
                header: 1..2,
                marker: 7..8,
            };
            assert!(!full_lines(source, &t.table, t.columns, t.rows));
            assert!(plan(source, Snapshot::default(), &t, Action::InsertAfter).is_none());
        }
    }
    #[test]
    fn lexical_plan_cannot_authenticate_a_fenced_table() {
        let raw = "|a|b|\n|-|-|\n|x|y|\n";
        let source = format!("```\n{raw}```\n");
        let mut t = target(raw, 2, 2, 0);
        t.table = 4..4 + raw.len();
        t.header = t.header.start + 4..t.header.end + 4;
        t.marker = t.marker.start + 4..t.marker.end + 4;
        assert!(plan(&source, Snapshot::default(), &t, Action::InsertBefore).is_some());
    }
    #[test]
    fn delete_first_or_last_removes_only_selected_segments_and_keeps_old_alignment() {
        let source = "| Name | Count |\n  | ---  | --: |\n | café🙂 **raw** | a\\|b &amp; |\n";
        assert_eq!(
            apply(source, &target(source, 2, 2, 0), Action::Delete),
            "| Count |\n  | --: |\n | a\\|b &amp; |\n"
        );
        assert_eq!(
            apply(source, &target(source, 2, 2, 1), Action::Delete),
            "| Name |\n  | ---  |\n | café🙂 **raw** |\n"
        );
    }
    #[test]
    fn delete_middle_handles_escaped_pipes_raw_markup_and_header_neighbor_caret() {
        let source = "| A | `B` | é🙂 |\t\n| :-- | --- | --: |\t\n   | a\\|b | **gone** | keep\\\\\\|x |\t\n";
        let expected = "| A | é🙂 |\t\n| :-- | --: |\t\n   | a\\|b | keep\\\\\\|x |\t\n";
        let t = target(source, 3, 2, 1);
        let change = plan(source, Snapshot::default(), &t, Action::Delete).unwrap();
        assert_eq!(change.replacement, expected);
        assert_eq!(change.caret, expected.find("é🙂").unwrap());
        let t = target(source, 3, 2, 2);
        let change = plan(source, Snapshot::default(), &t, Action::Delete).unwrap();
        assert_eq!(change.caret, change.replacement.find("`B`").unwrap());
    }
    #[test]
    fn last_column_cannot_be_deleted_including_empty_and_header_only_tables() {
        for source in ["|a|\n|-|\n", "|  |\n|---|\n|  |\n"] {
            let rows = source.lines().count() - 1;
            assert!(plan(
                source,
                Snapshot::default(),
                &target(source, 1, rows, 0),
                Action::Delete
            )
            .is_none());
        }
        for (rows, columns) in [
            (0, 2),
            (129, 2),
            (1, 1),
            (1, 13),
            (usize::MAX, usize::MAX),
            (128, 5),
        ] {
            assert!(!can_delete(rows, columns));
        }
    }
    #[test]
    fn delete_frees_capacity_even_when_insert_at_column_or_cell_limit_is_forbidden() {
        for (columns, rows) in [(12, 42), (4, 128)] {
            let source = format!(
                "|{}\n|{}\n{}",
                "h|".repeat(columns),
                "---|".repeat(columns),
                format!("|{}\n", "x|".repeat(columns)).repeat(rows - 1)
            );
            assert!(!can_insert(rows, columns));
            assert!(can_delete(rows, columns));
            let t = target(&source, columns, rows, columns - 1);
            let result = apply(&source, &t, Action::Delete);
            assert!(preview_cell::cells(&result, &(0..result.len()), columns - 1, rows).is_some());
        }
    }
    #[test]
    fn delete_requires_exact_snapshot_dimensions_column_header_and_marker() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for case in 0..12 {
            let mut t = target(source, 2, 2, 0);
            match case {
                0 => t.snapshot.document_id += 1,
                1 => t.snapshot.revision += 1,
                2 => t.snapshot.state_id += 1,
                3 => t.table.end -= 1,
                4 => t.column = usize::MAX,
                5 => t.columns = usize::MAX,
                6 => t.rows = usize::MAX,
                7 => t.header.start += 1,
                8 => t.header.end += 1,
                9 => t.marker.start += 1,
                10 => t.marker.end += 1,
                _ => t.table.start = usize::MAX,
            }
            assert!(plan(source, Snapshot::default(), &t, Action::Delete).is_none());
        }
    }
    #[test]
    fn delete_rejects_noncanonical_lines_but_lexical_scans_cannot_authenticate_fences() {
        for source in [
            "|a|b|\n|-|-|\n|x|\n",
            "a|b\n-|-\nx|y\n",
            "|a|b|\n-|-\n|x|y|\n",
            "|a|b|\n|-|-|\n|x|y|",
            "|a|b|\n|-|-|\n|x|y|z|\n",
        ] {
            let t = Target {
                snapshot: Snapshot::default(),
                table: 0..source.len(),
                column: 0,
                columns: 2,
                rows: 2,
                header: 1..2,
                marker: 7..8,
            };
            assert!(plan(source, Snapshot::default(), &t, Action::Delete).is_none());
        }
        let raw = "|a|b|\n|-|-|\n|x|y|\n";
        let source = format!("```\n{raw}```\n");
        let mut t = target(raw, 2, 2, 0);
        t.table = 4..4 + raw.len();
        t.header = t.header.start + 4..t.header.end + 4;
        t.marker = t.marker.start + 4..t.marker.end + 4;
        assert!(plan(&source, Snapshot::default(), &t, Action::Delete).is_some());
    }
    #[test]
    fn delete_preserves_prefix_indented_header_suffix_tabs_and_empty_neighbor() {
        let raw = "|a|  |\t\n  |---|--:|\t\n |é|  |\t\n";
        let source = format!("before\n\n   {raw}\nafter\n\t");
        let mut t = target(raw, 2, 2, 0);
        t.table = 11..11 + raw.len();
        t.header = t.header.start + 11..t.header.end + 11;
        t.marker = t.marker.start + 11..t.marker.end + 11;
        let change = plan(&source, Snapshot::default(), &t, Action::Delete).unwrap();
        assert_eq!(change.caret, 3); // Empty payload at the closing pipe after two spaces.
        assert_eq!(
            apply(&source, &t, Action::Delete),
            "before\n\n   |  |\t\n  |--:|\t\n |  |\t\n\nafter\n\t"
        );
    }
}
