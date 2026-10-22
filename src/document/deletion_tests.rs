use super::*;

struct Buffer {
    text: String,
    caret: usize,
    state: u64,
    next_state: u64,
}
impl Buffer {
    fn new(text: &str, caret: usize) -> Self {
        Self {
            text: text.into(),
            caret,
            state: 0,
            next_state: 0,
        }
    }
    fn record(&self) -> Record {
        Record {
            text: self.text.clone(),
            caret: self.caret,
            state_id: self.state,
        }
    }
    fn delete(&mut self, h: &mut History, backwards: bool, at: Instant) {
        let before = self.record();
        let range = if backwards {
            self.text[..self.caret]
                .char_indices()
                .next_back()
                .map(|(start, _)| start..self.caret)
        } else {
            self.text[self.caret..]
                .chars()
                .next()
                .map(|ch| self.caret..self.caret + ch.len_utf8())
        };
        if let Some(range) = range {
            self.caret = range.start;
            self.text.replace_range(range, "");
            self.next_state += 1;
            self.state = self.next_state;
        }
        h.record_edit(
            before,
            &self.text,
            self.caret,
            self.state,
            if backwards {
                EditKind::Backspace
            } else {
                EditKind::Delete
            },
            at,
        );
    }
    fn undo(&mut self, h: &mut History) {
        let record = h.undo(self.record()).unwrap();
        self.text = record.text;
        self.caret = record.caret;
        self.state = record.state_id;
    }
    fn redo(&mut self, h: &mut History) {
        let record = h.redo(self.record()).unwrap();
        self.text = record.text;
        self.caret = record.caret;
        self.state = record.state_id;
    }
}

#[test]
fn backwards_unicode_deletions_restore_one_base_and_original_caret() {
    let at = Instant::now();
    let original = "aé世界";
    let mut b = Buffer::new(original, original.len());
    let mut h = History::default();
    for _ in 0..3 {
        b.delete(&mut h, true, at);
    }
    assert_eq!(b.text, "a");
    assert_eq!(b.state, 3);
    b.undo(&mut h);
    assert_eq!(b.text, original);
    assert_eq!(b.caret, original.len());
    assert_eq!(b.state, 0);
    assert!(h.peek_undo().is_none());
    b.redo(&mut h);
    assert_eq!(b.text, "a");
    assert_eq!(b.caret, 1);
    assert_eq!(b.state, 3);
}

#[test]
fn forward_deletions_keep_the_caret_and_untouched_prefix_suffix() {
    let at = Instant::now();
    let original = "head é世界 tail";
    let mut b = Buffer::new(original, "head ".len());
    let mut h = History::default();
    for _ in 0..3 {
        b.delete(&mut h, false, at);
    }
    assert_eq!(b.text, "head  tail");
    assert_eq!(b.caret, 5);
    b.undo(&mut h);
    assert_eq!(b.text, original);
    assert_eq!(b.caret, 5);
    assert!(h.peek_undo().is_none());
}

#[test]
fn whitespace_and_text_groups_are_separate_in_both_directions() {
    let at = Instant::now();
    for backwards in [false, true] {
        let mut b = Buffer::new("ab  cd", if backwards { 6 } else { 0 });
        let mut h = History::default();
        for _ in 0..6 {
            b.delete(&mut h, backwards, at);
        }
        for expected in if backwards {
            ["ab", "ab  ", "ab  cd"]
        } else {
            ["cd", "  cd", "ab  cd"]
        } {
            b.undo(&mut h);
            assert_eq!(b.text, expected);
        }
        assert!(h.peek_undo().is_none());
    }
}

#[test]
fn punctuation_newline_and_tabs_are_atomic_boundaries() {
    let at = Instant::now();
    for separator in ['!', '\n', '\t', '\r'] {
        let original = format!("ab{separator}cd");
        let mut b = Buffer::new(&original, original.len());
        let mut h = History::default();
        for _ in 0..5 {
            b.delete(&mut h, true, at);
        }
        for expected in ["ab".to_owned(), format!("ab{separator}"), original] {
            b.undo(&mut h);
            assert_eq!(b.text, expected);
        }
    }
    assert_eq!(classify('_'), Some(Class::Text));
    assert_eq!(classify('\u{a0}'), Some(Class::Whitespace));
}

#[test]
fn changing_direction_cannot_merge_even_with_contiguous_state_and_caret() {
    let mut b = Buffer::new("abcd", 2);
    let mut h = History::default();
    let at = Instant::now();
    b.delete(&mut h, false, at); // abd, caret 2
    b.delete(&mut h, true, at); // ad, caret 1
    b.undo(&mut h);
    assert_eq!(b.text, "abd");
    b.undo(&mut h);
    assert_eq!(b.text, "abcd");
}

#[test]
fn insertion_deletion_and_atomic_selection_replacement_never_merge() {
    let at = Instant::now();
    let mut h = History::default();
    let mut b = Buffer::new("abc", 3);
    b.delete(&mut h, true, at);
    let before = b.record();
    b.text.push('x');
    b.caret += 1;
    b.next_state += 1;
    b.state = b.next_state;
    h.record_edit(before, &b.text, b.caret, b.state, EditKind::Insert('x'), at);
    b.delete(&mut h, true, at);
    for expected in ["abx", "ab", "abc"] {
        b.undo(&mut h);
        assert_eq!(b.text, expected);
    }
    let mut h = History::default();
    let mut b = Buffer::new("abcd", 4);
    b.delete(&mut h, true, at);
    // A selected single scalar is still atomic even if its delta happens to
    // look identical to Backspace. Document must classify selection first.
    let before = b.record();
    b.text.pop();
    b.caret -= 1;
    b.state += 1;
    h.record_edit(before, &b.text, b.caret, b.state, EditKind::Atomic, at);
    assert!(h.group.is_none());
    b.undo(&mut h);
    assert_eq!(b.text, "abc");
}

#[test]
fn idle_limit_and_total_burst_duration_apply_to_both_delete_directions() {
    let at = Instant::now();
    for backwards in [false, true] {
        let mut b = Buffer::new("aaaa", if backwards { 4 } else { 0 });
        let mut h = History::default();
        b.delete(&mut h, backwards, at);
        b.delete(
            &mut h,
            backwards,
            at + IDLE_LIMIT - Duration::from_millis(1),
        );
        b.delete(
            &mut h,
            backwards,
            at + IDLE_LIMIT * 2 - Duration::from_millis(1),
        );
        b.undo(&mut h);
        assert_eq!(b.text.len(), 2);
        b.undo(&mut h);
        assert_eq!(b.text.len(), 4);

        let text = "a".repeat(11);
        let mut b = Buffer::new(&text, if backwards { 11 } else { 0 });
        let mut h = History::default();
        for n in 0..=10 {
            b.delete(&mut h, backwards, at + Duration::from_millis(n * 500));
        }
        b.undo(&mut h);
        assert_eq!(b.text.len(), 1);
        b.undo(&mut h);
        assert_eq!(b.text.len(), 11);
    }
}

#[test]
fn the_sixty_fifth_deleted_scalar_starts_a_new_step_not_a_byte_cap() {
    let text = "✨".repeat(65);
    let at = Instant::now();
    for backwards in [false, true] {
        let mut b = Buffer::new(&text, if backwards { text.len() } else { 0 });
        let mut h = History::default();
        for _ in 0..65 {
            b.delete(&mut h, backwards, at);
        }
        b.undo(&mut h);
        assert_eq!(b.text.chars().count(), 1);
        b.undo(&mut h);
        assert_eq!(b.text, text);
    }
}

#[test]
fn save_and_navigation_barriers_preserve_intermediate_checkpoints() {
    let at = Instant::now();
    for backwards in [false, true] {
        let mut b = Buffer::new("abcd", if backwards { 4 } else { 0 });
        let mut h = History::default();
        b.delete(&mut h, backwards, at);
        b.delete(&mut h, backwards, at);
        let saved = b.record();
        h.break_group(); // capture save / navigation / UI command
        b.delete(&mut h, backwards, at);
        b.delete(&mut h, backwards, at);
        b.undo(&mut h);
        assert_eq!(b.record(), saved);
        b.undo(&mut h);
        assert_eq!(b.text, "abcd");
    }
}

#[test]
fn grapheme_word_and_nonlocal_deltas_stay_atomic() {
    let at = Instant::now();
    for (before, old_caret, after, caret, backwards) in [
        ("e\u{301}", 3, "", 0, true),
        ("👩\u{200d}💻", 11, "", 0, true),
        ("abcd", 4, "ab", 2, true),
        ("abcd", 0, "cd", 0, false),
        ("abcd", 2, "acd", 2, false), // right length, wrong removal location
        ("abcd", 3, "abd", 1, true),  // right removal, wrong resulting caret
        ("abcd", 4, "Xbc", 3, true),  // also changed a prefix
        ("abcd", 0, "bcX", 0, false), // also changed a suffix
    ] {
        let before = Record {
            text: before.into(),
            caret: old_caret,
            state_id: 0,
        };
        assert!(deletion(&before, after, caret, backwards).is_none());
        let mut h = History::default();
        h.record_edit(
            before.clone(),
            after,
            caret,
            1,
            if backwards {
                EditKind::Backspace
            } else {
                EditKind::Delete
            },
            at,
        );
        assert!(h.group.is_none());
        assert_eq!(h.peek_undo(), Some(&before));
    }
}

#[test]
fn invalid_offsets_and_overflow_are_rejected_without_slicing_panics() {
    for backwards in [false, true] {
        for old in [0, 1, 3, usize::MAX] {
            for new in [0, 1, 2, usize::MAX] {
                let before = Record {
                    text: "éx".into(),
                    caret: old,
                    state_id: 0,
                };
                let _ = deletion(&before, "é", new, backwards);
            }
        }
    }
    assert!(deletion(
        &Record {
            text: "é".into(),
            caret: 1,
            state_id: 0
        },
        "",
        0,
        true
    )
    .is_none());
}

#[test]
fn every_scalar_deletion_and_caret_boundary_is_checked_in_both_directions() {
    let text = "aé✨e\u{301}👩\u{200d}💻אב";
    for start in (0..text.len()).filter(|offset| text.is_char_boundary(*offset)) {
        let ch = text[start..].chars().next().unwrap();
        let end = start + ch.len_utf8();
        let mut after = text.to_owned();
        after.replace_range(start..end, "");
        for backwards in [false, true] {
            let before = Record {
                text: text.into(),
                caret: if backwards { end } else { start },
                state_id: 0,
            };
            for caret in (0..=after.len()).filter(|offset| after.is_char_boundary(*offset)) {
                assert_eq!(
                    deletion(&before, &after, caret, backwards),
                    if caret == start { classify(ch) } else { None }
                );
            }
        }
    }
}

#[test]
fn state_caret_and_clock_discontinuities_break_deletion_groups() {
    let at = Instant::now();
    for discontinuity in 0..3 {
        let mut b = Buffer::new("abcde", 5);
        let mut h = History::default();
        b.delete(&mut h, true, at + Duration::from_secs(1));
        if discontinuity == 0 {
            b.state = 99;
        }
        if discontinuity == 1 {
            b.caret = 2;
        }
        let before = b.record();
        b.delete(
            &mut h,
            true,
            if discontinuity == 2 {
                at
            } else {
                at + Duration::from_secs(1)
            },
        );
        assert_eq!(h.peek_undo(), Some(&before));
    }
}

#[test]
fn no_op_at_document_edge_preserves_redo_and_seals_the_group() {
    let at = Instant::now();
    let mut b = Buffer::new("ab", 2);
    let mut h = History::default();
    b.delete(&mut h, true, at);
    b.undo(&mut h);
    assert!(h.peek_redo().is_some());
    b.delete(&mut h, false, at); // at EOF, no effect
    assert!(h.peek_undo().is_none());
    assert!(h.peek_redo().is_some());
    assert!(h.group.is_none());
    b.redo(&mut h);
    b.delete(&mut h, true, at);
    b.undo(&mut h);
    assert_eq!(b.text, "a");
}

#[test]
fn branching_after_undo_never_reuses_the_old_delete_group() {
    let at = Instant::now();
    let mut b = Buffer::new("abcd", 4);
    let mut h = History::default();
    b.delete(&mut h, true, at);
    b.delete(&mut h, true, at);
    b.undo(&mut h);
    b.delete(&mut h, true, at);
    assert_eq!(b.state, 3); // new branch uses a fresh ID
    assert!(h.peek_redo().is_none());
    b.undo(&mut h);
    assert_eq!(b.text, "abcd");
}

#[test]
fn deletion_groups_respect_byte_entry_and_missing_base_limits() {
    let at = Instant::now();
    let mut b = Buffer::new("abcde", 5);
    let mut h = History::with_limits(2, 4);
    b.delete(&mut h, true, at); // oversize base cannot be retained
    assert!(h.group.is_none());
    assert!(h.peek_undo().is_none());
    assert!(h.limited());
    b.delete(&mut h, true, at); // now the base fits
    assert!(h.group.is_some());
    h.undo.clear(); // simulate a missing/trimmed base; never merge into it
    b.delete(&mut h, true, at);
    assert_eq!(h.peek_undo().unwrap().text, "abc");
    b.undo(&mut h);
    assert!(h.undo.len() + h.redo.len() <= 2);
    assert!(
        h.undo
            .iter()
            .chain(&h.redo)
            .map(|r| r.text.len())
            .sum::<usize>()
            <= 4
    );
    let mut h = History::with_limits(0, 10);
    b.delete(&mut h, true, at);
    assert!(h.group.is_none());
    assert!(h.peek_undo().is_none());
}
