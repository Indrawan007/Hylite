//! Conservative root-table body-row edits: whole LF-terminated pipe-leading lines.
//! Insertions add only a bounded blank canonical row; existing bytes stay raw.
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
    InsertBefore,
    InsertAfter,
    MoveUp,
    MoveDown,
}
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Duplicate => "Duplicate",
            Self::Delete => "Delete",
            Self::InsertBefore => "Insert blank before",
            Self::InsertAfter => "Insert blank after",
            Self::MoveUp => "Move up",
            Self::MoveDown => "Move down",
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

/// Body index only; full snapshot/root/line authentication is still required.
pub fn neighbor(target: &Target, action: Action) -> Option<usize> {
    if !(2..=MAX_ROWS).contains(&target.rows)
        || !(1..=MAX_COLUMNS).contains(&target.columns)
        || target.rows * target.columns > MAX_CELLS
        || target.row == 0
        || target.row >= target.rows
    {
        return None;
    }
    match action {
        Action::MoveUp if target.row > 1 => Some(target.row - 1),
        Action::MoveDown if target.row + 1 < target.rows => Some(target.row + 1),
        _ => None,
    }
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
        Action::InsertBefore | Action::InsertAfter
            if can_duplicate(target.rows, target.columns) =>
        {
            let line = source.get(target.line.clone())?;
            let indent = line.bytes().take_while(|b| *b == b' ').count();
            let at = if action == Action::InsertBefore {
                target.line.start
            } else {
                target.line.end
            };
            Some(TextChange {
                range: at..at,
                replacement: format!("{}|{}\n", " ".repeat(indent), "  |".repeat(target.columns)),
                // Inside the first blank cell, not the selected old row.
                caret: indent + 2,
            })
        }
        Action::MoveUp | Action::MoveDown => {
            let adjacent = rows.get(neighbor(target, action)? - 1)?;
            let selected = source.get(target.line.clone())?;
            let other = source.get(adjacent.clone())?;
            if selected == other {
                return None; // Identical raw rows do not create a hidden edge.
            }
            let (range, replacement, caret) = if action == Action::MoveUp {
                (
                    adjacent.start..target.line.end,
                    format!("{selected}{other}"),
                    0,
                )
            } else {
                (
                    target.line.start..adjacent.end,
                    format!("{other}{selected}"),
                    other.len(),
                )
            };
            Some(TextChange {
                range,
                replacement,
                caret,
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
    #[test]
    fn insert_before_after_preserves_every_old_byte_and_places_caret_in_first_blank_cell() {
        let table = "|a|b|\n|---|--:|\n   | café🙂 **raw** | a\\|b &amp; |\n| short |\n";
        let source = format!("before\n\n{table}\nafter\n\t");
        let mut t = target(table, 1);
        t.table = 8..8 + table.len();
        t.line = t.line.start + 8..t.line.end + 8;
        for (action, at) in [
            (Action::InsertBefore, t.line.start),
            (Action::InsertAfter, t.line.end),
        ] {
            let change = plan(&source, Snapshot::default(), &t, action).unwrap();
            assert_eq!(change.range, at..at);
            assert_eq!(change.replacement, "   |  |  |\n");
            assert_eq!(change.caret, 5);
            let mut expected = source.clone();
            expected.insert_str(at, "   |  |  |\n");
            let mut actual = source.clone();
            actual.replace_range(change.range, &change.replacement);
            assert_eq!(actual, expected);
            assert_eq!(&actual[..at], &source[..at]);
            assert_eq!(&actual[at + change.replacement.len()..], &source[at..]);
        }
    }
    #[test]
    fn inserted_blank_row_uses_header_column_count_not_ragged_body_width() {
        for columns in 1..=MAX_COLUMNS {
            let source = format!(
                "|{}\n|{}\n| ragged |\n",
                "h|".repeat(columns),
                "---|".repeat(columns)
            );
            let table = 0..source.len();
            let line = lines(&source, &table, columns).unwrap()[0].clone();
            let t = Target {
                snapshot: Snapshot::default(),
                table,
                row: 1,
                rows: 2,
                columns,
                line,
            };
            for action in [Action::InsertBefore, Action::InsertAfter] {
                let change = plan(&source, Snapshot::default(), &t, action).unwrap();
                assert_eq!(change.replacement, format!("|{}\n", "  |".repeat(columns)));
                assert_eq!(
                    change.replacement.bytes().filter(|b| *b == b'|').count(),
                    columns + 1
                );
                assert_eq!(change.caret, 2);
            }
        }
    }
    #[test]
    fn insert_refuses_stale_forged_header_and_bad_dimensions_without_allocating_a_row() {
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for action in [Action::InsertBefore, Action::InsertAfter] {
            for case in 0..10 {
                let mut t = target(source, 1);
                match case {
                    0 => t.snapshot.document_id += 1,
                    1 => t.snapshot.revision += 1,
                    2 => t.snapshot.state_id += 1,
                    3 => t.row = 0,
                    4 => t.row = usize::MAX,
                    5 => t.rows = usize::MAX,
                    6 => t.columns = usize::MAX,
                    7 => t.line.start += 1,
                    8 => t.line.end -= 1,
                    _ => t.table.end -= 1,
                }
                assert!(plan(source, Snapshot::default(), &t, action).is_none());
            }
        }
    }
    #[test]
    fn insert_growth_caps_and_noncanonical_eof_are_all_or_nothing() {
        for columns in [2, 12] {
            let count = if columns == 2 { 127 } else { 41 };
            let source = format!(
                "|{}\n|{}\n{}",
                "h|".repeat(columns),
                "---|".repeat(columns),
                format!("|{}\n", "x|".repeat(columns)).repeat(count)
            );
            let table = 0..source.len();
            let t = Target {
                snapshot: Snapshot::default(),
                line: lines(&source, &table, columns).unwrap()[0].clone(),
                table,
                row: 1,
                rows: count + 1,
                columns,
            };
            for action in [Action::InsertBefore, Action::InsertAfter] {
                assert!(plan(&source, Snapshot::default(), &t, action).is_none());
            }
        }
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        let mut t = target(source, 1);
        t.table.end -= 1;
        t.line.end -= 1;
        for action in [Action::InsertBefore, Action::InsertAfter] {
            assert!(plan(
                source.trim_end_matches('\n'),
                Snapshot::default(),
                &t,
                action
            )
            .is_none());
        }
    }
    #[test]
    fn moving_up_down_swaps_whole_raw_lines_without_touching_header_or_neighbors() {
        let source = "|a|b|\n  | --- | --: |\n  | café🙂 **raw** | a\\|b &amp; |\n| short |\n";
        let expected = "|a|b|\n  | --- | --: |\n| short |\n  | café🙂 **raw** | a\\|b &amp; |\n";
        for (row, action) in [(1, Action::MoveDown), (2, Action::MoveUp)] {
            let t = target(source, row);
            let change = plan(source, Snapshot::default(), &t, action).unwrap();
            assert_eq!(apply_move(source, &change), expected);
            assert_eq!(change.range, 22..source.len());
            assert_eq!(change.caret, if row == 1 { "| short |\n".len() } else { 0 });
        }
    }
    fn apply_move(source: &str, change: &TextChange) -> String {
        let mut result = source.to_owned();
        result.replace_range(change.range.clone(), &change.replacement);
        result
    }
    #[test]
    fn move_neighbors_are_body_only_bounded_and_reject_invalid_shapes() {
        let source = "|a|b|\n|-|-|\n|x|y|\n|z|w|\n";
        let mut t = target(source, 1);
        assert_eq!(neighbor(&t, Action::MoveUp), None);
        assert_eq!(neighbor(&t, Action::MoveDown), Some(2));
        t.row = 2;
        assert_eq!(neighbor(&t, Action::MoveUp), Some(1));
        assert_eq!(neighbor(&t, Action::MoveDown), None);
        for action in [
            Action::Duplicate,
            Action::Delete,
            Action::InsertBefore,
            Action::InsertAfter,
        ] {
            assert_eq!(neighbor(&t, action), None);
        }
        for (row, rows, columns) in [
            (0, 3, 2),
            (3, 3, 2),
            (usize::MAX, 3, 2),
            (1, 0, 2),
            (1, 129, 2),
            (1, 3, 0),
            (1, 3, 13),
            (1, 128, 5),
            (1, usize::MAX, usize::MAX),
        ] {
            t.row = row;
            t.rows = rows;
            t.columns = columns;
            for action in [Action::MoveUp, Action::MoveDown] {
                assert!(neighbor(&t, action).is_none());
            }
        }
        let source = "|a|b|\n|-|-|\n|x|y|\n";
        for action in [Action::MoveUp, Action::MoveDown] {
            assert!(plan(source, Snapshot::default(), &target(source, 1), action).is_none());
        }
    }
    #[test]
    fn move_caret_follows_selected_line_after_variable_utf8_length_preserving_outside_bytes() {
        let raw = "|a|b|\n|-|-|\n| first | x |\n   | é🙂 | z |\t\n";
        let source = format!("before\n\n{raw}\nafter\n\t");
        let mut t = target(raw, 1);
        t.table = 8..8 + raw.len();
        t.line = t.line.start + 8..t.line.end + 8;
        let change = plan(&source, Snapshot::default(), &t, Action::MoveDown).unwrap();
        assert_eq!(
            change.range.start + change.caret,
            8 + 12 + "   | é🙂 | z |\t\n".len()
        );
        assert_eq!(
            apply_move(&source, &change),
            "before\n\n|a|b|\n|-|-|\n   | é🙂 | z |\t\n| first | x |\n\nafter\n\t"
        );
    }
    #[test]
    fn moving_requires_exact_snapshot_count_index_line_and_table_range() {
        let source = "|a|b|\n|-|-|\n|x|y|\n|z|w|\n";
        for action in [Action::MoveUp, Action::MoveDown] {
            for case in 0..10 {
                let mut t = target(source, if action == Action::MoveUp { 2 } else { 1 });
                match case {
                    0 => t.snapshot.document_id += 1,
                    1 => t.snapshot.revision += 1,
                    2 => t.snapshot.state_id += 1,
                    3 => t.row = 0,
                    4 => t.row = usize::MAX,
                    5 => t.rows += 1,
                    6 => t.columns = 1,
                    7 => t.line.start += 1,
                    8 => t.line.end -= 1,
                    _ => t.table.end -= 1,
                }
                assert!(plan(source, Snapshot::default(), &t, action).is_none());
            }
        }
    }
    #[test]
    fn identical_raw_rows_are_noops_but_render_equivalent_raw_rows_still_move() {
        let source = "|a|b|\n|-|-|\n|x|y|\n|x|y|\n";
        for (row, action) in [(1, Action::MoveDown), (2, Action::MoveUp)] {
            assert!(plan(source, Snapshot::default(), &target(source, row), action).is_none());
        }
        let source = "|a|b|\n|-|-|\n|x|y|\n |x|y|\n";
        assert!(plan(
            source,
            Snapshot::default(),
            &target(source, 1),
            Action::MoveDown
        )
        .is_some());
    }
    #[test]
    fn moves_do_not_need_growth_room_even_at_row_and_cell_caps() {
        for (rows, columns) in [(128, 2), (128, 4)] {
            let first = format!("|{}\n", "a|".repeat(columns));
            let other = format!("|{}\n", "b|".repeat(columns));
            let header = format!("|{}\n|{}\n", "h|".repeat(columns), "---|".repeat(columns));
            let source = format!("{header}{first}{}", other.repeat(rows - 2));
            let table = 0..source.len();
            let line = lines(&source, &table, columns).unwrap()[0].clone();
            let t = Target {
                snapshot: Snapshot::default(),
                table,
                row: 1,
                rows,
                columns,
                line,
            };
            assert!(!can_duplicate(rows, columns));
            let change = plan(&source, Snapshot::default(), &t, Action::MoveDown).unwrap();
            assert!(apply_move(&source, &change).starts_with(&format!("{header}{other}{first}")));
        }
    }
    #[test]
    fn move_rejects_noncanonical_eof_and_lexical_fenced_success_is_not_provenance() {
        let raw = "|a|b|\n|-|-|\n|x|y|\n|z|w|\n";
        let mut t = target(raw, 1);
        t.table.end -= 1;
        assert!(plan(raw, Snapshot::default(), &t, Action::MoveDown).is_none());
        let source = format!("```\n{raw}```\n");
        let mut t = target(raw, 1);
        t.table = 4..4 + raw.len();
        t.line = t.line.start + 4..t.line.end + 4;
        assert!(plan(&source, Snapshot::default(), &t, Action::MoveDown).is_some());
    }
}
