//! Deterministic model-based tests of production Source core modules.
//! This rig is NOT Document or an Iced/controller test. It exercises History,
//! transactions, search and navigation together without a GUI or filesystem.
use super::{EditKind, History, Record};
use crate::document::Snapshot;
use crate::{continuation, line_edit, navigation, search::Search, text_edit};
use std::ops::Range;
use std::time::{Duration, Instant};

struct Rng(u64);
impl Rng {
    fn pick(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 32) as usize) % bound
    }
}

// Reference frames use scalar vectors/cursors, not production UTF-8 slicing.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Frame {
    chars: Vec<char>,
    caret: usize,
    state: u64,
}
impl Frame {
    fn record(&self) -> Record {
        Record {
            text: self.chars.iter().collect(),
            caret: self.chars[..self.caret]
                .iter()
                .map(|ch| ch.len_utf8())
                .sum(),
            state_id: self.state,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Gesture {
    mode: u8, // 0 insert, 1 Backspace, 2 Delete; never merge directions
    space: bool,
}
fn gesture(mode: u8, ch: char) -> Option<Gesture> {
    if ch.is_control() || (ch.is_ascii_punctuation() && ch != '_') {
        None
    } else {
        Some(Gesture {
            mode,
            space: ch.is_whitespace(),
        })
    }
}
struct Burst {
    gesture: Gesture,
    started: u64,
    last: u64,
    count: usize,
}
struct Reference {
    current: Frame,
    undo: Vec<Frame>,
    redo: Vec<Frame>,
    burst: Option<Burst>,
    next_state: u64,
}
impl Reference {
    fn new(text: &str) -> Self {
        Self {
            current: Frame {
                chars: text.chars().collect(),
                caret: 0,
                state: 0,
            },
            undo: Vec::new(),
            redo: Vec::new(),
            burst: None,
            next_state: 0,
        }
    }
    fn edit(
        &mut self,
        range: Range<usize>,
        replacement: &str,
        caret: usize,
        kind: Option<Gesture>,
        ms: u64,
    ) {
        let mut chars = self.current.chars.clone();
        chars.splice(range, replacement.chars());
        if chars == self.current.chars {
            self.burst = None;
            return;
        }
        // Independent behavioral oracle: known scalar operations, numeric test
        // clock and Vec stacks; no production delta verifier or Group metadata.
        let merge = self.burst.as_ref().is_some_and(|burst| {
            kind == Some(burst.gesture)
                && ms >= burst.last
                && ms - burst.last < 750
                && ms >= burst.started
                && ms - burst.started < 5000
                && burst.count < 64
        });
        if !merge {
            self.undo.push(self.current.clone());
        }
        self.redo.clear();
        self.next_state += 1;
        self.current = Frame {
            chars,
            caret,
            state: self.next_state,
        };
        self.burst = kind.map(|kind| Burst {
            gesture: kind,
            started: if merge {
                self.burst.as_ref().unwrap().started
            } else {
                ms
            },
            last: ms,
            count: if merge {
                self.burst.as_ref().unwrap().count + 1
            } else {
                1
            },
        });
    }
    fn restore(&mut self, redo: bool) -> Option<Record> {
        self.burst = None;
        let (from, to) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        let target = from.pop()?;
        to.push(std::mem::replace(&mut self.current, target));
        Some(self.current.record())
    }
}

struct Rig {
    current: Record,
    history: History,
    next_state: u64,
    revision: u64,
    clock: Instant,
    search: Search,
}
impl Rig {
    fn new(text: &str) -> Self {
        Self {
            current: Record {
                text: text.into(),
                caret: 0,
                state_id: 0,
            },
            // Deliberately untrimmed for the reference-model test. Budget
            // behavior is exercised separately against tight production limits.
            history: History::with_limits(10000, 64 * 1024 * 1024),
            next_state: 0,
            revision: 0,
            clock: Instant::now(),
            search: Search::default(),
        }
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            state_id: self.current.state_id,
            ..Snapshot::default()
        }
    }
    fn edit(
        &mut self,
        range: Range<usize>,
        replacement: &str,
        caret: usize,
        kind: EditKind,
        ms: u64,
    ) {
        let before = self.current.clone();
        self.current.text.replace_range(range, replacement);
        if self.current.text != before.text {
            self.next_state += 1;
            self.revision += 1;
            self.current.caret = caret;
            self.current.state_id = self.next_state;
        }
        self.history.record_edit(
            before,
            &self.current.text,
            self.current.caret,
            self.current.state_id,
            kind,
            self.clock + Duration::from_millis(ms),
        );
    }
    fn restore(&mut self, redo: bool) -> Option<Record> {
        let target = if redo {
            self.history.redo(self.current.clone())
        } else {
            self.history.undo(self.current.clone())
        };
        if let Some(target) = &target {
            self.current = target.clone();
            self.revision += 1;
        }
        target
    }
    fn barrier(&mut self, reference: &mut Reference) {
        self.history.break_group();
        reference.burst = None;
    }
    fn check(&mut self, reference: &Reference, seed: u64, step: usize) {
        let context = format!("seed={seed}, step={step}");
        assert_eq!(self.current, reference.current.record(), "{context}");
        assert_eq!(self.history.undo.len(), reference.undo.len(), "{context}");
        assert_eq!(self.history.redo.len(), reference.redo.len(), "{context}");
        assert_eq!(
            self.history.peek_undo(),
            reference.undo.last().map(Frame::record).as_ref(),
            "{context}"
        );
        assert_eq!(
            self.history.peek_redo(),
            reference.redo.last().map(Frame::record).as_ref(),
            "{context}"
        );
        assert_eq!(
            self.history.group.is_some(),
            reference.burst.is_some(),
            "{context}"
        );
        assert!(
            self.current.text.is_char_boundary(self.current.caret),
            "{context}"
        );
        assert!(
            !self.history.limited(),
            "untrimmed model exceeded its test budget: {context}"
        );
        self.search.refresh(&self.current.text, self.snapshot());
        let query: Vec<_> = self.search.query.chars().collect();
        let chars = &reference.current.chars;
        let mut expected = 0;
        let mut at = 0;
        while !query.is_empty() && at + query.len() <= chars.len() {
            if chars[at..at + query.len()] == query {
                expected += 1;
                at += query.len();
            } else {
                at += 1;
            }
        }
        assert_eq!(self.search.count(), expected, "{context}");
        if let Some(range) = self.search.current() {
            assert!(
                text_edit::valid_range(&self.current.text, &range),
                "{context}"
            );
            assert_eq!(self.current.text[range], self.search.query, "{context}");
        }
    }
}

fn byte_at(chars: &[char], scalar: usize) -> usize {
    chars[..scalar].iter().map(|ch| ch.len_utf8()).sum()
}
fn commit(
    rig: &mut Rig,
    reference: &mut Reference,
    range: Range<usize>,
    replacement: &str,
    caret: usize,
    label: (EditKind, Option<Gesture>),
    ms: u64,
) {
    let (kind, candidate) = label;
    let start = byte_at(&reference.current.chars, range.start);
    let end = byte_at(&reference.current.chars, range.end);
    let mut after = reference.current.chars.clone();
    after.splice(range.clone(), replacement.chars());
    let byte_caret = byte_at(&after, caret);
    rig.edit(start..end, replacement, byte_caret, kind, ms);
    reference.edit(range, replacement, caret, candidate, ms);
}
fn transaction(rig: &mut Rig, reference: &mut Reference, change: text_edit::TextChange, ms: u64) {
    assert!(text_edit::valid_range(&rig.current.text, &change.range));
    assert!(change.replacement.is_char_boundary(change.caret));
    let start = rig.current.text[..change.range.start].chars().count();
    let end = rig.current.text[..change.range.end].chars().count();
    let caret = start + change.replacement[..change.caret].chars().count();
    commit(
        rig,
        reference,
        start..end,
        &change.replacement,
        caret,
        (EditKind::Atomic, None),
        ms,
    );
}

#[test]
fn generated_source_workflows_match_a_scalar_vector_history_model() {
    const ALPHABET: &[char] = &[
        'a', 'b', '_', ' ', '\t', '\n', '!', 'é', '世', '✨', '\u{301}', '\u{200d}', 'א', '\u{a0}',
    ];
    let mut coverage = [0usize; 16];
    for seed in 0..64 {
        let mut rng = Rng(seed + 1);
        let mut rig = Rig::new("- é✨\na  b\n");
        let mut reference = Reference::new(&rig.current.text);
        let mut ms: u64 = 10000;
        for step in 0..512 {
            // Includes clock reversal and exact idle/burst boundaries, with no
            // sleep, random crate, environment seed or external input.
            ms = if rng.pick(16) == 0 {
                ms.saturating_sub(800)
            } else {
                ms + [0, 1, 100, 749, 750, 5000][rng.pick(6)]
            };
            let choice = if reference.current.chars.len() > 128 {
                4
            } else {
                rng.pick(16)
            };
            coverage[choice] += 1;
            let len = reference.current.chars.len();
            let caret = reference.current.caret;
            match choice {
                0..=1 => {
                    let ch = ALPHABET[rng.pick(ALPHABET.len())];
                    commit(
                        &mut rig,
                        &mut reference,
                        caret..caret,
                        &ch.to_string(),
                        caret + 1,
                        (EditKind::Insert(ch), gesture(0, ch)),
                        ms,
                    );
                }
                2..=3 => {
                    let backwards = choice == 2;
                    let range = if backwards {
                        caret.saturating_sub(1)..caret
                    } else {
                        caret..(caret + 1).min(len)
                    };
                    let candidate = reference
                        .current
                        .chars
                        .get(range.clone())
                        .and_then(|slice| slice.first())
                        .and_then(|ch| gesture(if backwards { 1 } else { 2 }, *ch));
                    let next = range.start;
                    commit(
                        &mut rig,
                        &mut reference,
                        range,
                        "",
                        next,
                        (
                            if backwards {
                                EditKind::Backspace
                            } else {
                                EditKind::Delete
                            },
                            candidate,
                        ),
                        ms,
                    );
                }
                4 => {
                    rig.barrier(&mut reference);
                    let first = if len > 128 { 0 } else { rng.pick(len + 1) };
                    let last = if len > 128 {
                        len
                    } else {
                        first + rng.pick(len - first + 1)
                    };
                    let text = ["", "é✨", "a\na", " "][rng.pick(4)];
                    commit(
                        &mut rig,
                        &mut reference,
                        first..last,
                        text,
                        first + text.chars().count(),
                        (EditKind::Atomic, None),
                        ms,
                    );
                }
                5 => {
                    assert_eq!(
                        rig.restore(false),
                        reference.restore(false),
                        "seed={seed}, step={step}"
                    );
                }
                6 => {
                    assert_eq!(
                        rig.restore(true),
                        reference.restore(true),
                        "seed={seed}, step={step}"
                    );
                }
                7 => {
                    rig.barrier(&mut reference);
                    let at = rng.pick(len + 1);
                    // Resolve scalar coordinates back through production Go to Line.
                    let prefix: String = reference.current.chars[..at].iter().collect();
                    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
                    let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
                    let target =
                        navigation::resolve(&rig.current.text, &format!("{line}:{column}"))
                            .unwrap();
                    rig.current.caret = target.offset;
                    reference.current.caret = at;
                    rig.search.clear_current();
                }
                8 => {
                    // Save capture is a barrier; the explicit checkpoint
                    // round-trip regression below checks its retained record.
                    rig.barrier(&mut reference);
                }
                9 => {
                    rig.barrier(&mut reference);
                    rig.search.query = ["", "a", "é", "a\na", " "][rng.pick(5)].into();
                    rig.search.refresh(&rig.current.text, rig.snapshot());
                    if let Some(range) = rig.search.navigate(rng.pick(2) == 0) {
                        rig.current.caret = range.start;
                        reference.current.caret = rig.current.text[..range.start].chars().count();
                    }
                }
                10..=11 => {
                    rig.barrier(&mut reference);
                    let start = rng.pick(len + 1);
                    let end = start + rng.pick(len - start + 1);
                    let range = byte_at(&reference.current.chars, start)
                        ..byte_at(&reference.current.chars, end);
                    let action = [
                        line_edit::Action::Duplicate,
                        line_edit::Action::Delete,
                        line_edit::Action::MoveUp,
                        line_edit::Action::MoveDown,
                    ][rng.pick(4)];
                    if let Some(change) = line_edit::edit(&rig.current.text, range, action) {
                        transaction(&mut rig, &mut reference, change, ms);
                    }
                }
                12 => {
                    rig.barrier(&mut reference);
                    if let Some(change) = continuation::enter(&rig.current.text, rig.current.caret)
                    {
                        transaction(&mut rig, &mut reference, change, ms);
                    }
                }
                13 => {
                    rig.barrier(&mut reference);
                    rig.search.refresh(&rig.current.text, rig.snapshot());
                    rig.search.replacement = ["", "é", "a a"][rng.pick(3)].into();
                    if let Some(change) = rig.search.replacement_all(&rig.current.text) {
                        transaction(&mut rig, &mut reference, change, ms);
                    }
                }
                14 => {
                    rig.barrier(&mut reference);
                    let at = byte_at(&reference.current.chars, caret);
                    if let Some(change) =
                        text_edit::indent(&rig.current.text, at..at, 2, rng.pick(2) == 0)
                    {
                        transaction(&mut rig, &mut reference, change, ms);
                    }
                }
                _ => {
                    // Read-only tick: check() below refreshes the search cache,
                    // but no user gesture should seal the history group.
                }
            }
            rig.check(&reference, seed, step);
        }
        // Compare every retained checkpoint, not just final text or a stack size.
        while !reference.undo.is_empty() {
            assert_eq!(rig.restore(false), reference.restore(false), "seed={seed}");
        }
        assert!(rig.restore(false).is_none());
        while !reference.redo.is_empty() {
            assert_eq!(rig.restore(true), reference.restore(true), "seed={seed}");
        }
        assert!(rig.restore(true).is_none());
        rig.check(&reference, seed, 512);
    }
    assert!(coverage.iter().all(|count| *count > 0), "{coverage:?}");
}

#[test]
fn generated_tight_budget_histories_keep_valid_immediate_checkpoints() {
    for max_entries in [0, 1, 2, 7] {
        for max_bytes in [0, 1, 8, 64] {
            let mut rng = Rng(1 + max_entries as u64 * 100 + max_bytes as u64);
            let mut rig = Rig::new("é✨\na");
            rig.history = History::with_limits(max_entries, max_bytes);
            for step in 0..512 {
                match rng.pick(6) {
                    0 => {
                        let expected = rig.history.peek_undo().cloned();
                        assert_eq!(rig.restore(false), expected);
                    }
                    1 => {
                        let expected = rig.history.peek_redo().cloned();
                        assert_eq!(rig.restore(true), expected);
                    }
                    2 => rig.history.break_group(),
                    _ => {
                        let before = rig.current.text.clone();
                        if before.len() > 128 || rng.pick(2) == 0 {
                            let end = before.chars().next().map_or(0, char::len_utf8);
                            rig.edit(0..end, "", 0, EditKind::Delete, step);
                        } else {
                            let ch = ['a', 'é', '✨', ' ', '\n'][rng.pick(5)];
                            rig.edit(
                                0..0,
                                &ch.to_string(),
                                ch.len_utf8(),
                                EditKind::Insert(ch),
                                step,
                            );
                        }
                    }
                }
                assert!(rig.history.undo.len() + rig.history.redo.len() <= max_entries);
                assert!(
                    rig.history
                        .undo
                        .iter()
                        .chain(&rig.history.redo)
                        .map(|record| record.text.len())
                        .sum::<usize>()
                        <= max_bytes
                );
                assert!(rig.current.text.is_char_boundary(rig.current.caret));
                for record in rig.history.undo.iter().chain(&rig.history.redo) {
                    assert!(record.text.is_char_boundary(record.caret));
                }
                if let Some(group) = rig.history.group {
                    assert_eq!(rig.history.peek_undo().unwrap().state_id, group.base_state);
                    assert_eq!(rig.current.state_id, group.last_state);
                    assert_eq!(rig.current.caret, group.caret);
                }
            }
        }
    }
}

#[test]
fn composed_transactions_can_undo_to_a_captured_save_boundary_then_redo() {
    let mut rig = Rig::new("a\nb");
    let mut reference = Reference::new("a\nb");
    for ch in "é✨".chars() {
        let at = reference.current.caret;
        commit(
            &mut rig,
            &mut reference,
            at..at,
            &ch.to_string(),
            at + 1,
            (EditKind::Insert(ch), gesture(0, ch)),
            0,
        );
    }
    rig.barrier(&mut reference); // capture before asynchronous save, not at ack
    let saved = rig.current.clone();
    let revision = rig.revision;
    let change = line_edit::edit(&rig.current.text, 0..0, line_edit::Action::Duplicate).unwrap();
    transaction(&mut rig, &mut reference, change, 1);
    rig.search.query = "a".into();
    rig.search.replacement = "世界".into();
    rig.search.refresh(&rig.current.text, rig.snapshot());
    let change = rig.search.replacement_all(&rig.current.text).unwrap();
    transaction(&mut rig, &mut reference, change, 2);
    for _ in 0..2 {
        assert_eq!(rig.restore(false), reference.restore(false));
    }
    assert_eq!(rig.current, saved);
    assert!(rig.revision > revision); // rig clock only, NOT native Document proof
    assert_eq!(rig.restore(false), reference.restore(false));
    assert_eq!(rig.current.text, "a\nb");
    assert_eq!(rig.restore(true), reference.restore(true));
    assert_eq!(rig.current, saved);
    rig.check(&reference, 0, 0);
}

#[test]
fn malformed_candidate_deltas_never_join_a_valid_existing_group() {
    let at = Instant::now();
    for mode in 0..3 {
        for seed in 0..64 {
            let mut history = History::default();
            let prefix = "é".repeat(seed);
            let original = Record {
                text: format!("{prefix}abcé✨-{seed}"),
                caret: prefix.len() + 2,
                state_id: 0,
            };
            let mut after = original.text.clone();
            let old_caret = original.caret;
            let (caret, kind) = match mode {
                0 => {
                    after.insert(old_caret, 'X');
                    (old_caret + 1, EditKind::Insert('X'))
                }
                1 => {
                    after.remove(old_caret - 1);
                    (old_caret - 1, EditKind::Backspace)
                }
                _ => {
                    after.remove(old_caret);
                    (old_caret, EditKind::Delete)
                }
            };
            history.record_edit(original, &after, caret, 1, kind, at);
            assert!(history.group.is_some());
            // Ineligible labels/deltas: multi-scalar removal, normalization,
            // wrong prefix/suffix/caret, underflow or split UTF-8 metadata.
            let before = Record {
                text: after.clone(),
                caret,
                state_id: 1,
            };
            let (bad_after, bad_caret) = [
                ("", 0),
                ("X", 1),
                ("changed prefix", usize::MAX),
                ("é✨", 1),
                (after.as_str(), caret),
            ][seed % 5];
            history.record_edit(before.clone(), bad_after, bad_caret, 2, kind, at);
            assert!(history.group.is_none());
            if bad_after == after {
                assert_eq!(history.undo.len(), 1);
            } else {
                assert_eq!(history.peek_undo(), Some(&before));
            }
        }
    }
}
