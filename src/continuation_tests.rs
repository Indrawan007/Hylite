use super::*;

fn apply(text: &str, caret: usize) -> (String, usize) {
    let change = enter(text, caret).expect("supported list Enter");
    assert!(crate::text_edit::valid_range(text, &change.range));
    assert!(change.replacement.is_char_boundary(change.caret));
    let caret = change.range.start + change.caret;
    let result = format!(
        "{}{}{}",
        &text[..change.range.start],
        change.replacement,
        &text[change.range.end..]
    );
    assert!(result.is_char_boundary(caret));
    (result, caret)
}
fn end(text: &str) -> String {
    apply(text, text.len()).0
}

#[test]
fn bullet_styles_indentation_and_separator_spacing_are_preserved() {
    for prefix in ["- ", "+ ", "* ", "  -  ", "   + ", "-\t", "- \t"] {
        let text = format!("{prefix}item");
        assert_eq!(end(&text), format!("{text}\n{prefix}"));
    }
}

#[test]
fn ordered_numbers_increment_preserving_delimiter_and_zero_padding() {
    for (before, after) in [
        ("0. x", "1. "),
        ("9) x", "10) "),
        ("009. x", "010. "),
        ("099) x", "100) "),
        ("  3. x", "  4. "),
    ] {
        assert_eq!(end(before), format!("{before}\n{after}"));
    }
}

#[test]
fn ordered_overflow_and_ten_digit_markers_fall_back_to_native() {
    for text in ["999999999. x", "1234567890) x"] {
        assert!(enter(text, text.len()).is_none());
    }
    assert_eq!(end("999999998. x"), "999999998. x\n999999999. ");
}

#[test]
fn tasks_continue_unchecked_with_the_original_list_style() {
    for (before, after) in [
        ("- [x] done", "- [ ] "),
        ("+ [X] done", "+ [ ] "),
        ("* [ ] todo", "* [ ] "),
        ("3) [x] done", "4) [ ] "),
        ("-  [x]  done", "-  [ ]  "),
    ] {
        assert_eq!(end(before), format!("{before}\n{after}"));
    }
    assert_eq!(end("- [x]word"), "- [x]word\n- ");
}

#[test]
fn empty_items_exit_in_place_without_an_extra_newline() {
    for text in ["-", "- ", "+ \t", "1.", "009) ", "- [x]", "- [ ]   "] {
        assert_eq!(end(text), "");
    }
    assert_eq!(end("  - [X] \t"), "  ");
    assert_eq!(apply("before\n- \nafter", 9), ("before\n\nafter".into(), 7));
}

#[test]
fn only_ascii_blank_item_bodies_are_removed() {
    assert_eq!(end("- \u{a0}"), "- \u{a0}\n- ");
    assert_eq!(end("- \u{2003}"), "- \u{2003}\n- ");
}

#[test]
fn splitting_unicode_text_preserves_both_halves_and_final_lf() {
    let text = "before\n- é世界✨\nafter\n";
    let caret = "before\n- é".len();
    let (result, next) = apply(text, caret);
    assert_eq!(result, "before\n- é\n- 世界✨\nafter\n");
    assert_eq!(next, "before\n- é\n- ".len());
    assert_eq!(apply("- abc", 2), ("- \n- abc".into(), 5));
}

#[test]
fn carets_in_markers_or_checkboxes_do_not_continue() {
    for text in ["12. body", "- [x] body"] {
        let body = item(text).unwrap().body;
        for caret in 0..body {
            assert!(enter(text, caret).is_none(), "{text}: {caret}");
        }
    }
}

#[test]
fn plain_quoted_deeply_indented_and_code_like_lines_are_not_guessed() {
    for text in [
        "plain",
        "    - code",
        "\t- code",
        "> - quoted",
        "-     code",
        "-word",
        "# title",
        "-\t\tcode",
    ] {
        assert!(enter(text, text.len()).is_none(), "{text}");
    }
}

#[test]
fn thematic_rules_are_not_mistaken_for_bullet_items() {
    for text in ["---", "- - -", "* * *", "  *\t* *", "___"] {
        assert!(enter(text, text.len()).is_none(), "{text}");
    }
    assert_eq!(end("* - *"), "* - *\n* ");
}

#[test]
fn top_level_fences_suppress_auto_continuation_until_the_matching_close() {
    for marker in ["```", "~~~"] {
        let text = format!("{marker}\n- code");
        assert!(enter(&text, text.len()).is_none());
        let closed = format!("{marker}\n- code\n{marker}\n- item");
        assert_eq!(end(&closed), format!("{closed}\n- "));
    }
}

#[test]
fn payload_list_or_quote_prefixes_cannot_prematurely_close_a_root_fence() {
    for false_close in ["- ```", "> ```", "    ```", "``` info", "~~~", "``"] {
        let text = format!("```\n{false_close}\n- still code");
        assert!(enter(&text, text.len()).is_none(), "{false_close}");
    }
}

#[test]
fn common_list_and_quote_fences_are_guarded_and_close_in_their_own_container() {
    for (open, close) in [
        ("- ```rust", "  ```"),
        ("- - ```", "    ```"),
        ("> ```", "> ```"),
        ("> - ~~~", ">   ~~~"),
        ("    ```", "    ```"),
    ] {
        let text = format!("{open}\n- payload");
        assert!(enter(&text, text.len()).is_none(), "{open}");
        let closed = format!("{text}\n{close}\n- item");
        assert_eq!(end(&closed), format!("{closed}\n- "), "{open}");
    }
}

#[test]
fn enter_on_a_fence_opening_item_is_native_not_another_list_marker() {
    for text in ["- ```", "- ~~~rust", "1. ```rs", "  + ```", "- - ```"] {
        assert!(enter(text, text.len()).is_none(), "{text}");
    }
    assert_eq!(end("- ```bad`info"), "- ```bad`info\n- ");
}

#[test]
fn fence_length_info_and_quote_depth_are_respected() {
    for prefix in ["````\n```\n", "> ```\n>> ```\n"] {
        let text = format!("{prefix}- payload");
        assert!(enter(&text, text.len()).is_none());
    }
    assert_eq!(end("```bad`info\n- item"), "```bad`info\n- item\n- ");
    assert_eq!(end("   ```\n```\n- item"), "   ```\n```\n- item\n- ");
}

#[test]
fn invalid_utf8_offsets_and_long_lines_fall_back_without_truncation() {
    for caret in [1, 999, usize::MAX] {
        assert!(enter("é", caret).is_none());
    }
    let text = format!("- {}", "a".repeat(MAX_LINE_BYTES));
    assert!(enter(&text, text.len()).is_none());
}

#[test]
fn utf8_caret_matrix_produces_valid_ranges_and_never_loses_nonblank_body() {
    for text in [
        "- é✨",
        "2) 世界\n",
        "- [x] אב",
        "before\n  + e\u{301}\nafter",
        "```\n- ✨",
        "- \n",
    ] {
        for caret in (0..=text.len()).filter(|offset| text.is_char_boundary(*offset)) {
            if enter(text, caret).is_some() {
                apply(text, caret);
            }
        }
    }
}
