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
#[derive(Clone, Debug, Default)]
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
    /// Deterministic LCG so failures are reproducible from the printed seed.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 33
        }
        fn pick(&mut self, bound: u64) -> u64 {
            self.next() % bound
        }
        fn text(&mut self, seed: u64) -> String {
            let size = match self.pick(10) {
                0..=5 => (self.pick(40) as usize) + 1,
                6..=8 => (self.pick(2_000) as usize) + 1,
                _ => (self.pick(MAX_BYTES as u64) as usize) + 1,
            };
            let mut text = String::with_capacity(size);
            while text.len() < size {
                if self.pick(9) == 0 {
                    text.push('\n');
                } else {
                    text.push(char::from(b'a' + (seed % 26) as u8));
                }
            }
            text
        }
    }

    /// Independent reference for the documented policy. It recomputes the byte
    /// total from scratch instead of maintaining a running counter, so it
    /// catches accounting drift rather than repeating the production bookkeeping.
    #[derive(Default)]
    struct Model {
        undo: Vec<Record>,
        redo: Vec<Record>,
    }
    impl Model {
        fn bytes(&self) -> usize {
            self.undo
                .iter()
                .chain(&self.redo)
                .map(|r| r.text.len())
                .sum()
        }
        fn record(&mut self, before: Record, after: &Record) -> bool {
            if !before.valid() || !after.valid() || before.text == after.text {
                return false;
            }
            self.redo.clear();
            self.undo.push(before);
            self.trim();
            true
        }
        fn step(&mut self, current: Record, redo: bool) -> Option<Record> {
            if !current.valid() {
                return None;
            }
            let (from, to) = if redo {
                (&mut self.redo, &mut self.undo)
            } else {
                (&mut self.undo, &mut self.redo)
            };
            let target = from.pop()?;
            to.push(current);
            self.trim();
            Some(target)
        }
        fn trim(&mut self) {
            while self.undo.len() + self.redo.len() > MAX_RECORDS || self.bytes() > MAX_TEXT_BYTES {
                if self.undo.is_empty() && self.redo.is_empty() {
                    return;
                }
                if self.undo.is_empty() {
                    self.redo.remove(0);
                } else {
                    self.undo.remove(0);
                }
            }
        }
    }

    #[test]
    fn generated_history_matches_an_independently_bookkept_reference() {
        for seed in [1_u64, 7, 42, 99, 2026] {
            let mut rng = Rng(seed);
            let mut h = History::default();
            let mut model = Model::default();
            let mut current = r(&format!("seed {seed}\ntext"));
            for step in 0..400 {
                match rng.pick(10) {
                    0..=4 => {
                        let next = r(&rng.text(step));
                        assert_eq!(
                            h.record(current.clone(), &next),
                            model.record(current.clone(), &next),
                            "record diverged at seed {seed} step {step}"
                        );
                        if h.undo.back() == Some(&current) {
                            current = next;
                        }
                    }
                    5..=7 => {
                        let expected = model.step(current.clone(), false);
                        assert_eq!(
                            h.step(current.clone(), false),
                            expected,
                            "undo diverged at seed {seed} step {step}"
                        );
                        if let Some(target) = expected {
                            current = target;
                        }
                    }
                    _ => {
                        let expected = model.step(current.clone(), true);
                        assert_eq!(
                            h.step(current.clone(), true),
                            expected,
                            "redo diverged at seed {seed} step {step}"
                        );
                        if let Some(target) = expected {
                            current = target;
                        }
                    }
                }
                assert_eq!(h.can_undo(), !model.undo.is_empty());
                assert_eq!(h.can_redo(), !model.redo.is_empty());
                assert_eq!(h.bytes, model.bytes(), "byte drift at seed {seed}");
                assert_eq!(h.undo.len(), model.undo.len());
                assert_eq!(h.redo.len(), model.redo.len());
                check(&h);
            }
            assert!(h.undo.len() + h.redo.len() <= MAX_RECORDS);
        }
    }

    #[test]
    fn eviction_always_takes_the_oldest_undo_then_the_farthest_redo() {
        let mut rng = Rng(11);
        let mut h = History::default();
        let mut model = Model::default();
        let mut current = r("start\ntext");
        for n in 0..MAX_RECORDS {
            let next = r(&format!("full {n}\ntext"));
            assert!(h.record(current.clone(), &next));
            assert!(model.record(current.clone(), &next));
            current = next;
        }
        assert_eq!(h.undo.len(), MAX_RECORDS);
        // Build a redo stack, then force eviction while both stacks are full.
        for _ in 0..40 {
            let target = h.step(current.clone(), false).unwrap();
            assert_eq!(Some(target.clone()), model.step(current.clone(), false));
            current = target;
        }
        assert_eq!(h.redo.len(), 40);
        for n in 0..60 {
            let next = r(&rng.text(n as u64));
            assert_eq!(
                h.record(current.clone(), &next),
                model.record(current.clone(), &next)
            );
            current = next;
            check(&h);
        }
        // Entries are ordered oldest-first, so the retained windows must match
        // the reference exactly, not only in size.
        assert_eq!(
            h.undo.iter().map(|r| r.text.clone()).collect::<Vec<_>>(),
            model
                .undo
                .iter()
                .map(|r| r.text.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            h.redo.iter().map(|r| r.text.clone()).collect::<Vec<_>>(),
            model
                .redo
                .iter()
                .map(|r| r.text.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(h.bytes, model.bytes());
    }

    #[test]
    fn refused_records_and_steps_leave_both_stacks_byte_identical() {
        let mut rng = Rng(5);
        let mut h = History::default();
        let mut current = r("base\ntext");
        for _ in 0..20 {
            let next = r(&rng.text(3));
            h.record(current.clone(), &next);
            current = next;
        }
        h.step(current.clone(), false);
        let snapshot = (
            h.undo.iter().cloned().collect::<Vec<_>>(),
            h.redo.iter().cloned().collect::<Vec<_>>(),
            h.bytes,
        );
        let oversized = "x".repeat(MAX_BYTES + 1);
        let invalid_caret = Record {
            text: "é\n🙂".into(),
            caret: 1,
        };
        let out_of_bounds = Record {
            text: "short".into(),
            caret: usize::MAX,
        };
        for bad in [
            Record {
                text: oversized,
                caret: 5,
            },
            invalid_caret,
            out_of_bounds,
        ] {
            assert!(!h.record(bad.clone(), &r("fine\ntext")));
            assert!(!h.record(r("fine\ntext"), &bad));
            assert_eq!(h.step(bad.clone(), false), None);
            assert_eq!(h.step(bad, true), None);
            assert_eq!(
                (
                    h.undo.iter().cloned().collect::<Vec<_>>(),
                    h.redo.iter().cloned().collect::<Vec<_>>(),
                    h.bytes,
                ),
                snapshot
            );
        }
        check(&h);
    }
}
