//! Conservative multiline raw paragraph edits; scanners never authenticate roots.
use crate::document::Snapshot;
pub use crate::preview_paragraph::Target;
use crate::text_edit::TextChange;
use std::ops::Range;
pub const MAX_BYTES: usize = 16 * 1024;
pub const MAX_LINES: usize = 128;
pub fn valid_value(value: &str) -> bool {
    if value.len() > MAX_BYTES || value.chars().any(|c| c.is_control() && c != '\n') {
        return false;
    }
    let mut count = 0;
    for line in value.split('\n') {
        count += 1;
        if count > MAX_LINES || line.is_empty() || line.starts_with(char::is_whitespace) {
            return false;
        }
    }
    count >= 2
}
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
    if payload(source, snapshot, target)? == value || !valid_value(value) {
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
    fn raw_soft_and_hard_breaks_unicode_markup_spaces_and_outside_bytes_stay_exact() {
        let source = "before\n\ncafé🙂 **raw**  \na\\|b &amp;\n\nafter\n\t";
        let start = source.find("café").unwrap();
        let end = source.find("\n\nafter").unwrap() + 1;
        let t = Target {
            snapshot: Snapshot::default(),
            block: start..end,
            payload: start..end - 1,
        };
        assert_eq!(
            payload(source, t.snapshot, &t),
            Some("café🙂 **raw**  \na\\|b &amp;")
        );
        let change = plan(source, t.snapshot, &t, "new **é🙂**\nsecond  \nthird").unwrap();
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        assert_eq!(
            result,
            "before\n\nnew **é🙂**\nsecond  \nthird\n\nafter\n\t"
        );
        assert_eq!(change.caret, "new **é🙂**\nsecond  \nthird".len());
    }
    #[test]
    fn byte_and_line_caps_are_bounded_without_overflow() {
        assert!(valid_value(&format!(
            "{}\nx",
            "é".repeat((MAX_BYTES - 2) / 2)
        )));
        assert!(valid_value(&vec!["x"; MAX_LINES].join("\n")));
        assert!(!valid_value(&vec!["x"; MAX_LINES + 1].join("\n")));
        assert!(!valid_value(&format!("{}\nx", "x".repeat(MAX_BYTES))));
    }
    #[test]
    fn blank_indented_control_single_line_and_eof_payloads_are_rejected() {
        for value in [
            "",
            "single",
            "one\n",
            "\none",
            "one\n\ntwo",
            "one\n two",
            "one\n\ttwo",
            "one\ntwo\r",
            "one\ntab\t",
            "one\n\0",
            "one\n\u{2003}two",
        ] {
            assert!(!valid_value(value), "{value:?}");
        }
        for source in ["one\ntwo", "one\n\ntwo\n", "one\n two\n", "one\ntwo\r\n"] {
            assert!(payload_range(source, &(0..source.len())).is_none());
        }
    }
    #[test]
    fn forged_snapshot_range_utf8_and_overflow_are_refused() {
        let source = "é🙂\nsecond\n";
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
                5 => bad.payload.start = 1,
                6 => bad.payload.end -= 1,
                7 => bad.block = usize::MAX..usize::MAX,
                _ => bad.payload = 0..0,
            };
            assert!(plan(source, t.snapshot, &bad, "new\ntext").is_none());
        }
    }
    #[test]
    fn unchanged_noop_and_lexical_fence_or_structure_positive_need_root_parser_proof() {
        let source = "first\nsecond\n";
        let t = Target {
            snapshot: Snapshot::default(),
            block: 0..source.len(),
            payload: 0..source.len() - 1,
        };
        assert!(plan(source, t.snapshot, &t, "first\nsecond").is_none());
        for value in [
            "first\n# heading",
            "first\n- list",
            "first\n---",
            "first\n```",
            "first\n<div>",
        ] {
            assert!(plan(source, t.snapshot, &t, value).is_some());
        }
        let fenced = format!("```\n{source}```\n");
        let inner = Target {
            snapshot: t.snapshot,
            block: 4..4 + source.len(),
            payload: 4..3 + source.len(),
        };
        assert!(plan(&fenced, t.snapshot, &inner, "new\ntext").is_some());
    }
}
