//! Conservative root-table body-row edits: whole LF-terminated pipe-leading lines.
//! Lexical validation is not provenance; Document authenticates the warm model.
use crate::document::Snapshot;
use crate::preview_table_edit::{self, MAX_COLUMNS};
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

pub const MAX_ROWS: usize = 128;
pub const MAX_CELLS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Duplicate,
    Delete,
}
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Duplicate => "Duplicate",
            Self::Delete => "Delete",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub table: Range<usize>,
    /// Body rows are numbered 1..; zero would be the protected header.
    pub row: usize,
    pub rows: usize,
    pub columns: usize,
    pub line: Range<usize>,
}

pub fn can_duplicate(rows: usize, columns: usize) -> bool {
    (1..MAX_ROWS).contains(&rows)
        && (1..=MAX_COLUMNS).contains(&columns)
        && (rows + 1) * columns <= MAX_CELLS
}

pub fn lines(source: &str, table: &Range<usize>, columns: usize) -> Option<Vec<Range<usize>>> {
    if !valid_range(source, table) {
        return None;
    }
    preview_table_edit::delimiters(source, table, columns)?;
    let raw = source.get(table.clone())?;
    if !raw.ends_with('\n') {
        return None;
    }
    let mut at = table.start;
    let mut result = Vec::new();
    for (index, line) in raw.split_inclusive('\n').enumerate() {
        if index >= 2 {
            let indent = line.bytes().take_while(|b| *b == b' ').count();
            if indent > 3 || !line[indent..].starts_with('|') {
                return None;
            }
            result.push(at..at + line.len());
        }
        at += line.len();
    }
    if result.len() + 1 > MAX_ROWS || (result.len() + 1) * columns > MAX_CELLS {
        return None;
    }
    Some(result)
}

pub fn plan(
    source: &str,
    current: Snapshot,
    target: &Target,
    action: Action,
) -> Option<TextChange> {
    if current != target.snapshot || target.row == 0 {
        return None;
    }
    let rows = lines(source, &target.table, target.columns)?;
    if rows.len() + 1 != target.rows || rows.get(target.row - 1) != Some(&target.line) {
        return None;
    }
    match action {
        Action::Duplicate if can_duplicate(target.rows, target.columns) => {
            let replacement = source.get(target.line.clone())?.to_owned();
            Some(TextChange {
                range: target.line.end..target.line.end,
                caret: 0,
                replacement,
            })
        }
        Action::Delete => Some(TextChange {
            range: target.line.clone(),
            replacement: String::new(),
            caret: 0,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(source: &str, row: usize) -> Target {
        let table = 0..source.len();
        let lines = lines(source, &table, 2).unwrap();
        Target {
            snapshot: Snapshot::default(),
            table,
            row,
            rows: lines.len() + 1,
            columns: 2,
            line: lines[row - 1].clone(),
        }
    }
    fn apply(source: &str, target: &Target, action: Action) -> String {
        let change = plan(source, Snapshot::default(), target, action).unwrap();
        assert_eq!(change.caret, 0);
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        result
    }
    #[test]
    fn duplicate_preserves_raw_unicode_escaped_pipe_indent_tabs_and_ragged_row() {
        let source = "|a|b|\n  | --- | --: |\n  | café🙂 **raw** | a\\|b &amp; |\n| short |\n";
        let target = target(source, 1);
        assert_eq!(apply(source, &target, Action::Duplicate), "|a|b|\n  | --- | --: |\n  | café🙂 **raw** | a\\|b &amp; |\n  | café🙂 **raw** | a\\|b &amp; |\n| short |\n");
    }
    #[test]
    fn delete_only_selected_complete_line_and_protects_header_delimiter() {
        let source = "|a|b|\n|---|---|\n|one|two|\n|last|\n";
        assert_eq!(
            apply(source, &target(source, 1), Action::Delete),
            "|a|b|\n|---|---|\n|last|\n"
        );
        assert_eq!(
            apply(source, &target(source, 2), Action::Delete),
            "|a|b|\n|---|---|\n|one|two|\n"
        );
        let mut bad = target(source, 1);
        bad.row = 0;
        assert!(plan(source, Snapshot::default(), &bad, Action::Delete).is_none());
    }
    #[test]
    fn last_body_row_deletion_leaves_a_header_only_table() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        assert_eq!(
            apply(source, &target(source, 1), Action::Delete),
            "|a|b|\n|-|-|\n"
        );
        assert!(lines("|a|b|\n|-|-|\n", &(0..12), 2).unwrap().is_empty());
    }
    #[test]
    fn non_lf_non_pipe_multiline_malformed_and_bad_ranges_are_rejected() {
        for source in [
            "a|b\n-|-\nx|y\n",
            "|a|b|\n|-|-|\n|x|y|",
            "|a|b|\n|-|-|\n    |x|y|\n",
            "|a|b|\n|-|-|\n\t|x|y|\n",
            "|a|b|\n|-|-|\n\n",
        ] {
            assert!(lines(source, &(0..source.len()), 2).is_none(), "{source:?}");
        }
        for range in [Range { start: 3, end: 1 }, 0..usize::MAX, 1..2] {
            assert!(lines("é", &range, 2).is_none());
        }
    }
    #[test]
    fn exact_snapshot_index_count_column_and_line_required() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for case in 0..9 {
            let mut bad = target(source, 1);
            match case {
                0 => bad.snapshot.document_id += 1,
                1 => bad.snapshot.revision += 1,
                2 => bad.snapshot.state_id += 1,
                3 => bad.row = usize::MAX,
                4 => bad.rows += 1,
                5 => bad.columns = 1,
                6 => bad.line.start += 1,
                7 => bad.line.end -= 1,
                _ => bad.table.end -= 1,
            }
            assert!(plan(source, Snapshot::default(), &bad, Action::Delete).is_none());
        }
    }
    #[test]
    fn duplicate_caps_are_checked_before_allocating_a_prefix() {
        assert!(can_duplicate(127, 2));
        assert!(!can_duplicate(128, 2));
        assert!(can_duplicate(41, 12));
        assert!(!can_duplicate(42, 12));
        assert!(!can_duplicate(usize::MAX, usize::MAX));
        assert!(!can_duplicate(0, 1));
        let source = format!("|a|b|\n|-|-|\n{}", "|x|y|\n".repeat(127));
        let target = target(&source, 127);
        assert!(plan(&source, Snapshot::default(), &target, Action::Duplicate).is_none());
        assert!(plan(&source, Snapshot::default(), &target, Action::Delete).is_some());
    }
    #[test]
    fn edits_preserve_prefix_suffix_and_caret_points_to_new_or_following_line_start() {
        let table = "|a|b|\n|-|-|\n|x|y|\n";
        let source = format!("before\n\n{table}\nafter\n\t");
        let mut t = target(table, 1);
        t.table = 8..8 + table.len();
        t.line = (t.line.start + 8)..(t.line.end + 8);
        let change = plan(&source, Snapshot::default(), &t, Action::Duplicate).unwrap();
        assert_eq!(change.range.start + change.caret, t.line.end);
        let mut result = source;
        result.replace_range(change.range, &change.replacement);
        assert_eq!(result, format!("before\n\n{table}|x|y|\n\nafter\n\t"));
    }
    #[test]
    fn lexical_lines_cannot_authenticate_a_fenced_table() {
        let source = "```\n|a|b|\n|-|-|\n|x|y|\n```\n";
        assert!(lines(source, &(4..source.len() - 4), 2).is_some());
    }
}
