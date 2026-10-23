//! Bounded, draft-only raw text snapshots. No Document identity or Source history.
use crate::preview_paragraph_multiline::MAX_BYTES;
use std::collections::VecDeque;

pub const MAX_RECORDS: usize = 100;
/// Retained snapshot text only, not a total process/allocation memory claim.
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub text: String,
    /// Logical UTF-8 byte offset; restoration collapses selection at this caret.
    pub caret: usize,
}
impl Record {
    fn valid(&self) -> bool {
        self.text.len() <= MAX_BYTES && self.text.is_char_boundary(self.caret)
    }
}
#[derive(Default)]
pub struct History {
    undo: VecDeque<Record>,
    redo: VecDeque<Record>,
    bytes: usize,
}
impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn target(&self, redo: bool) -> Option<&Record> {
        if redo { &self.redo } else { &self.undo }.back()
    }
    /// One effective native edit = one step, no timing/coalescing heuristics.
    pub fn record(&mut self, before: Record, after: &Record) -> bool {
        if !before.valid() || !after.valid() || before.text == after.text {
            return false;
        }
        self.bytes -= self.redo.iter().map(|r| r.text.len()).sum::<usize>();
        self.redo.clear();
        self.bytes += before.text.len();
        self.undo.push_back(before);
        self.trim();
        true
    }
    /// Caller preflights native reconstruction before consuming the target.
    pub fn step(&mut self, current: Record, redo: bool) -> Option<Record> {
        if !current.valid() {
            return None;
        }
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        let target = from.pop_back()?;
        self.bytes -= target.text.len();
        self.bytes += current.text.len();
        to.push_back(current);
        self.trim();
        Some(target)
    }
    fn trim(&mut self) {
        while self.undo.len() + self.redo.len() > MAX_RECORDS || self.bytes > MAX_TEXT_BYTES {
            // Oldest undo first, then farthest redo; retain immediate next targets.
            let Some(oldest) = self.undo.pop_front().or_else(|| self.redo.pop_front()) else {
                break;
            };
            self.bytes -= oldest.text.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(text: &str) -> Record {
        Record {
            text: text.into(),
            caret: text.len(),
        }
    }
    fn check(h: &History) {
        assert!(h.undo.len() + h.redo.len() <= MAX_RECORDS);
        assert!(h.bytes <= MAX_TEXT_BYTES);
        assert_eq!(
            h.bytes,
            h.undo.iter().chain(&h.redo).map(|r| r.text.len()).sum()
        );
    }
    #[test]
    fn empty_history_is_inert() {
        let mut h = History::default();
        assert!(!h.can_undo() && !h.can_redo());
        assert_eq!(h.target(false), None);
        assert_eq!(h.step(r("first\nsecond"), false), None);
        assert_eq!(h.step(r("first\nsecond"), true), None);
        check(&h);
    }
    #[test]
    fn raw_unicode_hard_breaks_invalid_final_lf_and_controls_round_trip() {
        let values = ["café🙂  \nsecond", "\n", "one\n\ntwo\t", "one\ntwo\n", ""];
        let mut h = History::default();
        for pair in values.windows(2) {
            assert!(h.record(r(pair[0]), &r(pair[1])));
        }
        let mut current = r(values.last().unwrap());
        for expected in values[..values.len() - 1].iter().rev() {
            current = h.step(current, false).unwrap();
            assert_eq!(current, r(expected));
            check(&h);
        }
        for expected in &values[1..] {
            current = h.step(current, true).unwrap();
            assert_eq!(current, r(expected));
            check(&h);
        }
    }
    #[test]
    fn noop_and_caret_only_actions_preserve_redo() {
        let mut h = History::default();
        h.record(r("one\ntwo"), &r("new\ntext"));
        let mut current = h.step(r("new\ntext"), false).unwrap();
        current.caret = 0;
        assert!(!h.record(r("one\ntwo"), &current));
        assert!(h.can_redo());
        assert_eq!(h.step(current, true), Some(r("new\ntext")));
        check(&h);
    }
    #[test]
    fn effective_branch_discards_only_draft_redo() {
        let mut h = History::default();
        h.record(r("a\nb"), &r("c\nd"));
        let old = h.step(r("c\nd"), false).unwrap();
        assert!(h.record(old, &r("branch\ntext")));
        assert!(!h.can_redo());
        assert_eq!(h.step(r("branch\ntext"), false), Some(r("a\nb")));
        check(&h);
    }
    #[test]
    fn invalid_utf8_carets_and_oversized_records_are_refused_without_consuming_history() {
        let mut h = History::default();
        h.record(r("é\nx"), &r("new\ntext"));
        for bad in [
            Record {
                text: "é\nx".into(),
                caret: 1,
            },
            Record {
                text: "é".into(),
                caret: usize::MAX,
            },
            r(&"x".repeat(MAX_BYTES + 1)),
        ] {
            assert!(!h.record(bad.clone(), &r("other\ntext")));
            assert!(!h.record(r("fine\ntext"), &bad));
            assert_eq!(h.step(bad, false), None);
            assert_eq!(h.target(false), Some(&r("é\nx")));
            check(&h);
        }
    }
    #[test]
    fn entry_cap_keeps_the_most_recent_hundred_edits() {
        let mut h = History::default();
        for n in 0..150 {
            h.record(r(&format!("{n}\ntext")), &r(&format!("{}\ntext", n + 1)));
            check(&h);
        }
        let mut current = r("150\ntext");
        for n in (50..150).rev() {
            current = h.step(current, false).unwrap();
            assert_eq!(current, r(&format!("{n}\ntext")));
            check(&h);
        }
        assert!(!h.can_undo());
    }
    #[test]
    fn byte_budget_keeps_recent_full_sized_records() {
        let mut h = History::default();
        let a = r(&"a".repeat(MAX_BYTES));
        let b = r(&"b".repeat(MAX_BYTES));
        for _ in 0..150 {
            h.record(a.clone(), &b);
            check(&h);
        }
        assert_eq!(h.undo.len(), MAX_TEXT_BYTES / MAX_BYTES);
        assert_eq!(h.bytes, MAX_TEXT_BYTES);
    }
    #[test]
    fn undo_and_redo_transfers_rebudget_different_sized_current_text() {
        let mut h = History::default();
        let small = r("x");
        let large = r(&"y".repeat(MAX_BYTES));
        for _ in 0..100 {
            h.record(small.clone(), &large);
        }
        for _ in 0..100 {
            if h.step(large.clone(), false).is_none() {
                break;
            }
            check(&h);
        }
        assert!(h.can_redo());
        for _ in 0..100 {
            if h.step(small.clone(), true).is_none() {
                break;
            }
            check(&h);
        }
    }
    #[test]
    fn caret_at_start_middle_and_trailing_empty_line_are_preserved() {
        for caret in [0, 2, 3, 7, 8] {
            let before = Record {
                text: "é\nnext\n".into(),
                caret,
            };
            let mut h = History::default();
            assert!(h.record(before.clone(), &r("other\ntext")));
            assert_eq!(h.step(r("other\ntext"), false), Some(before));
        }
    }
    #[test]
    fn independent_sessions_do_not_share_text_or_redo() {
        let mut h = History::default();
        h.record(r("a\nb"), &r("c\nd"));
        let mut other = History::default();
        assert_eq!(other.step(r("a\nb"), false), None);
        h.step(r("c\nd"), false);
        assert!(!other.can_redo());
        check(&other);
        check(&h);
    }
    #[test]
    fn target_peeking_is_non_consuming_for_failed_reconstruction() {
        let mut h = History::default();
        h.record(r("a\nb"), &r("c\nd"));
        for _ in 0..3 {
            assert_eq!(h.target(false), Some(&r("a\nb")));
        }
        assert!(!h.can_redo());
        check(&h);
    }
    #[test]
    fn generated_small_workflow_matches_simple_snapshot_stacks() {
        let mut h = History::default();
        let mut current = r("start\ntext");
        let (mut undo, mut redo) = (Vec::new(), Vec::new());
        for n in 0..1000 {
            match n % 7 {
                0..=2 => {
                    let next = r(&format!("é🙂 {n}\nraw  "));
                    h.record(current.clone(), &next);
                    undo.push(current);
                    if undo.len() > MAX_RECORDS {
                        undo.remove(0);
                    }
                    redo.clear();
                    current = next;
                }
                3 | 4 => {
                    if let Some(target) = undo.pop() {
                        redo.push(current.clone());
                        assert_eq!(h.step(current, false), Some(target.clone()));
                        current = target;
                    }
                }
                _ => {
                    if let Some(target) = redo.pop() {
                        undo.push(current.clone());
                        assert_eq!(h.step(current, true), Some(target.clone()));
                        current = target;
                    }
                }
            }
            assert_eq!(h.can_undo(), !undo.is_empty());
            assert_eq!(h.can_redo(), !redo.is_empty());
            check(&h);
        }
    }
}
