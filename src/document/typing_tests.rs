use super::*;

struct Buffer {
    text: String,
    caret: usize,
    state: u64,
}
impl Buffer {
    fn new() -> Self {
        Self {
            text: String::new(),
            caret: 0,
            state: 0,
        }
    }
    fn record(&self) -> Record {
        Record {
            text: self.text.clone(),
            caret: self.caret,
            state_id: self.state,
        }
    }
    fn insert(&mut self, history: &mut History, ch: char, at: Instant) {
        let before = self.record();
        self.text.insert(self.caret, ch);
        self.caret += ch.len_utf8();
        self.state += 1;
        history.record_edit(
            before,
            &self.text,
            self.caret,
            self.state,
            EditKind::Insert(ch),
            at,
        );
    }
    fn restore(&mut self, record: Record) {
        self.text = record.text;
        self.caret = record.caret;
        self.state = record.state_id;
    }
}

#[test]
fn adjacent_unicode_text_is_one_step_but_each_edit_keeps_its_state_id() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let start = Instant::now();
    for (i, ch) in "é世界👩\u{200d}💻e\u{301}".chars().enumerate() {
        b.insert(&mut h, ch, start + Duration::from_millis(i as u64 * 10));
    }
    assert_eq!(b.state, 8);
    let text = b.text.clone();
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "");
    assert_eq!(b.state, 0);
    assert!(h.peek_undo().is_none());
    b.restore(h.redo(b.record()).unwrap());
    assert_eq!(b.text, text);
    assert_eq!(b.state, 8);
}

#[test]
fn text_whitespace_and_ascii_punctuation_are_separate_steps() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    for ch in "hello  world!".chars() {
        b.insert(&mut h, ch, at);
    }
    for expected in ["hello  world", "hello  ", "hello", ""] {
        b.restore(h.undo(b.record()).unwrap());
        assert_eq!(b.text, expected);
    }
}

#[test]
fn underscores_are_text_but_control_and_newline_insertions_are_atomic() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    for ch in "a_b\nc".chars() {
        b.insert(&mut h, ch, at);
    }
    for expected in ["a_b\n", "a_b", ""] {
        b.restore(h.undo(b.record()).unwrap());
        assert_eq!(b.text, expected);
    }
}

#[test]
fn idle_at_the_exact_limit_starts_a_new_step() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    b.insert(&mut h, 'a', at);
    b.insert(&mut h, 'b', at + IDLE_LIMIT - Duration::from_millis(1));
    b.insert(&mut h, 'c', at + IDLE_LIMIT * 2 - Duration::from_millis(1));
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "ab");
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "");
}

#[test]
fn continuous_typing_still_splits_at_the_total_burst_limit() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    for n in 0..=10 {
        b.insert(&mut h, 'a', at + Duration::from_millis(n * 500));
    }
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text.len(), 10);
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "");
}

#[test]
fn scalar_limit_is_not_a_utf8_byte_limit() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    for _ in 0..=MAX_SCALARS {
        b.insert(&mut h, '✨', at);
    }
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text.chars().count(), usize::from(MAX_SCALARS));
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "");
}

#[test]
fn explicit_boundaries_preserve_save_checkpoints_between_bursts() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    for ch in "ab".chars() {
        b.insert(&mut h, ch, at);
    }
    let saved = b.state;
    h.break_group(); // save begins BEFORE capturing/sending its snapshot
    for ch in "cd".chars() {
        b.insert(&mut h, ch, at);
    }
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "ab");
    assert_eq!(b.state, saved);
    b.restore(h.redo(b.record()).unwrap());
    assert_eq!(b.text, "abcd");
}

#[test]
fn state_or_caret_discontinuities_never_merge() {
    let at = Instant::now();
    for change_state in [false, true] {
        let mut h = History::default();
        let mut b = Buffer::new();
        b.insert(&mut h, 'a', at);
        if change_state {
            b.state = 99;
        } else {
            b.caret = 0;
        }
        let before = b.record();
        b.insert(&mut h, 'b', at);
        assert_eq!(h.undo(b.record()), Some(before));
    }
}

#[test]
fn time_going_backwards_is_a_boundary_not_a_panic() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    b.insert(&mut h, 'a', at + Duration::from_secs(1));
    b.insert(&mut h, 'b', at);
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "a");
}

#[test]
fn wrong_insert_labels_or_nonlocal_deltas_fall_back_to_atomic_history() {
    let at = Instant::now();
    for (after, caret, ch) in [
        ("aYZ", 3, 'Z'),
        ("Xa", 1, 'X'),
        ("ab", 0, 'b'),
        ("a✨", 4, 'b'),
    ] {
        let mut h = History::default();
        let mut b = Buffer::new();
        b.insert(&mut h, 'a', at);
        let before = b.record();
        h.record_edit(before.clone(), after, caret, 2, EditKind::Insert(ch), at);
        assert_eq!(h.peek_undo(), Some(&before));
        assert!(h.group.is_none());
    }
}

#[test]
fn invalid_caret_metadata_cannot_panic_or_coalesce() {
    let before = Record {
        text: "é".into(),
        caret: 1,
        state_id: 0,
    };
    assert!(insertion(&before, "éa", 3, 'a').is_none());
    let before = Record {
        caret: usize::MAX,
        ..before
    };
    assert!(insertion(&before, "éa", 3, 'a').is_none());
}

#[test]
fn paste_and_explicit_atomic_commands_remain_boundaries() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    b.insert(&mut h, 'a', at);
    let before = b.record();
    b.text.push_str("paste");
    b.caret = b.text.len();
    b.state += 1;
    h.record_edit(before, &b.text, b.caret, b.state, EditKind::Atomic, at);
    b.insert(&mut h, 'b', at);
    for expected in ["apaste", "a", ""] {
        b.restore(h.undo(b.record()).unwrap());
        assert_eq!(b.text, expected);
    }
}

#[test]
fn undo_redo_and_branching_end_the_previous_typing_group() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    for ch in "ab".chars() {
        b.insert(&mut h, ch, at);
    }
    b.restore(h.undo(b.record()).unwrap());
    b.restore(h.redo(b.record()).unwrap());
    b.insert(&mut h, 'c', at);
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "ab");
    b.state = 100; // document allocates a fresh ID on the new branch
    b.insert(&mut h, 'x', at);
    assert!(h.peek_redo().is_none());
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "ab");
}

#[test]
fn no_ops_preserve_redo_and_do_not_record_an_entry() {
    let mut h = History::default();
    let mut b = Buffer::new();
    let at = Instant::now();
    b.insert(&mut h, 'a', at);
    b.restore(h.undo(b.record()).unwrap());
    h.record_edit(b.record(), &b.text, b.caret, 99, EditKind::Atomic, at);
    assert!(h.peek_undo().is_none());
    assert!(h.peek_redo().is_some());
}

#[test]
fn budgets_cover_coalesced_groups_and_oversized_post_edit_states() {
    let mut h = History::with_limits(2, 2);
    let mut b = Buffer::new();
    let at = Instant::now();
    for ch in "abc".chars() {
        b.insert(&mut h, ch, at);
    }
    assert!(h.limited());
    assert!(h.group.is_none());
    assert!(h.undo.len() + h.redo.len() <= 2);
    assert!(
        h.undo
            .iter()
            .chain(&h.redo)
            .map(|record| record.text.len())
            .sum::<usize>()
            <= 2
    );
    b.restore(h.undo(b.record()).unwrap());
    assert_eq!(b.text, "ab");
    assert!(h.peek_redo().is_none()); // oversized current text cannot be retained
    let mut h = History::with_limits(0, 10);
    b.insert(&mut h, 'x', at);
    assert!(h.group.is_none());
    assert!(h.peek_undo().is_none());
}
