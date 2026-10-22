use super::*;

fn offset(text: &str, query: &str) -> usize {
    let destination = resolve(text, query).unwrap();
    assert!(text.is_char_boundary(destination.offset));
    destination.offset
}

#[test]
fn addresses_are_one_based_and_line_only_defaults_to_column_one() {
    let text = "first\nsecond\nthird";
    assert_eq!(offset(text, "1"), 0);
    assert_eq!(offset(text, "2"), 6);
    assert_eq!(offset(text, "3:3"), 15);
    assert_eq!(offset(text, "  002:003\t"), 8);
    assert_eq!(resolve(text, "2:3").unwrap().line, 2);
    assert_eq!(resolve(text, "2:3").unwrap().column, 3);
}

#[test]
fn empty_documents_and_trailing_empty_lines_are_addressable() {
    assert_eq!(offset("", "1:1"), 0);
    assert_eq!(offset("a\n", "2:1"), 2);
    assert_eq!(offset("\n\n", "3"), 2);
    assert_eq!(offset("a\n\nb", "2:1"), 2);
    assert_eq!(resolve("", "2"), Err(Error::LineOutOfRange));
    assert_eq!(resolve("a\n", "3"), Err(Error::LineOutOfRange));
}

#[test]
fn unicode_scalar_columns_map_to_utf8_bytes_not_graphemes_or_cells() {
    let text = "é✨世界\ne\u{301}👩\u{200d}💻\nאב";
    assert_eq!(offset(text, "1:2"), "é".len());
    assert_eq!(offset(text, "1:5"), "é✨世界".len());
    assert_eq!(offset(text, "2:3"), "é✨世界\ne\u{301}".len());
    assert_eq!(offset(text, "2:5"), "é✨世界\ne\u{301}👩\u{200d}".len());
    assert_eq!(
        offset(text, "3:2"),
        "é✨世界\ne\u{301}👩\u{200d}💻\nא".len()
    );
}

#[test]
fn tabs_count_as_one_scalar_not_a_tab_stop() {
    assert_eq!(offset("\tword", "1:2"), 1);
    assert_eq!(offset("\tword", "1:6"), 5);
}

#[test]
fn end_of_line_is_allowed_but_addresses_are_never_clamped() {
    assert_eq!(offset("abc\nx", "1:4"), 3);
    assert_eq!(resolve("abc\nx", "1:5"), Err(Error::ColumnOutOfRange));
    assert_eq!(resolve("abc\nx", "2:3"), Err(Error::ColumnOutOfRange));
    assert_eq!(resolve("abc\nx", "3"), Err(Error::LineOutOfRange));
    assert_eq!(resolve("", "1:2"), Err(Error::ColumnOutOfRange));
}

#[test]
fn zeros_signs_non_ascii_digits_and_malformed_separators_are_rejected() {
    for query in [
        "", " ", "0", "0:1", "1:0", "+1", "-1", "1:-2", "1:+2", "1:", ":1", "1:2:3", "1.2", "1 2",
        "1 :2", "1: 2", "１", "١", "1\n2", "1:2x",
    ] {
        assert_eq!(
            resolve("abc", query),
            Err(Error::InvalidAddress),
            "{query:?}"
        );
    }
}

#[test]
fn overflow_and_long_input_do_not_panic_or_scan_the_document() {
    assert_eq!(resolve("abc", &"9".repeat(64)), Err(Error::InvalidAddress));
    assert_eq!(resolve("abc", &"1".repeat(65)), Err(Error::TooLong));
    assert_eq!(
        resolve("abc", &format!("1:{}", usize::MAX)),
        Err(Error::ColumnOutOfRange)
    );
    assert_eq!(
        resolve("abc", &usize::MAX.to_string()),
        Err(Error::LineOutOfRange)
    );
}

#[test]
fn every_scalar_boundary_round_trips_including_empty_final_lines() {
    for text in [
        "",
        "é✨\n",
        "\n\n",
        "a\n\t👩\u{200d}💻\ne\u{301}",
        "אב\n世界",
    ] {
        for byte in (0..=text.len()).filter(|byte| text.is_char_boundary(*byte)) {
            let prefix = &text[..byte];
            let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
            assert_eq!(offset(text, &format!("{line}:{column}")), byte, "{text:?}");
        }
    }
}

#[test]
fn panel_lifecycle_clears_errors_and_prefills_the_current_location() {
    let mut panel = GoTo::default();
    assert!(!panel.open);
    panel.show(2, 3);
    assert!(panel.open);
    assert_eq!(panel.query, "2:3");
    panel.error = Some(Error::InvalidAddress);
    panel.change_query("1".into());
    assert!(panel.error.is_none());
    panel.error = Some(Error::LineOutOfRange);
    panel.close();
    assert!(!panel.open);
    assert!(panel.error.is_none());
    panel.show(7, 1);
    assert_eq!(panel.query, "7:1");
}
