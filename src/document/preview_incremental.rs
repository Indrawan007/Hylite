//! Conservative block-local reparsing for isolated, unstyled root paragraphs.
//!
//! Markdown block parsing has cross-line and document-wide context (lists,
//! fences, Setext headings, reference definitions, and more). This module only
//! reuses a cache when an exact Source diff is contained in one already-proven
//! single-line plain paragraph, both surrounding boundaries are isolated, and
//! reparsing that line still produces exactly one plain paragraph. Everything
//! else returns `None` so the caller uses the canonical full parser.
use super::preview_model::{self, Block, Kind, Style};
use std::ops::Range;

/// Reparse one safe paragraph and shift every absolute Source range after it.
/// A `None` result is deliberately indistinguishable from an unsafe edit: the
/// caller must run `preview_model::parse` over the complete current Source.
pub(crate) fn reparse_plain_paragraph(
    previous_source: &str,
    previous_blocks: &[Block],
    source: &str,
) -> Option<Vec<Block>> {
    let (old_change, new_change) = single_diff(previous_source, source)?;
    let (index, previous) = previous_blocks.iter().enumerate().find(|(_, block)| {
        is_plain_paragraph(block, previous_source)
            && block
                .paragraph_payload
                .as_ref()
                .is_some_and(|payload| encloses(payload, &old_change))
            && isolated_boundaries(previous_source, &block.source)
    })?;
    let payload = previous.paragraph_payload.as_ref()?;
    if !encloses(payload, &old_change) {
        return None;
    }

    let old_region = previous.source.clone();
    let old_region_text = previous_source.get(old_region.clone())?;
    if old_region_text.is_empty() {
        return None;
    }
    let removed = old_change.end.checked_sub(old_change.start)?;
    let inserted = new_change.end.checked_sub(new_change.start)?;
    let new_region_len = old_region
        .end
        .checked_sub(old_region.start)?
        .checked_sub(removed)?
        .checked_add(inserted)?;
    let new_region = old_region.start..old_region.start.checked_add(new_region_len)?;
    let new_region_text = source.get(new_region.clone())?;
    let new_payload = new_region.start..new_region.end.checked_sub(1)?;
    if !encloses(&new_payload, &new_change)
        || !isolated_boundaries(source, &new_region)
        || !is_single_line_region(new_region_text)
    {
        return None;
    }

    let mut reparsed = preview_model::parse(new_region_text);
    if reparsed.len() != 1 || !is_plain_paragraph(&reparsed[0], new_region_text) {
        return None;
    }
    offset_block(&mut reparsed[0], new_region.start)?;

    if previous_blocks[..index]
        .iter()
        .any(|block| block.source.end > old_region.start)
        || previous_blocks[index + 1..]
            .iter()
            .any(|block| block.source.start < old_region.end)
    {
        return None;
    }

    let mut blocks = Vec::with_capacity(previous_blocks.len());
    blocks.extend(previous_blocks[..index].iter().cloned());
    blocks.push(reparsed.pop()?);
    for previous in &previous_blocks[index + 1..] {
        let mut block = previous.clone();
        shift_block_after(
            &mut block,
            old_region.end,
            old_region.len(),
            new_region.len(),
        )?;
        blocks.push(block);
    }
    Some(blocks)
}

fn single_diff(previous: &str, current: &str) -> Option<(Range<usize>, Range<usize>)> {
    if previous == current {
        return None;
    }
    let mut prefix = previous
        .as_bytes()
        .iter()
        .zip(current.as_bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while prefix > 0 && (!previous.is_char_boundary(prefix) || !current.is_char_boundary(prefix)) {
        prefix -= 1;
    }

    let mut suffix = 0usize;
    while suffix < previous.len() - prefix
        && suffix < current.len() - prefix
        && previous.as_bytes()[previous.len() - suffix - 1]
            == current.as_bytes()[current.len() - suffix - 1]
    {
        suffix += 1;
    }
    while suffix > 0
        && (!previous.is_char_boundary(previous.len() - suffix)
            || !current.is_char_boundary(current.len() - suffix))
    {
        suffix -= 1;
    }

    Some((
        prefix..previous.len().checked_sub(suffix)?,
        prefix..current.len().checked_sub(suffix)?,
    ))
}

fn encloses(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn isolated_boundaries(source: &str, region: &Range<usize>) -> bool {
    let Some(before) = source.get(..region.start) else {
        return false;
    };
    let Some(after) = source.get(region.end..) else {
        return false;
    };
    (region.start == 0 || before.ends_with("\n\n"))
        && (region.end == source.len() || after.starts_with('\n'))
}

fn is_single_line_region(source: &str) -> bool {
    source
        .strip_suffix('\n')
        .is_some_and(|payload| !payload.contains('\n'))
}

fn is_plain_paragraph(block: &Block, source: &str) -> bool {
    if !matches!(&block.kind, Kind::Paragraph)
        || !block.children.is_empty()
        || block.heading_marker.is_some()
        || block.setext_marker.is_some()
        || block.multiline_paragraph_payload.is_some()
    {
        return false;
    }
    let Some(payload) = block.paragraph_payload.as_ref() else {
        return false;
    };
    let Some(payload_text) = source.get(payload.clone()) else {
        return false;
    };
    let Some(region_text) = source.get(block.source.clone()) else {
        return false;
    };
    // Reference links and footnotes can be affected by definitions elsewhere
    // in the document even when an isolated parse looks like literal text.
    if payload_text.contains('\n')
        || payload_text.contains('[')
        || payload_text.contains(']')
        || block.source.start != payload.start
        || payload.end.checked_add(1) != Some(block.source.end)
        || region_text.strip_suffix('\n') != Some(payload_text)
    {
        return false;
    }
    let [run] = block.runs.as_slice() else {
        return false;
    };
    &run.source == payload
        && run.text == payload_text
        && run.style == Style::default()
        && run.math.is_none()
        && run.image.is_none()
}

fn offset(value: usize, amount: usize) -> Option<usize> {
    value.checked_add(amount)
}

fn shift_after(value: usize, boundary: usize, old_len: usize, new_len: usize) -> Option<usize> {
    if value < boundary {
        return None;
    }
    if new_len >= old_len {
        value.checked_add(new_len - old_len)
    } else {
        value.checked_sub(old_len - new_len)
    }
}

fn remap_range(range: &mut Range<usize>, map: &impl Fn(usize) -> Option<usize>) -> Option<()> {
    range.start = map(range.start)?;
    range.end = map(range.end)?;
    Some(())
}

fn remap_block(block: &mut Block, map: &impl Fn(usize) -> Option<usize>) -> Option<()> {
    remap_range(&mut block.source, map)?;
    for run in &mut block.runs {
        remap_range(&mut run.source, map)?;
    }
    for child in &mut block.children {
        remap_block(child, map)?;
    }
    if let Some(range) = &mut block.heading_marker {
        remap_range(range, map)?;
    }
    if let Some(range) = &mut block.setext_marker {
        remap_range(range, map)?;
    }
    if let Some(range) = &mut block.paragraph_payload {
        remap_range(range, map)?;
    }
    if let Some(range) = &mut block.multiline_paragraph_payload {
        remap_range(range, map)?;
    }
    if let Some(range) = &mut block.inline_text_payload {
        remap_range(range, map)?;
    }
    match &mut block.kind {
        Kind::Item(Some(task)) => {
            if let Some(range) = &mut task.source {
                remap_range(range, map)?;
            }
        }
        Kind::Table(table) => {
            for row in &mut table.rows {
                remap_range(&mut row.source, map)?;
                for cell in &mut row.cells {
                    remap_range(&mut cell.source, map)?;
                    for run in &mut cell.runs {
                        remap_range(&mut run.source, map)?;
                    }
                }
            }
            if let Some(ranges) = &mut table.delimiters {
                for range in ranges {
                    remap_range(range, map)?;
                }
            }
            if let Some(ranges) = &mut table.body_lines {
                for range in ranges {
                    remap_range(range, map)?;
                }
            }
            if let Some(rows) = &mut table.editable_cells {
                for row in rows {
                    for range in row {
                        remap_range(range, map)?;
                    }
                }
            }
        }
        _ => {}
    }
    Some(())
}

fn offset_block(block: &mut Block, amount: usize) -> Option<()> {
    remap_block(block, &|value| offset(value, amount))
}

fn shift_block_after(
    block: &mut Block,
    boundary: usize,
    old_len: usize,
    new_len: usize,
) -> Option<()> {
    remap_block(block, &|value| {
        shift_after(value, boundary, old_len, new_len)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains_task(blocks: &[Block]) -> bool {
        blocks.iter().any(|block| {
            matches!(&block.kind, Kind::Item(Some(task)) if task.source.is_some())
                || contains_task(&block.children)
        })
    }

    #[test]
    fn unicode_edit_reparses_only_an_isolated_plain_paragraph_and_matches_full_parse() {
        let previous_source = concat!(
            "# Title **one**\n\n",
            "Introductory paragraph.\n\n",
            "editable text\n\n",
            "> quoted **text**\n\n",
            "- [x] checked task\n",
            "  - nested task\n\n",
            "| Name | Count |\n",
            "| --- | ---: |\n",
            "| café🙂 | **3** |\n\n",
            "## Closing heading\n\n",
            "```rs\nlet x = 1;\n```\n\n",
            "$x^2$\n",
        );
        let previous = preview_model::parse(previous_source);
        let start = previous_source.find("editable text").unwrap();
        let old_end = start + "editable text".len();
        let replacement = "改訂済み🙂";
        let mut current_source = previous_source.to_owned();
        current_source.replace_range(start..old_end, replacement);

        let incremental =
            reparse_plain_paragraph(previous_source, &previous, &current_source).unwrap();
        assert_eq!(incremental, preview_model::parse(&current_source));
        assert!(incremental
            .iter()
            .any(|block| matches!(&block.kind, Kind::Quote)));
        assert!(incremental
            .iter()
            .any(|block| matches!(&block.kind, Kind::Table(_))));
        assert!(contains_task(&incremental));
        assert!(incremental
            .iter()
            .any(|block| matches!(&block.kind, Kind::Code { .. })));
    }

    #[test]
    fn document_wide_reference_context_forces_the_full_parse_fallback() {
        let previous_source = "plain text\n\n[ref]: /destination\n\n";
        let previous = preview_model::parse(previous_source);
        let mut current = previous_source.to_owned();
        current.replace_range(0.."plain text".len(), "[ref]");
        assert!(reparse_plain_paragraph(previous_source, &previous, &current).is_none());
    }

    #[test]
    fn unsafe_changes_use_the_full_parse_fallback() {
        let previous_source = "before\n\nplain text\n\nafter\n";
        let previous = preview_model::parse(previous_source);
        for (range, replacement) in [
            (8..8, "\n"),
            (8..18, "# heading"),
            (8..18, "**formatted**"),
            (18..19, " "),
        ] {
            let mut current = previous_source.to_owned();
            current.replace_range(range, replacement);
            assert!(reparse_plain_paragraph(previous_source, &previous, &current).is_none());
        }
    }

    #[test]
    fn exact_diff_handles_utf8_boundaries_and_disjoint_edits_conservatively() {
        for (previous, current) in [("é🙂", "ê🙂"), ("a🙂z", "a✨z"), ("abc", "aXc")] {
            let (old, new) = single_diff(previous, current).unwrap();
            assert!(previous.is_char_boundary(old.start) && previous.is_char_boundary(old.end));
            assert!(current.is_char_boundary(new.start) && current.is_char_boundary(new.end));
            let mut rebuilt = previous.to_owned();
            rebuilt.replace_range(old, &current[new]);
            assert_eq!(rebuilt, current);
        }

        let previous_source = "before\n\nplain text\n\nafter\n";
        let previous = preview_model::parse(previous_source);
        let mut current = previous_source.to_owned();
        current.replace_range(8..9, "P");
        let after = current.find("after").unwrap();
        current.replace_range(after..after + 1, "A");
        assert!(reparse_plain_paragraph(previous_source, &previous, &current).is_none());
    }
}
