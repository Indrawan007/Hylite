use super::*;

fn settings() -> Settings {
    Settings {
        enabled: true,
        dark: true,
        buffer_key: (0, 0),
    }
}
fn lexer() -> Lexer {
    Lexer::new(settings())
}
fn tokens(line: &str) -> Vec<(String, Kind)> {
    let spans = lexer().highlight_line(line);
    valid(line, &spans);
    spans
        .into_iter()
        .map(|(range, kind)| (line[range].to_owned(), kind))
        .collect()
}
fn valid(line: &str, spans: &[(Range<usize>, Kind)]) {
    let mut end = 0;
    for (range, _) in spans {
        assert!(range.start >= end && range.start < range.end && range.end <= line.len());
        assert!(line.is_char_boundary(range.start) && line.is_char_boundary(range.end));
        end = range.end;
    }
}

#[test]
fn atx_headings_accept_one_to_six_and_require_a_separator() {
    for level in 1..=6 {
        let line = format!("{} 世界", "#".repeat(level));
        assert_eq!(tokens(&line), vec![(line, Kind::Heading)]);
    }
    for line in ["#word", "####### word", "    # code", "\t# code"] {
        assert!(tokens(line).is_empty());
    }
}

#[test]
fn quote_list_and_task_prefixes_are_separate_ordered_spans() {
    assert_eq!(
        tokens(">> - [X] é"),
        vec![
            (">> ".into(), Kind::Quote),
            ("- ".into(), Kind::Marker),
            ("[X]".into(), Kind::Task)
        ]
    );
    assert_eq!(tokens("12) text"), vec![("12) ".into(), Kind::Marker)]);
    assert!(tokens("1234567890. text").is_empty());
    assert!(tokens("-not-list").is_empty());
    assert_eq!(tokens("- [x]word"), vec![("- ".into(), Kind::Marker)]);
}

#[test]
fn thematic_rules_require_three_matching_markers() {
    for line in ["---", "* * *", "  _\t_ _"] {
        assert!(tokens(line).iter().any(|(_, kind)| *kind == Kind::Marker));
    }
    assert!(!tokens("- -")
        .iter()
        .any(|(text, kind)| *kind == Kind::Marker && text == "- -"));
    // This is a bullet item, not a thematic rule over the entire line.
    assert_eq!(tokens("* - *"), vec![("* ".into(), Kind::Marker)]);
}

#[test]
fn inline_unicode_emphasis_strong_and_triple_markers_have_byte_ranges() {
    assert_eq!(
        tokens("é *世界* **✨** ***both***"),
        vec![
            ("*世界*".into(), Kind::Emphasis),
            ("**✨**".into(), Kind::Strong),
            ("***both***".into(), Kind::Strong)
        ]
    );
    assert_eq!(
        tokens("_word_ __strong__"),
        vec![
            ("_word_".into(), Kind::Emphasis),
            ("__strong__".into(), Kind::Strong)
        ]
    );
}

#[test]
fn whitespace_and_intraword_underscore_do_not_create_emphasis() {
    for line in [
        "* space*",
        "*space *",
        "file_name_more",
        "é_世界_字",
        "__ word__",
    ] {
        // `* space*` has a valid bullet marker but no emphasis span.
        assert!(
            !tokens(line)
                .iter()
                .any(|(_, kind)| matches!(kind, Kind::Emphasis | Kind::Strong)),
            "{line}"
        );
    }
}

#[test]
fn code_spans_take_precedence_and_match_exact_backtick_runs() {
    assert_eq!(
        tokens("`**not bold**` ``a`b``"),
        vec![
            ("`**not bold**`".into(), Kind::Code),
            ("``a`b``".into(), Kind::Code)
        ]
    );
    assert_eq!(tokens(r"`x\` tail"), vec![(r"`x\`".into(), Kind::Code)]);
    assert!(tokens("`unclosed").is_empty());
}

#[test]
fn backslash_escapes_do_not_open_inline_constructs() {
    assert!(tokens(r"\*not* \[not](url) \`not`").is_empty());
    assert_eq!(
        tokens(r"**not\** still**"),
        vec![(r"**not\** still**".into(), Kind::Strong)]
    );
}

#[test]
fn links_and_images_support_nested_brackets_parentheses_and_escapes() {
    for line in [
        "[é](https://x/a(b))",
        "![x](image.png)",
        r"[a\]b](a\)b)",
        "[a[b]c](url)",
        "[empty]()",
    ] {
        assert_eq!(tokens(line), vec![(line.into(), Kind::Link)], "{line}");
    }
}

#[test]
fn malformed_links_and_reference_links_remain_plain() {
    for line in [
        "[missing",
        "[text]",
        "[text](unclosed",
        "[text][id]",
        "[x](a(b)",
    ] {
        assert!(tokens(line).is_empty(), "{line}");
    }
}

#[test]
fn fenced_code_suppresses_markdown_until_a_matching_close() {
    let mut h = lexer();
    assert_eq!(h.highlight_line("```rust"), vec![(0..7, Kind::Marker)]);
    assert_eq!(
        h.highlight_line("# *not a heading*"),
        vec![(0..17, Kind::Code)]
    );
    assert_eq!(h.highlight_line("~~~"), vec![(0..3, Kind::Code)]);
    assert_eq!(h.highlight_line("````"), vec![(0..4, Kind::Marker)]);
    assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
}

#[test]
fn tilde_fences_and_up_to_three_spaces_are_supported() {
    let mut h = lexer();
    h.highlight_line("   ~~~~ language");
    assert_eq!(h.highlight_line("text"), vec![(0..4, Kind::Code)]);
    assert_eq!(h.highlight_line("  ~~~~  \t"), vec![(0..9, Kind::Marker)]);
    assert!(h.highlight_line("plain").is_empty());
    for line in ["    ```", "\t~~~", "> ```", "- ```"] {
        let mut h = lexer();
        h.highlight_line(line);
        assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
    }
}

#[test]
fn short_or_annotated_closing_fences_do_not_close_the_block() {
    let mut h = lexer();
    h.highlight_line("````");
    assert_eq!(h.highlight_line("```"), vec![(0..3, Kind::Code)]);
    assert_eq!(h.highlight_line("```` trailing"), vec![(0..13, Kind::Code)]);
    h.highlight_line("````");
    assert!(h.highlight_line("text").is_empty());
}

#[test]
fn backticks_in_backtick_fence_info_prevent_opening_a_fence() {
    let mut h = lexer();
    h.highlight_line("```bad`info");
    assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
}

#[test]
fn empty_lines_keep_fence_context_and_advance_the_frontier() {
    let mut h = lexer();
    h.highlight_line("~~~");
    assert!(h.highlight_line("").is_empty());
    assert_eq!(h.current_line(), 2);
    assert_eq!(h.highlight_line("é"), vec![(0..2, Kind::Code)]);
}

#[test]
fn editing_the_opener_invalidates_downstream_fence_context() {
    let mut h = lexer();
    h.highlight_line("```");
    h.highlight_line("# code");
    h.highlight_line("```");
    h.change_line(0);
    assert_eq!(h.current_line(), 0);
    h.highlight_line("plain");
    assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
}

#[test]
fn rewinding_inside_a_fence_keeps_the_state_before_that_line() {
    let mut h = lexer();
    h.highlight_line("```");
    h.highlight_line("# code");
    h.highlight_line("```");
    h.change_line(1);
    assert_eq!(h.highlight_line("**changed**"), vec![(0..11, Kind::Code)]);
    h.highlight_line("```");
    assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
}

#[test]
fn notifications_beyond_the_frontier_do_not_skip_unknown_context() {
    let mut h = lexer();
    h.highlight_line("```");
    h.change_line(999);
    assert_eq!(h.current_line(), 1);
    assert_eq!(h.highlight_line("text"), vec![(0..4, Kind::Code)]);
    h.change_line(0);
    h.change_line(50);
    assert_eq!(h.current_line(), 0);
}

#[test]
fn incremental_insert_delete_and_edit_match_a_fresh_lexer() {
    for (old, new, dirty) in [
        ("a\n```\nx\n```\nend", "a\n```\nx\n```\n# title", 4),
        (
            "a\n```\n# code\n```\n# title",
            "a\nplain\n# code\n```\n# title",
            1,
        ),
        ("a\n```\nx\n```\nz", "a\n```\ninserted\nx\n```\nz", 2),
        ("a\n```\nx\n```\nz", "a\n```\n```\nz", 2),
    ] {
        let mut h = lexer();
        for line in old.split('\n') {
            h.highlight_line(line);
        }
        h.change_line(dirty);
        let got: Vec<_> = new
            .split('\n')
            .skip(dirty)
            .map(|line| h.highlight_line(line))
            .collect();
        let mut fresh = lexer();
        // Collect BEFORE slicing: Iterator::skip can optimize away map calls,
        // but the lexer must consume every preceding line to build context.
        let expected: Vec<_> = new
            .split('\n')
            .map(|line| fresh.highlight_line(line))
            .collect();
        assert_eq!(got, expected[dirty..]);
    }
}

#[test]
fn setting_changes_reset_but_identical_settings_keep_the_frontier() {
    let mut h = lexer();
    h.highlight_line("```");
    h.update(settings());
    assert_eq!(h.current_line(), 1);
    for changed in [
        Settings {
            dark: false,
            ..settings()
        },
        Settings {
            buffer_key: (0, 1),
            ..settings()
        },
        Settings {
            buffer_key: (1, 0),
            ..settings()
        },
    ] {
        let mut h = lexer();
        h.highlight_line("```");
        h.update(changed);
        assert_eq!(h.current_line(), 0);
        assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
    }
}

#[test]
fn disabling_returns_no_spans_and_does_not_retain_line_context() {
    let mut h = lexer();
    h.highlight_line("```");
    h.update(Settings {
        enabled: false,
        ..settings()
    });
    for _ in 0..500 {
        assert!(h.highlight_line("# *code*").is_empty());
    }
    assert_eq!(h.current_line(), 500);
    assert_eq!(h.states.len(), 1);
    h.change_line(100);
    assert_eq!(h.current_line(), 100);
    h.update(settings());
    assert_eq!(h.current_line(), 0);
    assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
}

#[test]
fn oversized_lines_are_plain_but_still_update_fence_context() {
    let mut h = lexer();
    assert!(h
        .highlight_line(&format!("```{}", " ".repeat(MAX_LINE_BYTES)))
        .is_empty());
    assert_eq!(h.highlight_line("# code"), vec![(0..6, Kind::Code)]);
    assert!(h.highlight_line(&"x".repeat(MAX_LINE_BYTES + 1)).is_empty());
    assert!(h
        .highlight_line(&format!("```{}", " ".repeat(MAX_LINE_BYTES)))
        .is_empty());
    assert_eq!(h.highlight_line("# title"), vec![(0..7, Kind::Heading)]);
    assert!(lexer()
        .highlight_line(&format!("# {}", "x".repeat(MAX_LINE_BYTES)))
        .is_empty());
}

#[test]
fn adversarial_unmatched_delimiters_terminate_with_valid_spans() {
    for pattern in ["[", "*_", "[a](b(", "`a**_", "\\[é_"] {
        let line: String = pattern.repeat(MAX_LINE_BYTES / pattern.len());
        valid(&line, &lexer().highlight_line(&line));
    }
}

#[test]
fn utf8_fixture_matrix_has_nonoverlapping_nonempty_boundary_aligned_spans() {
    let pieces = [
        "é", "אב", "e\u{301}", "✨", "*", "**", "`", "[", "](x)", "\\", " ", "\t", "\u{2028}",
    ];
    for a in pieces {
        for b in pieces {
            for c in pieces {
                let line = format!("{a}{b}{c}");
                valid(&line, &lexer().highlight_line(&line));
            }
        }
    }
}

#[test]
fn token_colors_have_distinct_light_and_dark_variants() {
    for kind in [
        Kind::Heading,
        Kind::Marker,
        Kind::Quote,
        Kind::Task,
        Kind::Strong,
        Kind::Emphasis,
        Kind::Code,
        Kind::Link,
    ] {
        assert_ne!(color(kind, true), color(kind, false));
    }
}
