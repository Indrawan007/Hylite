//! Bounded single-line raw paragraph drafts. Lexical checks are NOT root provenance.
use crate::document::Snapshot;
use crate::text_edit::TextChange;
use std::ops::Range;

pub const MAX_BYTES: usize = 16 * 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub block: Range<usize>,
    pub payload: Range<usize>,
}

pub fn valid_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BYTES
        && !value.starts_with(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

/// Conservative physical-line eligibility only. Document must authenticate root membership.
pub fn payload_range(source: &str, block: &Range<usize>) -> Option<Range<usize>> {
    if block.start != 0 && source.as_bytes().get(block.start.checked_sub(1)?) != Some(&b'\n') {
        return None;
    }
    let raw = source.get(block.clone())?.strip_suffix('\n')?;
    valid_value(raw).then_some(block.start..block.end.checked_sub(1)?)
}

pub fn payload<'a>(source: &'a str, snapshot: Snapshot, target: &Target) -> Option<&'a str> {
    if snapshot != target.snapshot || payload_range(source, &target.block)? != target.payload {
        return None;
    }
    source.get(target.payload.clone())
}

pub fn plan(source: &str, snapshot: Snapshot, target: &Target, value: &str) -> Option<TextChange> {
    let before = payload(source, snapshot, target)?;
    if !valid_value(value) || value == before {
        return None;
    }
    Some(TextChange {
        range: target.payload.clone(),
        replacement: value.to_owned(),
        caret: value.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_raw_line_preserves_inline_markdown_unicode_trailing_spaces_and_lf() {
        let source = "before\n\ncafé🙂 **raw** &amp; a\\|b  \n\nafter\n\t";
        let start = source.find("café").unwrap();
        let end = source.find("\n\nafter").unwrap() + 1;
        let t = Target {
            snapshot: Snapshot::default(),
            block: start..end,
            payload: start..end - 1,
        };
        assert_eq!(
            payload(source, t.snapshot, &t),
            Some("café🙂 **raw** &amp; a\\|b  ")
        );
        let change = plan(source, t.snapshot, &t, "new **é🙂** &amp; a\\|b").unwrap();
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        assert_eq!(result, "before\n\nnew **é🙂** &amp; a\\|b\n\nafter\n\t");
        assert_eq!(change.caret, "new **é🙂** &amp; a\\|b".len());
    }
    #[test]
    fn strict_single_line_budget_whitespace_and_control_bounds() {
        assert!(valid_value(&"x".repeat(MAX_BYTES)));
        for value in [
            "".to_owned(),
            " leading".into(),
            "\tleading".into(),
            "\u{2003}leading".into(),
            "two\nlines".into(),
            "cr\r".into(),
            "tab\t".into(),
            "nul\0".into(),
            "x".repeat(MAX_BYTES + 1),
        ] {
            assert!(!valid_value(&value));
        }
        for source in ["text", " text\n", "one\ntwo\n", "text\r\n", "\n"] {
            assert!(payload_range(source, &(0..source.len())).is_none());
        }
    }
    #[test]
    fn cold_lexical_ranges_cannot_authenticate_heading_or_fence() {
        let source = "```\nplain\n```\n";
        let t = Target {
            snapshot: Snapshot::default(),
            block: 4..10,
            payload: 4..9,
        };
        assert!(plan(source, t.snapshot, &t, "changed").is_some());
        assert!(payload_range("# heading\n", &(0..10)).is_some()); // parser proof mandatory
    }
    #[test]
    fn forged_snapshot_ranges_utf8_and_overflow_fail_without_panics() {
        let source = "é🙂 raw\n";
        let t = Target {
            snapshot: Snapshot::default(),
            block: 0..source.len(),
            payload: 0..source.len() - 1,
        };
        for case in 0..9 {
            let mut bad = t.clone();
            match case {
                0 => bad.snapshot.document_id += 1,
                1 => bad.snapshot.revision += 1,
                2 => bad.snapshot.state_id += 1,
                3 => bad.block.end -= 1,
                4 => bad.block.start = 1,
                5 => bad.payload.end -= 1,
                6 => bad.payload.start = 1,
                7 => bad.block = usize::MAX..usize::MAX,
                _ => bad.payload = usize::MAX..usize::MAX,
            }
            assert!(plan(source, t.snapshot, &bad, "new").is_none());
        }
    }
    #[test]
    fn unchanged_is_noop_but_structure_changing_values_need_parser_preflight() {
        let source = "plain\n";
        let t = Target {
            snapshot: Snapshot::default(),
            block: 0..6,
            payload: 0..5,
        };
        assert!(plan(source, t.snapshot, &t, "plain").is_none());
        for value in ["# heading", "- item", "> quote", "---", "```", "<div>"] {
            assert!(plan(source, t.snapshot, &t, value).is_some());
        }
    }
}
