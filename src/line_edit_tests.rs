use super::*;
use crate::text_edit::valid_range;

fn apply(text: &str, selection: Range<usize>, action: Action) -> (String, usize) {
    let change = edit(text, selection, action).unwrap();
    assert!(valid_range(text, &change.range));
    assert!(change.replacement.is_char_boundary(change.caret));
    let mut result = text.to_owned();
    let caret = change.range.start + change.caret;
    result.replace_range(change.range, &change.replacement);
    assert!(result.is_char_boundary(caret));
    (result, caret)
}

#[test]
fn duplicate_first_and_middle_lines_keeps_the_original_column() {
    assert_eq!(
        apply("one\ntwo\nthree", 1..1, Action::Duplicate),
        ("one\none\ntwo\nthree".into(), 5)
    );
    assert_eq!(
        apply("one\ntwo\nthree", 6..6, Action::Duplicate),
        ("one\ntwo\ntwo\nthree".into(), 10)
    );
    assert_eq!(
        apply("one\ntwo", 3..3, Action::Duplicate),
        ("one\none\ntwo".into(), 7)
    );
}

#[test]
fn duplicate_last_line_does_not_invent_a_trailing_newline() {
    assert_eq!(
        apply("one\ntwo", 5..5, Action::Duplicate),
        ("one\ntwo\ntwo".into(), 9)
    );
    assert_eq!(
        apply("word", 4..4, Action::Duplicate),
        ("word\nword".into(), 9)
    );
    assert_eq!(
        apply("word\n", 2..2, Action::Duplicate),
        ("word\nword\n".into(), 7)
    );
}

#[test]
fn duplicate_empty_documents_and_empty_eof_lines_adds_one_logical_line() {
    assert_eq!(apply("", 0..0, Action::Duplicate), ("\n".into(), 1));
    assert_eq!(apply("a\n", 2..2, Action::Duplicate), ("a\n\n".into(), 3));
    assert_eq!(
        apply("a\n\nb", 2..2, Action::Duplicate),
        ("a\n\n\nb".into(), 3)
    );
}

#[test]
fn duplicate_partial_selection_copies_whole_lines_and_collapses_to_copy_start() {
    assert_eq!(
        apply("alpha\nbeta\ngamma", 2..8, Action::Duplicate),
        ("alpha\nbeta\nalpha\nbeta\ngamma".into(), 11)
    );
    assert_eq!(
        apply("alpha\nbeta", 2..8, Action::Duplicate),
        ("alpha\nbeta\nalpha\nbeta".into(), 11)
    );
}

#[test]
fn selection_ending_at_next_column_zero_excludes_that_line_for_both_actions() {
    let text = "alpha\nbeta\ngamma";
    assert_eq!(
        apply(text, 2..6, Action::Duplicate).0,
        "alpha\nalpha\nbeta\ngamma"
    );
    assert_eq!(apply(text, 2..6, Action::Delete), ("beta\ngamma".into(), 0));
    assert_eq!(
        apply(text, 2..11, Action::Duplicate).0,
        "alpha\nbeta\nalpha\nbeta\ngamma"
    );
    assert_eq!(apply(text, 2..11, Action::Delete), ("gamma".into(), 0));
}

#[test]
fn duplicate_all_lines_preserves_existing_terminal_lf_without_extra_blank_copy() {
    assert_eq!(
        apply("a\nb\n", 0..4, Action::Duplicate),
        ("a\nb\na\nb\n".into(), 4)
    );
    assert_eq!(
        apply("a\nb", 0..3, Action::Duplicate),
        ("a\nb\na\nb".into(), 4)
    );
    assert_eq!(
        apply("\n\n", 0..2, Action::Duplicate),
        ("\n\n\n\n".into(), 2)
    );
}

#[test]
fn duplicate_preserves_unicode_tabs_indentation_and_source_markup_literally() {
    let text = "before\n\t- [x] café✨\nafter";
    let caret = "before\n\t- [x] café".len();
    let (result, new_caret) = apply(text, caret..caret, Action::Duplicate);
    assert_eq!(result, "before\n\t- [x] café✨\n\t- [x] café✨\nafter");
    assert_eq!(new_caret, "before\n\t- [x] café✨\n\t- [x] café".len());
    // Unlike Enter continuation, copying does not increment numbers or reset tasks.
    assert_eq!(
        apply("009) [X] item", 0..0, Action::Duplicate).0,
        "009) [X] item\n009) [X] item"
    );
}

#[test]
fn delete_first_middle_and_last_lines_consumes_the_correct_separator() {
    let text = "one\ntwo\nthree";
    assert_eq!(apply(text, 1..1, Action::Delete), ("two\nthree".into(), 0));
    assert_eq!(apply(text, 5..5, Action::Delete), ("one\nthree".into(), 4));
    assert_eq!(apply(text, 9..9, Action::Delete), ("one\ntwo".into(), 7));
}

#[test]
fn delete_final_empty_line_removes_one_lf_and_single_remaining_line_can_be_empty() {
    assert_eq!(apply("a\n", 2..2, Action::Delete), ("a".into(), 1));
    assert_eq!(apply("a\n\n", 3..3, Action::Delete), ("a\n".into(), 2));
    assert_eq!(apply("\n", 1..1, Action::Delete), ("".into(), 0));
    assert_eq!(apply("\n", 0..0, Action::Delete), ("".into(), 0));
    assert_eq!(apply("only", 2..2, Action::Delete), ("".into(), 0));
    assert!(edit("", 0..0, Action::Delete).is_none());
}

#[test]
fn delete_nonfinal_line_preserves_a_following_empty_eof_line() {
    assert_eq!(apply("a\nb\n", 2..2, Action::Delete), ("a\n".into(), 2));
    assert_eq!(apply("a\nb\n", 0..0, Action::Delete), ("b\n".into(), 0));
}

#[test]
fn deleting_a_partial_multiline_selection_removes_all_and_only_touched_lines() {
    assert_eq!(
        apply("a\nb\nc\nd", 3..5, Action::Delete),
        ("a\nd".into(), 2)
    );
    assert_eq!(apply("a\nb\nc", 3..5, Action::Delete), ("a".into(), 1));
    assert_eq!(apply("a\nb\nc", 0..5, Action::Delete), ("".into(), 0));
}

#[test]
fn delete_eof_block_keeps_the_previous_unicode_line_and_caret_boundary() {
    assert_eq!(
        apply("é✨\n世界", 6..12, Action::Delete),
        ("é✨".into(), "é✨".len())
    );
    assert_eq!(apply("é✨\n世界\n", 0..12, Action::Delete), ("".into(), 0));
}

#[test]
fn invalid_reversed_out_of_bounds_and_split_scalar_ranges_are_refused() {
    for action in [
        Action::Duplicate,
        Action::Delete,
        Action::MoveUp,
        Action::MoveDown,
    ] {
        for range in [
            1..2,
            0..99,
            Range { start: 3, end: 0 },
            usize::MAX..usize::MAX,
        ] {
            assert!(edit("é\nx", range, action).is_none());
        }
    }
}

#[test]
fn operations_do_not_parse_or_reflow_markdown_code_fences_or_lists() {
    assert_eq!(
        apply("```\n- code\n```", 5..5, Action::Duplicate).0,
        "```\n- code\n- code\n```"
    );
    assert_eq!(
        apply("```\n- code\n```", 5..5, Action::Delete).0,
        "```\n```"
    );
    assert_eq!(
        apply("> quote\n> nested", 0..7, Action::Duplicate).0,
        "> quote\n> quote\n> nested"
    );
}

#[test]
fn all_valid_utf8_selection_ranges_produce_valid_transactions_and_exact_copies() {
    for text in [
        "",
        "é✨\n世界\n",
        "\n\n",
        "a\nb",
        "e\u{301}\nאב",
        "\t- [x] item",
    ] {
        let boundaries: Vec<_> = (0..=text.len())
            .filter(|offset| text.is_char_boundary(*offset))
            .collect();
        for &start in &boundaries {
            for &end in boundaries.iter().filter(|end| **end >= start) {
                let lines = line_range(text, start..end).unwrap();
                let body = &text[lines.clone()];
                let change = edit(text, start..end, Action::Duplicate).unwrap();
                assert!(change.range.is_empty());
                assert_eq!(
                    change
                        .replacement
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count(),
                    body.bytes().filter(|byte| *byte == b'\n').count() + 1
                );
                let (duplicated, _) = apply(text, start..end, Action::Duplicate);
                assert_eq!(duplicated.len(), text.len() + body.len() + 1);
                // Removing exactly the inserted bytes reconstructs the original.
                let mut restored = duplicated;
                restored.replace_range(
                    change.range.start..change.range.start + change.replacement.len(),
                    "",
                );
                assert_eq!(restored, text);
                if !text.is_empty() {
                    let change = edit(text, start..end, Action::Delete).unwrap();
                    assert!(change.replacement.is_empty());
                    assert!(change.range.start <= lines.start && change.range.end >= lines.end);
                    let (deleted, caret) = apply(text, start..end, Action::Delete);
                    assert!(deleted.len() < text.len());
                    assert_eq!(caret, change.range.start);
                }
            }
        }
    }
}

#[test]
fn move_up_swaps_with_one_previous_line_and_keeps_the_caret_column() {
    assert_eq!(
        apply("one\ntwo\nthree", 6..6, Action::MoveUp),
        ("two\none\nthree".into(), 2)
    );
    assert_eq!(
        apply("one\ntwo", 7..7, Action::MoveUp),
        ("two\none".into(), 3)
    );
}

#[test]
fn move_down_swaps_with_one_next_line_with_and_without_terminal_lf() {
    assert_eq!(
        apply("one\ntwo\nthree", 6..6, Action::MoveDown),
        ("one\nthree\ntwo".into(), 12)
    );
    assert_eq!(
        apply("one\ntwo\n", 1..1, Action::MoveDown),
        ("two\none\n".into(), 5)
    );
    assert_eq!(
        apply("one\ntwo", 1..1, Action::MoveDown),
        ("two\none".into(), 5)
    );
}

#[test]
fn move_partial_multiline_selection_up_keeps_the_whole_block_in_order() {
    assert_eq!(
        apply("head\nalpha\nbeta\ntail", 7..13, Action::MoveUp),
        ("alpha\nbeta\nhead\ntail".into(), 0)
    );
}

#[test]
fn move_partial_multiline_selection_down_collapses_to_the_moved_block_start() {
    assert_eq!(
        apply("head\nalpha\nbeta\ntail", 7..13, Action::MoveDown),
        ("head\ntail\nalpha\nbeta".into(), 10)
    );
}

#[test]
fn move_selection_ending_at_next_column_zero_does_not_move_that_next_line() {
    let text = "a\nb\nc\nd";
    assert_eq!(apply(text, 2..4, Action::MoveUp), ("b\na\nc\nd".into(), 0));
    assert_eq!(
        apply(text, 2..4, Action::MoveDown),
        ("a\nc\nb\nd".into(), 4)
    );
    assert_eq!(
        apply(text, 0..2, Action::MoveDown),
        ("b\na\nc\nd".into(), 2)
    );
}

#[test]
fn ordinary_blank_lines_remain_movable_before_the_terminal_sentinel() {
    let text = "a\n\nb\n";
    assert_eq!(apply(text, 2..2, Action::MoveUp), ("\na\nb\n".into(), 0));
    assert_eq!(apply(text, 2..2, Action::MoveDown), ("a\nb\n\n".into(), 4));
    assert_eq!(apply(text, 3..3, Action::MoveUp), ("a\nb\n\n".into(), 2));
}

#[test]
fn terminal_empty_line_is_not_a_movable_line_or_target() {
    for text in ["", "a\n", "a\nb\n", "\n\n"] {
        for action in [Action::MoveUp, Action::MoveDown] {
            assert!(edit(text, text.len()..text.len(), action).is_none());
        }
        if !text.is_empty() {
            let start = text[..text.len() - 1]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            assert!(edit(text, start..start, Action::MoveDown).is_none());
        }
    }
}

#[test]
fn move_at_document_boundaries_or_of_the_entire_document_is_a_noop() {
    for text in ["", "only", "a\nb", "a\nb\n"] {
        assert!(edit(text, 0..0, Action::MoveUp).is_none());
        assert!(edit(text, text.len()..text.len(), Action::MoveDown).is_none());
        for action in [Action::MoveUp, Action::MoveDown] {
            assert!(edit(text, 0..text.len(), action).is_none());
        }
    }
}

#[test]
fn identical_swaps_are_genuine_noops_not_caret_only_transforms() {
    assert!(edit("same\nsame", 6..6, Action::MoveUp).is_none());
    assert!(edit("same\nsame", 1..1, Action::MoveDown).is_none());
    assert!(edit("a\na\na", 2..5, Action::MoveUp).is_none());
    assert!(edit("\n\n", 0..0, Action::MoveDown).is_none());
}

#[test]
fn moving_unicode_keeps_byte_columns_without_visual_width_assumptions() {
    let text = "世界\n\te\u{301}👩\u{200d}💻\nאב";
    let caret = "世界\n\te\u{301}👩".len();
    let (moved, offset) = apply(text, caret..caret, Action::MoveUp);
    assert_eq!(moved, "\te\u{301}👩\u{200d}💻\n世界\nאב");
    assert_eq!(offset, "\te\u{301}👩".len());
    let (moved, offset) = apply(text, caret..caret, Action::MoveDown);
    assert_eq!(moved, "世界\nאב\n\te\u{301}👩\u{200d}💻");
    assert_eq!(offset, "世界\nאב\n\te\u{301}👩".len());
}

#[test]
fn moves_preserve_source_markup_and_do_not_renumber_reset_or_balance_it() {
    assert_eq!(
        apply("before\n009) [X] item\n~~~", 7..7, Action::MoveUp).0,
        "009) [X] item\nbefore\n~~~"
    );
    assert_eq!(
        apply("```\n- code\n```", 5..5, Action::MoveUp).0,
        "- code\n```\n```"
    );
}

#[test]
fn moving_single_lines_then_back_restores_text_and_every_valid_caret_column() {
    for text in ["a\né✨\n世界", "a\né✨\n世界\n", "a\n\nb", "e\u{301}\nאב"] {
        for caret in (0..=text.len()).filter(|offset| text.is_char_boundary(*offset)) {
            for (action, inverse) in [
                (Action::MoveUp, Action::MoveDown),
                (Action::MoveDown, Action::MoveUp),
            ] {
                if edit(text, caret..caret, action).is_some() {
                    let (moved, next_caret) = apply(text, caret..caret, action);
                    assert_eq!(
                        apply(&moved, next_caret..next_caret, inverse),
                        (text.into(), caret)
                    );
                }
            }
        }
    }
}

#[test]
fn all_move_ranges_match_an_independent_logical_line_permutation() {
    // Reference model deliberately swaps a Vec of logical lines, unlike the
    // production planner's localized byte-range replacement.
    fn reference(text: &str, selection: Range<usize>, up: bool) -> Option<String> {
        let first = text[..selection.start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count();
        let end = if !selection.is_empty() && text.as_bytes().get(selection.end - 1) == Some(&b'\n')
        {
            selection.end - 1
        } else {
            selection.end
        };
        let last = text[..end].bytes().filter(|byte| *byte == b'\n').count();
        let mut lines: Vec<_> = text.split('\n').collect();
        let content_lines = lines.len() - usize::from(text.ends_with('\n'));
        if first >= content_lines || (up && first == 0) || (!up && last + 1 >= content_lines) {
            return None;
        }
        if up {
            let previous = lines.remove(first - 1);
            lines.insert(last, previous);
        } else {
            let next = lines.remove(last + 1);
            lines.insert(first, next);
        }
        let result = lines.join("\n");
        (result != text && result.ends_with('\n') == text.ends_with('\n')).then_some(result)
    }
    for text in [
        "",
        "a",
        "a\nb",
        "a\nb\n\n",
        "é✨\n世界\n",
        "\n\n",
        "a\n\nb\nc",
        "e\u{301}\nאב",
        "a\na\na",
    ] {
        let boundaries: Vec<_> = (0..=text.len())
            .filter(|offset| text.is_char_boundary(*offset))
            .collect();
        for &start in &boundaries {
            for &end in boundaries.iter().filter(|end| **end >= start) {
                for (action, up) in [(Action::MoveUp, true), (Action::MoveDown, false)] {
                    let expected = reference(text, start..end, up);
                    let change = edit(text, start..end, action);
                    assert_eq!(
                        change.is_some(),
                        expected.is_some(),
                        "{text:?}, {start}..{end}, {up}"
                    );
                    if let Some(change) = change {
                        assert_eq!(change.range.len(), change.replacement.len());
                        let (result, _) = apply(text, start..end, action);
                        assert_eq!(Some(result.clone()), expected);
                        assert_eq!(result.len(), text.len());
                        assert_eq!(result.ends_with('\n'), text.ends_with('\n'));
                        assert_eq!(
                            result.bytes().filter(|byte| *byte == b'\n').count(),
                            text.bytes().filter(|byte| *byte == b'\n').count()
                        );
                        assert_eq!(result[..change.range.start], text[..change.range.start]);
                        assert_eq!(result[change.range.end..], text[change.range.end..]);
                        let mut before: Vec<_> = text.split('\n').collect();
                        let mut after: Vec<_> = result.split('\n').collect();
                        before.sort_unstable();
                        after.sort_unstable();
                        assert_eq!(before, after);
                    }
                }
            }
        }
    }
}

#[test]
fn blank_line_moves_that_would_create_a_terminal_lf_are_refused() {
    // Both directions represent the same swap at the final blank/nonblank edge.
    assert!(edit("a\n\nb", 2..2, Action::MoveDown).is_none());
    assert!(edit("a\n\nb", 3..3, Action::MoveUp).is_none());
    // A multi-line block ending in a blank line obeys the same policy.
    assert!(edit("a\n\nb", 0..3, Action::MoveDown).is_none());
    // Blank lines away from EOF remain movable without changing newline style.
    assert_eq!(apply("a\n\nb\nc", 2..2, Action::MoveDown).0, "a\nb\n\nc");
}
