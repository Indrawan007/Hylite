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
}
