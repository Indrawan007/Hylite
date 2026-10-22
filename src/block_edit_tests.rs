use super::*;

fn apply(text: &str, change: TextChange) -> String {
    assert!(valid_range(text, &change.range));
    assert!(change.replacement.is_char_boundary(change.caret));
    format!(
        "{}{}{}",
        &text[..change.range.start],
        change.replacement,
        &text[change.range.end..]
    )
}

fn formatted(text: &str, style: BlockStyle) -> String {
    apply(text, block(text, 0..text.len(), style).unwrap())
}

#[test]
fn headings_normalize_levels_and_toggle_only_when_all_match() {
    assert_eq!(
        formatted("# one\n## two\nplain", BlockStyle::Heading(2)),
        "## one\n## two\n## plain"
    );
    assert_eq!(
        formatted("## one\n## two", BlockStyle::Heading(2)),
        "one\ntwo"
    );
    assert_eq!(
        formatted("#not-heading\n####### literal", BlockStyle::Heading(1)),
        "# #not-heading\n# ####### literal"
    );
}

#[test]
fn blank_lines_indentation_and_trailing_lf_are_preserved() {
    assert_eq!(
        formatted("  one\n \n\ttwo\n", BlockStyle::BulletList),
        "  - one\n \n\t- two\n"
    );
    assert_eq!(formatted("\n\n", BlockStyle::Heading(1)), "# \n\n");
    assert_eq!(formatted("   ", BlockStyle::Heading(1)), "   # ");
}

#[test]
fn list_conversions_do_not_duplicate_markers_and_number_nonblank_lines() {
    assert_eq!(
        formatted("- é\n+ 世界\n* ✨", BlockStyle::BulletList),
        "é\n世界\n✨"
    );
    assert_eq!(
        formatted("- a\n\n7) b\nc", BlockStyle::OrderedList),
        "1. a\n\n2. b\n3. c"
    );
    assert_eq!(formatted("3. a\n4) b", BlockStyle::OrderedList), "a\nb");
    assert_eq!(
        formatted("12.   a\nplain", BlockStyle::BulletList),
        "-   a\n- plain"
    );
}

#[test]
fn task_conversion_preserves_checked_state_in_mixed_selections() {
    assert_eq!(
        formatted("- [X] done\nplain\n2. [ ] pending", BlockStyle::TaskList),
        "- [x] done\n- [ ] plain\n- [ ] pending"
    );
    assert_eq!(
        formatted("- [x] done\n- [ ] pending", BlockStyle::TaskList),
        "done\npending"
    );
    assert_eq!(
        formatted("- [x] done\n- pending", BlockStyle::BulletList),
        "- done\n- pending"
    );
    assert_eq!(
        formatted("- [x]word", BlockStyle::TaskList),
        "- [ ] [x]word"
    );
}

#[test]
fn marker_like_literals_and_long_numbers_are_not_stripped() {
    assert_eq!(
        formatted("-text\n1234567890. text\n*literal", BlockStyle::BulletList),
        "- -text\n- 1234567890. text\n- *literal"
    );
    assert_eq!(formatted("9.\n-", BlockStyle::OrderedList), "1. \n2. ");
}

#[test]
fn quote_removes_one_level_and_does_not_strip_other_prefix_families() {
    assert_eq!(
        formatted("> a\n>> nested", BlockStyle::Quote),
        "a\n> nested"
    );
    assert_eq!(
        formatted("# heading\n- item", BlockStyle::Quote),
        "> # heading\n> - item"
    );
    assert_eq!(formatted("- item", BlockStyle::Heading(1)), "# - item");
}

#[test]
fn block_targets_current_line_and_excludes_next_line_at_column_zero() {
    let text = "before\né✨\nafter";
    let end = "before\né✨\n".len();
    assert_eq!(
        apply(text, block(text, 9..end, BlockStyle::Heading(3)).unwrap()),
        "before\n### é✨\nafter"
    );
    assert_eq!(
        apply(text, block(text, 7..7, BlockStyle::BulletList).unwrap()),
        "before\n- é✨\nafter"
    );
}

#[test]
fn invalid_ranges_and_heading_levels_are_refused() {
    for level in [0, 7, 255] {
        assert!(block("text", 0..4, BlockStyle::Heading(level)).is_none());
    }
    for range in [1..2, 0..99, Range { start: 4, end: 0 }] {
        assert!(block("é✨", range.clone(), BlockStyle::Quote).is_none());
        assert!(link("é✨", range.clone()).is_none());
        assert!(code_block("é✨", range).is_none());
    }
}

#[test]
fn links_escape_label_brackets_and_backslashes_and_position_url_caret() {
    let text = "é [x]\\path";
    let change = link(text, 0..text.len()).unwrap();
    assert_eq!(
        &change.replacement[..change.caret],
        "[é \\[x\\]\\\\path](https://"
    );
    assert_eq!(apply(text, change), "[é \\[x\\]\\\\path](https://)");
    assert_eq!(apply("", link("", 0..0).unwrap()), "[text](https://)");
    assert_eq!(
        apply("a世界b", link("a世界b", 1..7).unwrap()),
        "a[世界](https://)b"
    );
}

#[test]
fn multiline_and_control_character_link_labels_are_refused() {
    for text in ["a\nb", "a\rb", "a\tb", "\u{7f}"] {
        assert!(link(text, 0..text.len()).is_none());
    }
}

#[test]
fn code_block_wraps_whole_lines_and_places_caret_inside_payload() {
    let text = "before\né✨\nafter";
    let caret = "before\né".len();
    let change = code_block(text, caret..caret).unwrap();
    assert_eq!(change.caret, "```\né".len());
    assert_eq!(apply(text, change), "before\n```\né✨\n```\nafter");
    assert_eq!(apply("", code_block("", 0..0).unwrap()), "```\n\n```");
}

#[test]
fn code_fences_are_longer_than_payload_runs_and_preserve_final_lf() {
    let text = "```rust\nlet x = ``;\n```\n";
    let change = code_block(text, 0..text.len()).unwrap();
    assert_eq!(change.replacement, "````\n```rust\nlet x = ``;\n```\n````");
    assert_eq!(
        apply(text, change),
        "````\n```rust\nlet x = ``;\n```\n````\n"
    );
    assert_eq!(
        apply("a\nb", code_block("a\nb", 0..2).unwrap()),
        "```\na\n```\nb"
    );
}

#[test]
fn every_menu_style_is_valid_and_has_a_label() {
    for style in BlockStyle::ALL {
        assert!(!style.to_string().is_empty());
        assert!(block("text", 0..4, style).is_some());
    }
}

#[test]
fn utf8_boundary_matrix_keeps_transactions_valid_and_outside_text_untouched() {
    for text in ["", "\n", "é✨\nאב\n", "  a\n\t\n# b", "e\u{301} [x] `"] {
        let boundaries: Vec<_> = (0..=text.len())
            .filter(|i| text.is_char_boundary(*i))
            .collect();
        for &start in &boundaries {
            for &end in boundaries.iter().filter(|end| **end >= start) {
                for style in BlockStyle::ALL {
                    apply(text, block(text, start..end, style).unwrap());
                }
                apply(text, code_block(text, start..end).unwrap());
                if let Some(change) = link(text, start..end) {
                    apply(text, change);
                }
            }
        }
    }
}
