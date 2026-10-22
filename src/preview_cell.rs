//! Explicit raw-Markdown cell drafts, not inline/glyph-based WYSIWYG.
//! Lexical ranges require authentication against the warm root projection.
use crate::document::Snapshot;
use crate::preview_table_edit::{self, MAX_COLUMNS};
use crate::preview_table_row::{MAX_CELLS, MAX_ROWS};
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

pub const MAX_BYTES: usize = 16 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub table: Range<usize>,
    /// Zero is the header; body starts at one.
    pub row: usize,
    pub column: usize,
    pub rows: usize,
    pub columns: usize,
    pub payload: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Previous,
    Next,
}

/// Row-major, bounded, no wrap beyond the first/last real cell.
pub fn neighbor(target: &Target, direction: Direction) -> Option<(usize, usize)> {
    if !(1..=MAX_ROWS).contains(&target.rows)
        || !(1..=MAX_COLUMNS).contains(&target.columns)
        || target.rows * target.columns > MAX_CELLS
        || target.row >= target.rows
        || target.column >= target.columns
    {
        return None;
    }
    let index = target.row * target.columns + target.column;
    let next = match direction {
        Direction::Previous => index.checked_sub(1)?,
        Direction::Next => index.checked_add(1)?,
    };
    (next < target.rows * target.columns).then_some((next / target.columns, next % target.columns))
}

pub fn valid_value(value: &str) -> bool {
    if value.len() > MAX_BYTES || value.chars().any(|c| c.is_control() && c != '\t') {
        return false;
    }
    let mut escaped = false;
    for byte in value.trim_end_matches([' ', '\t']).bytes() {
        if byte == b'|' && !escaped {
            return false;
        }
        escaped = byte == b'\\' && !escaped;
    }
    // A trailing odd backslash would escape the unchanged closing separator.
    !escaped
}

pub fn cells(
    source: &str,
    table: &Range<usize>,
    columns: usize,
    rows: usize,
) -> Option<Vec<Vec<Range<usize>>>> {
    if !valid_range(source, table)
        || !(1..=MAX_ROWS).contains(&rows)
        || !(1..=MAX_COLUMNS).contains(&columns)
        || rows * columns > MAX_CELLS
    {
        return None;
    }
    preview_table_edit::delimiters(source, table, columns)?;
    let mut result = Vec::new();
    let mut at = table.start;
    for (index, line) in source.get(table.clone())?.split_inclusive('\n').enumerate() {
        let start = at;
        at += line.len();
        if index == 1 {
            continue;
        }
        if result.len() == rows {
            return None;
        }
        let raw = line.strip_suffix('\n').unwrap_or(line);
        let indent = raw.bytes().take_while(|b| *b == b' ').count();
        let end = raw.trim_end_matches([' ', '\t']).len();
        if indent > 3
            || raw.as_bytes().get(indent) != Some(&b'|')
            || end <= indent + 1
            || raw.as_bytes().get(end - 1) != Some(&b'|')
        {
            return None;
        }
        let mut pipes = Vec::new();
        let mut escaped = false;
        for (index, byte) in raw.bytes().enumerate().take(end).skip(indent) {
            if byte == b'|' && !escaped {
                pipes.push(index);
            }
            escaped = byte == b'\\' && !escaped;
        }
        if pipes.len() != columns + 1 || pipes.last() != Some(&(end - 1)) {
            return None;
        }
        let mut payloads = Vec::new();
        for pair in pipes.windows(2) {
            let segment = raw.get(pair[0] + 1..pair[1])?;
            let left = segment.len() - segment.trim_start_matches([' ', '\t']).len();
            let payload = segment.trim_matches([' ', '\t']);
            if !valid_value(payload) {
                return None;
            }
            payloads.push(start + pair[0] + 1 + left..start + pair[0] + 1 + left + payload.len());
        }
        result.push(payloads);
    }
    (result.len() == rows).then_some(result)
}

pub fn payload<'a>(source: &'a str, current: Snapshot, target: &Target) -> Option<&'a str> {
    if current != target.snapshot {
        return None;
    }
    let cells = cells(source, &target.table, target.columns, target.rows)?;
    if cells.get(target.row)?.get(target.column)? != &target.payload {
        return None;
    }
    source.get(target.payload.clone())
}
pub fn plan(source: &str, current: Snapshot, target: &Target, value: &str) -> Option<TextChange> {
    let before = payload(source, current, target)?;
    if !valid_value(value) || before == value {
        return None;
    }
    Some(TextChange {
        range: target.payload.clone(),
        replacement: value.into(),
        caret: value.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(source: &str, row: usize, column: usize) -> Target {
        let table = 0..source.len();
        let cells = cells(source, &table, 2, 2).unwrap();
        Target {
            snapshot: Snapshot::default(),
            table,
            row,
            column,
            rows: 2,
            columns: 2,
            payload: cells[row][column].clone(),
        }
    }
    #[test]
    fn exact_payload_changes_preserve_header_delimiter_spacing_entities_and_suffix() {
        let source = "| Name | Count |\n  | --- | --: |\n | café🙂 **raw** | a\\|b &amp; |\n";
        let t = target(source, 1, 0);
        let c = plan(source, Snapshot::default(), &t, "new **bold** é🙂").unwrap();
        assert_eq!(c.caret, c.replacement.len());
        let mut result = source.to_owned();
        result.replace_range(c.range, &c.replacement);
        assert_eq!(
            result,
            "| Name | Count |\n  | --- | --: |\n | new **bold** é🙂 | a\\|b &amp; |\n"
        );
    }
    #[test]
    fn header_and_genuine_empty_cells_are_editable_without_adding_missing_cells() {
        let source = "| a | b |\n|---|---|\n|| x |\n";
        let t = target(source, 1, 0);
        assert_eq!(payload(source, Snapshot::default(), &t), Some(""));
        let t = target(source, 0, 1);
        assert_eq!(payload(source, Snapshot::default(), &t), Some("b"));
        assert!(plan(source, Snapshot::default(), &t, "").is_some());
    }
    #[test]
    fn odd_even_backslash_pipe_parity_and_trailing_backslashes_are_checked() {
        for good in [
            "",
            "hello\tworld",
            "a\\|b",
            "a\\\\\\|b",
            "a\\\\",
            "`a\\|b`",
            "[x](https://example.invalid)",
        ] {
            assert!(valid_value(good), "{good:?}");
        }
        for bad in [
            "a|b",
            "a\\\\|b",
            "trailing\\",
            "odd\\ \t",
            "\n",
            "\r",
            "\0",
            "\u{7f}",
        ] {
            assert!(!valid_value(bad), "{bad:?}");
        }
    }
    #[test]
    fn rectangular_edge_pipe_rows_only_no_padded_extra_or_noncanonical_targets() {
        for source in [
            "|a|b|\n|-|-|\n|x|\n",
            "|a|b|\n|-|-|\n|x|y|z|\n",
            "a|b\n-|-\nx|y\n",
            "|a|b|\n|-|-|\n    |x|y|\n",
            "|a|b|\n|-|-|\n|x|y\\|\n",
        ] {
            assert!(
                cells(source, &(0..source.len()), 2, 2).is_none(),
                "{source:?}"
            );
        }
        let source = "|a|b|\n|-|-|\n|x|y|";
        assert!(cells(source, &(0..source.len()), 2, 2).is_some());
    }
    #[test]
    fn all_snapshot_index_dimension_and_partial_range_forgery_is_rejected() {
        let source = "|a|b|\n|-|-|\n|é|y|\n";
        for case in 0..9 {
            let mut bad = target(source, 1, 0);
            match case {
                0 => bad.snapshot.document_id += 1,
                1 => bad.snapshot.revision += 1,
                2 => bad.snapshot.state_id += 1,
                3 => bad.row = usize::MAX,
                4 => bad.column = usize::MAX,
                5 => bad.rows += 1,
                6 => bad.columns += 1,
                7 => bad.payload.start += 1,
                _ => bad.table.end -= 2,
            }
            assert!(plan(source, Snapshot::default(), &bad, "new").is_none());
        }
    }
    #[test]
    fn exact_byte_budget_noop_and_invalid_ranges_are_refused() {
        assert!(valid_value(&"é".repeat(MAX_BYTES / 2)));
        assert!(!valid_value(&"x".repeat(MAX_BYTES + 1)));
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        let t = target(source, 1, 0);
        assert!(plan(source, Snapshot::default(), &t, "x").is_none());
        for range in [Range { start: 9, end: 1 }, 0..usize::MAX, 1..2] {
            assert!(cells("é", &range, 2, 2).is_none());
        }
        for (cols, rows) in [
            (0, 2),
            (13, 2),
            (2, 129),
            (usize::MAX, usize::MAX),
            (12, 43),
        ] {
            assert!(cells(source, &(0..source.len()), cols, rows).is_none());
        }
    }
    #[test]
    fn empty_payload_uses_whitespace_end_and_preserves_outer_whitespace() {
        let source = "|a|b|\n|-|-|\n| \t | y |\n";
        let t = target(source, 1, 0);
        let c = plan(source, Snapshot::default(), &t, "x").unwrap();
        let mut result = source.to_owned();
        result.replace_range(c.range, &c.replacement);
        assert_eq!(result, "|a|b|\n|-|-|\n| \t x| y |\n");
    }
    #[test]
    fn lexical_table_inside_fence_still_requires_model_provenance() {
        let source = "```\n|a|b|\n|-|-|\n|x|y|\n```\n";
        assert!(cells(source, &(4..source.len() - 4), 2, 2).is_some());
    }
    #[test]
    fn neighbors_are_row_major_including_header_and_cross_row_transitions() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        let t = target(source, 0, 1);
        assert_eq!(neighbor(&t, Direction::Next), Some((1, 0)));
        assert_eq!(
            neighbor(&target(source, 1, 0), Direction::Previous),
            Some((0, 1))
        );
        assert_eq!(
            neighbor(&target(source, 0, 0), Direction::Next),
            Some((0, 1))
        );
    }
    #[test]
    fn first_and_last_neighbors_never_wrap_or_repeat_the_current_cell() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        assert_eq!(neighbor(&target(source, 0, 0), Direction::Previous), None);
        assert_eq!(neighbor(&target(source, 1, 1), Direction::Next), None);
    }
    #[test]
    fn neighbor_invalid_dimensions_indices_and_integer_overflow_are_rejected() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for case in 0..8 {
            let mut t = target(source, 0, 0);
            match case {
                0 => t.rows = 0,
                1 => t.columns = 0,
                2 => t.rows = usize::MAX,
                3 => t.columns = usize::MAX,
                4 => t.row = usize::MAX,
                5 => t.column = usize::MAX,
                6 => {
                    t.rows = 43;
                    t.columns = 12;
                }
                _ => t.columns = 13,
            }
            for direction in [Direction::Previous, Direction::Next] {
                assert!(neighbor(&t, direction).is_none());
            }
        }
    }
    #[test]
    fn neighbor_traversal_and_inverse_cover_every_cell_at_existing_caps() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for (rows, columns) in [(1, 1), (128, 4), (42, 12)] {
            let mut t = target(source, 0, 0);
            t.rows = rows;
            t.columns = columns;
            for index in 0..rows * columns {
                assert_eq!((t.row, t.column), (index / columns, index % columns));
                if let Some((r, c)) = neighbor(&t, Direction::Next) {
                    let mut next = t.clone();
                    next.row = r;
                    next.column = c;
                    assert_eq!(
                        neighbor(&next, Direction::Previous),
                        Some((t.row, t.column))
                    );
                    t = next;
                } else {
                    assert_eq!(index, rows * columns - 1);
                }
            }
        }
    }
    #[test]
    fn neighbor_does_not_authenticate_snapshot_or_payload_provenance() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        let mut t = target(source, 0, 0);
        t.snapshot.revision = 99;
        t.payload = 0..usize::MAX;
        assert_eq!(neighbor(&t, Direction::Next), Some((0, 1)));
        assert!(payload(source, Snapshot::default(), &t).is_none());
    }
}
