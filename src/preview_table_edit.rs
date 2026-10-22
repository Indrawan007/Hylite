//! Root GFM delimiter edits. Preserve raw cells/pipes/whitespace/dash counts.
use crate::document::Snapshot;
use crate::text_edit::{valid_range, TextChange};
use std::ops::Range;

pub const MAX_COLUMNS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Default,
    Left,
    Center,
    Right,
}
impl Alignment {
    pub const ALL: [Self; 4] = [Self::Default, Self::Left, Self::Center, Self::Right];
}
impl std::fmt::Display for Alignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Default => "Default",
            Self::Left => "Left",
            Self::Center => "Center",
            Self::Right => "Right",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delimiter {
    pub source: Range<usize>,
    pub alignment: Alignment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub table: Range<usize>,
    pub column: usize,
    pub columns: usize,
    pub marker: Range<usize>,
    pub alignment: Alignment,
}

/// Lexical only. Document must authenticate exact root table/model membership.
pub fn delimiters(source: &str, table: &Range<usize>, columns: usize) -> Option<Vec<Delimiter>> {
    if !valid_range(source, table) || !(1..=MAX_COLUMNS).contains(&columns) {
        return None;
    }
    let header_start = source[..table.start].rfind('\n').map_or(0, |at| at + 1);
    let prefix = source.get(header_start..table.start)?;
    if prefix.len() > 3 || !prefix.bytes().all(|b| b == b' ') {
        return None;
    }
    let line_start = table.start + source.get(table.clone())?.find('\n')? + 1;
    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |at| line_start + at);
    if line_end > table.end {
        return None;
    }
    let raw = source.get(line_start..line_end)?;
    let indent = raw.bytes().take_while(|b| *b == b' ').count();
    if indent > 3 || raw.as_bytes().get(indent) == Some(&b'\t') {
        return None;
    }
    let mut start = line_start + indent;
    let mut end = line_end;
    while end > start && matches!(source.as_bytes()[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    if source.as_bytes().get(start) == Some(&b'|') {
        start += 1;
    }
    if end > start && source.as_bytes()[end - 1] == b'|' {
        end -= 1;
    }
    let mut result = Vec::new();
    let mut at = start;
    for cell in source.get(start..end)?.split('|') {
        if result.len() == columns {
            return None;
        }
        let left = cell
            .bytes()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count();
        let token = cell.trim_matches([' ', '\t']);
        let left_colon = token.starts_with(':');
        let right_colon = token.ends_with(':');
        let dashes = token.trim_matches(':');
        if dashes.is_empty()
            || !dashes.bytes().all(|b| b == b'-')
            || token.len() != dashes.len() + usize::from(left_colon) + usize::from(right_colon)
        {
            return None;
        }
        let alignment = match (left_colon, right_colon) {
            (false, false) => Alignment::Default,
            (true, false) => Alignment::Left,
            (true, true) => Alignment::Center,
            (false, true) => Alignment::Right,
        };
        result.push(Delimiter {
            source: at + left..at + left + token.len(),
            alignment,
        });
        at += cell.len() + 1;
    }
    (result.len() == columns).then_some(result)
}

pub fn plan(
    source: &str,
    current: Snapshot,
    target: &Target,
    desired: Alignment,
) -> Option<TextChange> {
    if current != target.snapshot || desired == target.alignment {
        return None;
    }
    let cells = delimiters(source, &target.table, target.columns)?;
    let cell = cells.get(target.column)?;
    if cell.source != target.marker || cell.alignment != target.alignment {
        return None;
    }
    let dashes = source.get(target.marker.clone())?.trim_matches(':');
    let replacement = match desired {
        Alignment::Default => dashes.to_owned(),
        Alignment::Left => format!(":{dashes}"),
        Alignment::Center => format!(":{dashes}:"),
        Alignment::Right => format!("{dashes}:"),
    };
    Some(TextChange {
        range: target.marker.clone(),
        caret: replacement.len(),
        replacement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn target(source: &str, column: usize, columns: usize) -> Target {
        let table = 0..source.len();
        let tokens = delimiters(source, &table, columns).unwrap();
        Target {
            snapshot: Snapshot::default(),
            marker: tokens[column].source.clone(),
            alignment: tokens[column].alignment,
            table,
            column,
            columns,
        }
    }
    #[test]
    fn all_alignment_pairs_preserve_raw_cells_whitespace_and_dash_count() {
        for (old, expected) in [
            ("---", Alignment::Default),
            (":---", Alignment::Left),
            (":---:", Alignment::Center),
            ("---:", Alignment::Right),
        ] {
            let source = format!(
                "| café🙂 **raw** | keep |\n  |\t{old}  | :--: |\n| a\\|b &amp; | untouched |\n\t"
            );
            let target = target(&source, 0, 2);
            assert_eq!(target.alignment, expected);
            for desired in Alignment::ALL {
                let change = plan(&source, Snapshot::default(), &target, desired);
                assert_eq!(change.is_some(), desired != expected);
                if let Some(change) = change {
                    let mut result = source.clone();
                    result.replace_range(change.range, &change.replacement);
                    let next = delimiters(&result, &(0..result.len()), 2).unwrap();
                    assert_eq!(next[0].alignment, desired);
                    assert_eq!(next[1].alignment, Alignment::Center);
                    assert!(result.starts_with("| café🙂 **raw** | keep |\n  |\t"));
                    assert!(result.ends_with("  | :--: |\n| a\\|b &amp; | untouched |\n\t"));
                    assert_eq!(change.replacement.bytes().filter(|b| *b == b'-').count(), 3);
                    assert_eq!(change.caret, change.replacement.len());
                }
            }
        }
    }
    #[test]
    fn optional_edge_pipes_indent_tabs_and_single_dash_tokens_are_preserved() {
        for line in [
            "|-|:--:|---:|",
            "- | :--: | ---:",
            "  - |\t:--:\t| ---:  ",
            "|-|:--:|---:",
        ] {
            let source = format!("a|b|c\n{line}");
            let cells = delimiters(&source, &(0..source.len()), 3).unwrap();
            assert_eq!(
                cells.iter().map(|x| x.alignment).collect::<Vec<_>>(),
                [Alignment::Default, Alignment::Center, Alignment::Right]
            );
        }
    }
    #[test]
    fn empty_malformed_colons_escaped_pipes_wrong_columns_and_indent_are_rejected() {
        for line in [
            "",
            "-||--",
            "- | : | --",
            "- | ::-- | --",
            "- | --:: | --",
            "- | --x | --",
            "- | -\\|-- | --",
            "    -|--|--",
            "\t-|--|--",
            "-|--",
        ] {
            let source = format!("a|b|c\n{line}\n");
            assert!(
                delimiters(&source, &(0..source.len()), 3).is_none(),
                "{line:?}"
            );
        }
        for columns in [0, MAX_COLUMNS + 1, usize::MAX] {
            assert!(delimiters("a\n-", &(0..3), columns).is_none());
        }
    }
    #[test]
    fn invalid_utf8_partial_reversed_overflow_and_absent_delimiter_lines_are_rejected() {
        let source = "é|b\n-|--\n";
        for range in [
            1..source.len(),
            0..usize::MAX,
            Range { start: 5, end: 2 },
            0..0,
            0..6,
        ] {
            assert!(delimiters(source, &range, 2).is_none());
        }
        assert!(delimiters("a|b", &(0..3), 2).is_none());
    }
    #[test]
    fn all_snapshot_fields_and_expected_marker_column_count_and_alignment_must_match() {
        let source = "a|b\n---|---\n";
        let good = target(source, 0, 2);
        for case in 0..7 {
            let mut bad = good.clone();
            match case {
                0 => bad.snapshot.document_id += 1,
                1 => bad.snapshot.revision += 1,
                2 => bad.snapshot.state_id += 1,
                3 => bad.marker.start += 1,
                4 => bad.column = 2,
                5 => bad.columns = 1,
                _ => bad.alignment = Alignment::Right,
            }
            assert!(plan(source, Snapshot::default(), &bad, Alignment::Center).is_none());
        }
    }
    #[test]
    fn no_op_is_rejected_instead_of_normalizing_the_delimiter() {
        let source = "a|b\n  | :-------:  | --- |\n";
        let target = target(source, 0, 2);
        assert!(plan(source, Snapshot::default(), &target, Alignment::Center).is_none());
    }
    #[test]
    fn any_column_is_edited_by_exact_byte_range_not_header_display_text() {
        let source = "same|same|same\n---|--|----\n1|2|3\n";
        let target = target(source, 1, 3);
        let change = plan(source, Snapshot::default(), &target, Alignment::Right).unwrap();
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        assert_eq!(result, "same|same|same\n---|--:|----\n1|2|3\n");
    }
    #[test]
    fn lexical_delimiters_are_not_proof_of_a_real_root_table() {
        assert!(delimiters("title\n---\n", &(0..10), 1).is_some()); // Setext, not a table.
        let source = "```md\na|b\n---|---\n```\n";
        assert!(delimiters(source, &(6..source.len() - 4), 2).is_some());
    }
}
