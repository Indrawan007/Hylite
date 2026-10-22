//! Final safety check for Enter candidates, independent of syntax coloring.
//! The lightweight planner is deliberately conservative, but its fence scanner
//! is not a container parser. Use the existing Markdown parser before commit.
use pulldown_cmark::{Event, Options, Parser, Tag};

pub(super) fn contains_code(text: &str, caret: usize) -> bool {
    Parser::new_ext(text, Options::all())
        .into_offset_iter()
        .any(|(event, range)| {
            matches!(event, Event::Start(Tag::CodeBlock(_)))
                && range.start <= caret
                // Include EOF of an unclosed block. A candidate on the next
                // list line necessarily has its caret beyond this endpoint.
                && caret <= range.end
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_ranges_include_unclosed_eof_and_nested_container_payloads() {
        for text in [
            "```\n- literal",
            "~~~rust\n1. literal",
            "- parent\n  ```\n  - literal",
            "> ```\n> - literal",
            "- parent\n\n      code\n  - literal",
            // A dedented fence ends the nested container and opens a new
            // root fence; a lexical scanner can mistake it for a closer.
            "- parent\n  ```\n  code\n```\n- literal",
            // Likewise, an indented-code false opener must not hide a real
            // root fence, then mistake its indented payload for a closer.
            "    ```\n\n```\n    ```\n- literal",
        ] {
            // The indented-code case deliberately asks about its code body,
            // not the following list item (which lies outside that block).
            let caret = if text.contains("      code") {
                text.find("code").unwrap() + 4
            } else {
                text.len()
            };
            assert!(contains_code(text, caret), "{text:?}");
        }
    }

    #[test]
    fn actual_closed_blocks_do_not_block_a_following_list() {
        for text in [
            "```\ncode\n```\n- item",
            "   ~~~\ncode\n~~~\n- item",
            "- parent\n  ```\n  code\n  ```\n- item",
            "    code\n\n- item",
            "- item",
        ] {
            assert!(!contains_code(text, text.len()), "{text:?}");
        }
    }
}
