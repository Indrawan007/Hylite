use super::*;

fn text(block: &Block) -> String {
    block.runs.iter().map(|run| run.text.as_str()).collect()
}

fn validate(blocks: &[Block], source: &str, parent: Range<usize>) {
    for block in blocks {
        assert!(
            parent.start <= block.source.start && block.source.end <= parent.end,
            "{block:?}"
        );
        assert!(source.get(block.source.clone()).is_some());
        for run in &block.runs {
            assert!(block.source.start <= run.source.start && run.source.end <= block.source.end);
            assert!(source.get(run.source.clone()).is_some());
        }
        validate(&block.children, source, block.source.clone());
    }
}

#[test]
fn empty_and_whitespace_documents_have_no_fake_blocks() {
    for source in ["", "\n\n", " \t\n"] {
        assert!(parse(source).is_empty());
    }
}

#[test]
fn headings_have_levels_and_inline_content_without_markers() {
    for level in 1..=6 {
        let source = format!("{} **café**\n", "#".repeat(level));
        let blocks = parse(&source);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, Kind::Heading(level as u8));
        assert_eq!(text(&blocks[0]), "café");
        assert!(blocks[0].runs.iter().all(|run| run.style.strong));
    }
}

#[test]
fn setext_headings_and_rules_remain_distinct() {
    let blocks = parse("Heading\n=======\n\n---\n");
    assert_eq!(blocks[0].kind, Kind::Heading(1));
    assert_eq!(blocks[1].kind, Kind::Rule);
}

#[test]
fn nested_styles_restore_outer_state_after_closing() {
    let blocks = parse("plain **bold *both* end** plain ~~gone~~\n");
    let runs = &blocks[0].runs;
    assert!(runs
        .iter()
        .any(|r| r.text == "both" && r.style.strong && r.style.emphasis));
    assert!(runs
        .iter()
        .any(|r| r.text == " end" && r.style.strong && !r.style.emphasis));
    assert!(runs.iter().any(|r| r.text == " plain " && !r.style.strong));
    assert!(runs.iter().any(|r| r.text == "gone" && r.style.strike));
    assert_eq!(text(&blocks[0]), "plain bold both end plain gone");
}

#[test]
fn code_spans_do_not_parse_their_markdown_as_styles() {
    let blocks = parse("a `**literal**` b");
    let code = blocks[0].runs.iter().find(|run| run.style.code).unwrap();
    assert_eq!(code.text, "**literal**");
    assert!(!code.style.strong);
}

#[test]
fn links_preserve_metadata_and_nested_styles_but_not_actions() {
    let blocks = parse("[**label**](javascript:alert%281%29) after");
    let run = blocks[0]
        .runs
        .iter()
        .find(|run| run.text == "label")
        .unwrap();
    assert_eq!(run.style.link.as_deref(), Some("javascript:alert%281%29"));
    assert!(run.style.strong);
    assert!(blocks[0].runs.last().unwrap().style.link.is_none());
}

#[test]
fn reference_links_are_resolved_by_the_existing_parser() {
    let blocks = parse("[label][id]\n\n[id]: https://example.invalid/path\n");
    assert_eq!(blocks.len(), 1);
    assert_eq!(
        blocks[0].runs[0].style.link.as_deref(),
        Some("https://example.invalid/path")
    );
}

#[test]
fn soft_break_is_a_space_and_hard_break_is_a_newline() {
    let blocks = parse("one\ntwo  \nthree\\\nfour");
    assert_eq!(text(&blocks[0]), "one two\nthree\nfour");
}

#[test]
fn fenced_code_preserves_tabs_spaces_and_empty_lines_in_payload() {
    let blocks = parse("```rust title\n\tlet x = 1;  \n\n```\n");
    assert_eq!(
        blocks[0].kind,
        Kind::Code {
            info: "rust title".into(),
            text: "\tlet x = 1;  \n\n".into()
        }
    );
}

#[test]
fn indented_and_unclosed_code_are_not_silently_trimmed() {
    assert_eq!(
        parse("    a  \n    b\n")[0].kind,
        Kind::Code {
            info: String::new(),
            text: "a  \nb\n".into()
        }
    );
    assert_eq!(
        parse("```\nlast\n\n")[0].kind,
        Kind::Code {
            info: String::new(),
            text: "last\n\n".into()
        }
    );
}

#[test]
fn nested_list_retains_ordered_start_and_parent_child_structure() {
    let blocks = parse("3. first\n   - child\n4. second\n");
    assert_eq!(blocks[0].kind, Kind::List(Some(3)));
    let items = &blocks[0].children;
    assert_eq!(items.len(), 2);
    assert_eq!(text(&items[0].children[0]), "first");
    assert_eq!(items[0].children[1].kind, Kind::List(None));
    assert_eq!(text(&items[1].children[0]), "second");
}

#[test]
fn loose_lists_keep_separate_paragraphs_in_the_same_item() {
    let blocks = parse("- first\n\n  second paragraph\n\n- next\n");
    assert_eq!(blocks[0].children.len(), 2);
    assert_eq!(blocks[0].children[0].children.len(), 2);
    assert_eq!(text(&blocks[0].children[0].children[1]), "second paragraph");
}

#[test]
fn task_markers_belong_to_the_correct_item_in_tight_and_loose_lists() {
    for source in ["- [x] done\n- [ ] todo\n", "- [x] done\n\n- [ ] todo\n"] {
        let blocks = parse(source);
        assert_eq!(blocks[0].children[0].kind, Kind::Item(Some(true)));
        assert_eq!(blocks[0].children[1].kind, Kind::Item(Some(false)));
        assert_eq!(text(&blocks[0].children[0].children[0]), "done");
    }
}

#[test]
fn nested_task_markers_do_not_leak_to_the_parent_item() {
    let blocks = parse("- parent\n  - [x] child\n");
    let parent = &blocks[0].children[0];
    assert_eq!(parent.kind, Kind::Item(None));
    assert_eq!(parent.children[1].children[0].kind, Kind::Item(Some(true)));
}

#[test]
fn quotes_preserve_paragraphs_lists_and_code_children() {
    let blocks = parse("> paragraph\n>\n> - child\n>\n> ```\n> code\n> ```\n");
    assert_eq!(blocks[0].kind, Kind::Quote);
    assert_eq!(blocks[0].children.len(), 3);
    assert_eq!(blocks[0].children[1].kind, Kind::List(None));
    assert!(matches!(blocks[0].children[2].kind, Kind::Code { .. }));
}

#[test]
fn table_fallback_keeps_exact_source_and_does_not_swallow_following_blocks() {
    let table = "| a | b |\n| --- | --- |\n| c | d |\n";
    let blocks = parse(&format!("{table}\nafter\n"));
    assert_eq!(
        blocks[0].kind,
        Kind::Literal {
            label: "Table — source fallback",
            text: table.into()
        }
    );
    assert_eq!(text(&blocks[1]), "after");
}

#[test]
fn raw_html_is_visible_inert_source_not_executed_or_erased() {
    let source = "<script>alert(1)</script>\n";
    assert_eq!(
        parse(source)[0].kind,
        Kind::Literal {
            label: "Raw HTML — inert source",
            text: source.into()
        }
    );
    let blocks = parse("hello <b>raw</b>");
    assert_eq!(text(&blocks[0]), "hello <b>raw</b>");
}

#[test]
fn images_and_math_have_explicit_literal_fallbacks() {
    let blocks = parse("![alt](https://example.invalid/x.png) $x+1$");
    let content = text(&blocks[0]);
    assert!(content.contains("[image not loaded: ![alt](https://example.invalid/x.png)]"));
    assert!(content.contains("$x+1$"));
    assert!(blocks[0]
        .runs
        .iter()
        .filter(|run| run.text.contains("image not loaded") || run.text.contains("$x+1$"))
        .all(|run| run.style.code));
}

#[test]
fn utf8_provenance_is_in_bounds_even_when_entities_and_escapes_change_display_text() {
    let source = "# é **世🙂**\n\n> café &amp; \\*literal\\*\n>\n> - [x] [label](https://example.invalid)\n\n```\n\tcode  \n```\n";
    let before = source.to_owned();
    let blocks = parse(source);
    validate(&blocks, source, 0..source.len());
    assert_eq!(source, before);
    assert_eq!(blocks, parse(source));
    let quote = &blocks[1];
    assert_eq!(text(&quote.children[0]), "café & *literal*");
}

#[test]
fn deeply_nested_input_has_an_explicit_bounded_fallback_and_keeps_the_tail() {
    let source = format!("{}deep\n\nafter\n", "> ".repeat(MAX_NESTING + 8));
    let blocks = parse(&source);
    validate(&blocks, &source, 0..source.len());
    let mut current = &blocks[0];
    let mut depth = 0;
    while let Some(child) = current.children.first() {
        current = child;
        depth += 1;
    }
    assert!(depth <= MAX_NESTING);
    assert!(matches!(
        current.kind,
        Kind::Literal {
            label: "Preview nesting limit — source",
            ..
        }
    ));
    assert_eq!(text(blocks.last().unwrap()), "after");
}

#[test]
fn footnote_references_and_definitions_are_visible_as_source() {
    let source = "ref[^a]\n\n[^a]: note **bold**\n";
    let blocks = parse(source);
    assert_eq!(text(&blocks[0]), "ref[^a]");
    assert!(
        matches!(&blocks[1].kind, Kind::Literal { label: "Footnote — source fallback", text } if text.contains("[^a]: note **bold**"))
    );
    validate(&blocks, source, 0..source.len());
}

#[test]
fn inline_depth_limit_keeps_literal_source_and_restores_outer_style() {
    let mut nested = "deep".to_owned();
    for _ in 0..MAX_NESTING + 8 {
        nested = format!("*a {nested} b*");
    }
    let source = format!("{nested} after");
    let blocks = parse(&source);
    assert!(blocks[0]
        .runs
        .iter()
        .any(|run| run.text.contains("[inline nesting limit:") && run.text.contains("deep")));
    let last = blocks[0].runs.last().unwrap();
    assert_eq!(last.text, " after");
    assert!(!last.style.emphasis);
    validate(&blocks, &source, 0..source.len());
}

#[test]
fn malformed_markers_remain_literal_and_provenance_is_valid() {
    for source in [
        "*unclosed",
        "[missing](",
        "![broken",
        "<not a tag",
        "**",
        "`unclosed",
        "\0é\n\t",
    ] {
        let blocks = parse(source);
        validate(&blocks, source, 0..source.len());
        assert!(!blocks.is_empty());
    }
}
