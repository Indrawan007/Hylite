//! Source-authenticated projection for visual Preview editing.
//!
//! A draft edits rendered text and style spans, then serializes one authenticated
//! text block as a single atomic Source transaction. Markdown syntax may be
//! canonicalized; exact per-run Source ranges are exposed only where parsing
//! proves a linear visible-text mapping.
use crate::document::preview_model::{Block, Kind, Style};
use crate::document::Snapshot;
use std::ops::Range;

pub const MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading(u8),
    TableCell { row: usize, column: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineFormat {
    Bold,
    Italic,
    Strike,
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub snapshot: Snapshot,
    pub block: Range<usize>,
    /// Source bytes replaced on Apply. Paragraphs replace only the payload;
    /// headings replace the entire ATX line to keep its marker canonical.
    pub replace: Range<usize>,
    pub kind: BlockKind,
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub visible: Range<usize>,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSegment {
    pub visible: Range<usize>,
    pub source: Range<usize>,
    /// True only when the bytes decode to this run without entities, escapes,
    /// soft-break normalization, or other source/display transformations.
    pub exact: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAffinity {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub visible: String,
    pub spans: Vec<Span>,
    pub source_segments: Vec<SourceSegment>,
}

impl Target {
    /// Make a target from parser-authenticated paragraphs, root ATX headings, or
    /// single-physical-line paragraph leaves inside quote/list containers.
    pub fn for_block(block: &Block, source: &str, snapshot: Snapshot) -> Option<Self> {
        let (kind, replace, prefix, suffix) = match &block.kind {
            Kind::Paragraph => paragraph_target(block, source)?,
            Kind::Heading(level) => {
                let level = *level;
                let marker = block.heading_marker.as_ref()?;
                if block.setext_marker.is_some() || !(1..=6).contains(&level) {
                    return None;
                }
                let before_marker = source.get(..marker.start)?;
                let line_start = before_marker.rfind('\n').map_or(0, |offset| offset + 1);
                let line_end = source[line_start..]
                    .find('\n')
                    .map_or(source.len(), |offset| line_start + offset);
                if block.source.end != line_end && block.source.end != line_end.saturating_add(1) {
                    return None;
                }
                let content_end = if line_end > line_start
                    && source.as_bytes().get(line_end - 1) == Some(&b'\r')
                {
                    line_end - 1
                } else {
                    line_end
                };
                let indent = source.get(line_start..marker.start)?;
                let hashes = source.get(marker.clone())?;
                if indent.len() > 3
                    || !indent.bytes().all(|byte| byte == b' ')
                    || hashes != "#".repeat(usize::from(level))
                    || marker.end > content_end
                {
                    return None;
                }
                let (replace_end, line_ending) = if line_end < source.len() {
                    (line_end + 1, source.get(content_end..line_end + 1)?)
                } else {
                    (line_end, "")
                };
                (
                    BlockKind::Heading(level),
                    line_start..replace_end,
                    format!("{indent}{hashes} "),
                    line_ending.to_owned(),
                )
            }
            _ => return None,
        };
        Some(Self {
            snapshot,
            block: block.source.clone(),
            replace,
            kind,
            prefix,
            suffix,
        })
    }

    /// Authenticate a non-empty cell in a canonical root table. Table cell
    /// offsets are exact raw payload ranges, not character-level projections.
    pub fn for_table_cell(
        block: &Block,
        source: &str,
        snapshot: Snapshot,
        row: usize,
        column: usize,
    ) -> Option<Self> {
        let Kind::Table(table) = &block.kind else {
            return None;
        };
        let payload = table
            .editable_cells
            .as_ref()?
            .get(row)?
            .get(column)?
            .clone();
        if payload.is_empty()
            || !crate::text_edit::valid_range(source, &payload)
            || Projection::from_target_cell(block, source, row, column).is_none()
        {
            return None;
        }
        Some(Self {
            snapshot,
            block: block.source.clone(),
            replace: payload,
            kind: BlockKind::TableCell { row, column },
            prefix: String::new(),
            suffix: String::new(),
        })
    }

    pub fn replacement(&self, visible: &str, spans: &[Span]) -> Option<String> {
        Some(format!(
            "{}{}{}",
            self.prefix,
            serialize(visible, spans)?,
            self.suffix
        ))
    }
}

fn paragraph_target(
    block: &Block,
    source: &str,
) -> Option<(BlockKind, Range<usize>, String, String)> {
    if let Some(payload) = block.paragraph_payload.clone() {
        if block.source.start != payload.start
            || payload.end.checked_add(1) != Some(block.source.end)
            || source.get(payload.clone())?.contains('\n')
            || source.get(payload.clone())?.contains('\r')
        {
            return None;
        }
        return Some((BlockKind::Paragraph, payload, String::new(), String::new()));
    }

    // Nested paragraph event ranges vary with container syntax. Accept one only
    // when its exact raw slice independently parses to the same one-line
    // paragraph projection; this keeps list/quote prefixes outside the edit.
    if block.source.is_empty() || block.source.len() > MAX_BYTES {
        return None;
    }
    let raw = source.get(block.source.clone())?;
    let (payload_len, line_ending) = if raw.ends_with("\r\n") {
        (raw.len().checked_sub(2)?, "\r\n")
    } else if raw.ends_with('\n') {
        (raw.len().checked_sub(1)?, "\n")
    } else {
        (raw.len(), "")
    };
    let line = raw.get(..payload_len)?;
    let content_start = line.len()
        - line
            .trim_start_matches(|character| matches!(character, ' ' | '\t'))
            .len();
    let content = line.get(content_start..)?;
    if content.is_empty() || content.contains('\n') || content.contains('\r') {
        return None;
    }
    let parsed = crate::document::preview_model::parse(content);
    let [standalone] = parsed.as_slice() else {
        return None;
    };
    if standalone.kind != Kind::Paragraph
        || standalone.source.start != 0
        || standalone.source.end != content.len()
    {
        return None;
    }
    let original_projection = Projection::from_block(block, source)?;
    let standalone_projection = Projection::from_block(standalone, content)?;
    if !original_projection.same_presentation(&standalone_projection) {
        return None;
    }
    let replace_start = block.source.start.checked_add(content_start)?;
    Some((
        BlockKind::Paragraph,
        replace_start..block.source.end,
        String::new(),
        line_ending.to_owned(),
    ))
}

impl Projection {
    /// Flatten ordinary inline runs and retain exact source segments where a
    /// run is byte-for-byte identical to the visible text. Other runs stay
    /// editable through block serialization but have no glyph-to-Source map.
    pub fn from_block(block: &Block, source: &str) -> Option<Self> {
        Self::from_runs(&block.runs, source)
    }

    pub fn for_target(block: &Block, source: &str, target: &Target) -> Option<Self> {
        match target.kind {
            BlockKind::TableCell { row, column } => {
                Self::from_target_cell(block, source, row, column)
            }
            BlockKind::Paragraph | BlockKind::Heading(_) => Self::from_block(block, source),
        }
    }

    fn from_target_cell(block: &Block, source: &str, row: usize, column: usize) -> Option<Self> {
        let Kind::Table(table) = &block.kind else {
            return None;
        };
        let cell = table.rows.get(row)?.cells.get(column)?;
        let payload = table.editable_cells.as_ref()?.get(row)?.get(column)?;
        (cell.source.start <= payload.start && payload.end <= cell.source.end)
            .then(|| Self::from_runs(&cell.runs, source))?
    }

    pub fn from_runs(runs: &[crate::document::preview_model::Run], source: &str) -> Option<Self> {
        let mut visible = String::new();
        let mut spans: Vec<Span> = Vec::with_capacity(runs.len());
        let mut source_segments: Vec<SourceSegment> = Vec::with_capacity(runs.len());
        for run in runs {
            if run.math.is_some() || run.image.is_some() {
                return None;
            }
            if run.text.chars().any(char::is_control) || run.text.contains('\n') {
                return None;
            }
            if run.text.is_empty() {
                continue;
            }
            let start = visible.len();
            visible.push_str(&run.text);
            let end = visible.len();
            let raw = source.get(run.source.clone())?;
            source_segments.push(SourceSegment {
                visible: start..end,
                source: run.source.clone(),
                exact: raw == run.text,
            });
            if let Some(previous) = spans.last_mut() {
                if previous.visible.end == start && previous.style == run.style {
                    previous.visible.end = end;
                    continue;
                }
            }
            spans.push(Span {
                visible: start..end,
                style: run.style.clone(),
            });
        }
        if visible.is_empty() || visible.len() > MAX_BYTES || !valid_spans(&visible, &spans) {
            return None;
        }
        Some(Self {
            visible,
            spans,
            source_segments,
        })
    }

    pub fn same_presentation(&self, other: &Self) -> bool {
        self.visible == other.visible && self.spans == other.spans
    }

    /// Map an unambiguous visible selection only when every selected byte is
    /// backed by contiguous, exact Source text in one or more parser runs.
    pub fn source_range_for_visible(&self, visible: Range<usize>) -> Option<Range<usize>> {
        if visible.is_empty()
            || visible.end > self.visible.len()
            || !self.visible.is_char_boundary(visible.start)
            || !self.visible.is_char_boundary(visible.end)
        {
            return None;
        }
        let mut next_visible = visible.start;
        let mut source_start = None;
        let mut source_end = None;
        for segment in self.source_segments.iter().filter(|segment| {
            segment.visible.start < visible.end && visible.start < segment.visible.end
        }) {
            if !segment.exact {
                return None;
            }
            let start = segment.visible.start.max(visible.start);
            let end = segment.visible.end.min(visible.end);
            if start != next_visible {
                return None;
            }
            let mapped_start = segment
                .source
                .start
                .checked_add(start - segment.visible.start)?;
            let mapped_end = segment
                .source
                .start
                .checked_add(end - segment.visible.start)?;
            if source_end.is_some_and(|previous| previous != mapped_start) {
                return None;
            }
            source_start.get_or_insert(mapped_start);
            source_end = Some(mapped_end);
            next_visible = end;
        }
        (next_visible == visible.end)
            .then(|| source_start.zip(source_end).map(|(start, end)| start..end))
            .flatten()
    }

    /// Map a visible caret boundary with explicit affinity when hidden Markdown
    /// delimiters create distinct source offsets on either side of the glyph.
    pub fn source_offset_for_visible(
        &self,
        visible: usize,
        affinity: SourceAffinity,
    ) -> Option<usize> {
        if visible > self.visible.len() || !self.visible.is_char_boundary(visible) {
            return None;
        }
        let interior = self
            .source_segments
            .iter()
            .find(|segment| segment.visible.start < visible && visible < segment.visible.end);
        if let Some(segment) = interior {
            return segment
                .exact
                .then(|| {
                    segment
                        .source
                        .start
                        .checked_add(visible - segment.visible.start)
                })
                .flatten();
        }
        let before = self
            .source_segments
            .iter()
            .find(|segment| segment.visible.end == visible);
        let after = self
            .source_segments
            .iter()
            .find(|segment| segment.visible.start == visible);
        let segment = match (before, after, affinity) {
            (None, Some(after), _) if visible == 0 => after,
            (Some(before), None, _) if visible == self.visible.len() => before,
            (Some(before), _, SourceAffinity::Before) => before,
            (_, Some(after), SourceAffinity::After) => after,
            _ => return None,
        };
        if !segment.exact {
            return None;
        }
        if segment.visible.end == visible {
            Some(segment.source.end)
        } else {
            Some(segment.source.start)
        }
    }
}

/// Carry the style at the edit boundary through a single contiguous text edit.
/// Prefix and suffix formatting are preserved, including Unicode boundaries.
pub fn restyle(before: &str, after: &str, spans: &[Span]) -> Option<Vec<Span>> {
    if !valid_spans(before, spans)
        || after.is_empty()
        || after.len() > MAX_BYTES
        || after.chars().any(char::is_control)
        || after.contains('\n')
    {
        return None;
    }
    if before == after {
        return Some(spans.to_vec());
    }

    let mut prefix = 0;
    for (left, right) in before.chars().zip(after.chars()) {
        if left != right {
            break;
        }
        prefix += left.len_utf8();
    }
    let before_tail = before.get(prefix..)?;
    let after_tail = after.get(prefix..)?;
    let mut suffix = 0;
    for (left, right) in before_tail.chars().rev().zip(after_tail.chars().rev()) {
        if left != right {
            break;
        }
        suffix += left.len_utf8();
    }
    let old_suffix_start = before.len().checked_sub(suffix)?;
    let new_suffix_start = after.len().checked_sub(suffix)?;
    let inserted = new_suffix_start.checked_sub(prefix)?;
    let inherited = (inserted > 0)
        .then(|| style_at(spans, prefix, before.len()))
        .flatten();

    let mut output = Vec::with_capacity(spans.len() + 2);
    for span in spans {
        let start = span.visible.start;
        let end = span.visible.end.min(prefix);
        if start < end {
            output.push(Span {
                visible: start..end,
                style: span.style.clone(),
            });
        }
    }
    if let Some(style) = inherited {
        output.push(Span {
            visible: prefix..new_suffix_start,
            style,
        });
    }
    for span in spans {
        let start = span.visible.start.max(old_suffix_start);
        let end = span.visible.end;
        if start < end {
            output.push(Span {
                visible: new_suffix_start + (start - old_suffix_start)
                    ..new_suffix_start + (end - old_suffix_start),
                style: span.style.clone(),
            });
        }
    }
    coalesce(&mut output);
    valid_spans(after, &output).then_some(output)
}

/// Find the current parser-owned block for a visual target, including leaves
/// nested in quotes and list items. The source range and snapshot must match.
pub fn find_block<'a>(
    blocks: &'a [Block],
    source: &str,
    snapshot: Snapshot,
    target: &Target,
) -> Option<&'a Block> {
    for block in blocks {
        let current = match target.kind {
            BlockKind::TableCell { row, column } => {
                Target::for_table_cell(block, source, snapshot, row, column)
            }
            BlockKind::Paragraph | BlockKind::Heading(_) => {
                Target::for_block(block, source, snapshot)
            }
        };
        if current.as_ref() == Some(target) {
            return Some(block);
        }
        if let Some(found) = find_block(&block.children, source, snapshot, target) {
            return Some(found);
        }
    }
    None
}

/// Locate a reparsed block intersecting the candidate source replacement while
/// preserving its parser-authenticated kind. Nested containers are traversed.
pub fn find_updated_block<'a>(
    blocks: &'a [Block],
    replacement: &Range<usize>,
    kind: BlockKind,
) -> Option<&'a Block> {
    for block in blocks {
        let kind_matches = match kind {
            BlockKind::Paragraph => block.kind == Kind::Paragraph,
            BlockKind::Heading(level) => block.kind == Kind::Heading(level),
            BlockKind::TableCell { .. } => matches!(block.kind, Kind::Table(_)),
        };
        if kind_matches
            && block.source.start < replacement.end
            && replacement.start < block.source.end
        {
            return Some(block);
        }
        if let Some(found) = find_updated_block(&block.children, replacement, kind) {
            return Some(found);
        }
    }
    None
}

pub fn valid_spans(visible: &str, spans: &[Span]) -> bool {
    if visible.is_empty() || spans.is_empty() || visible.len() > MAX_BYTES {
        return false;
    }
    let mut next = 0;
    for span in spans {
        if span.visible.start != next
            || span.visible.start >= span.visible.end
            || span.visible.end > visible.len()
            || !visible.is_char_boundary(span.visible.start)
            || !visible.is_char_boundary(span.visible.end)
        {
            return false;
        }
        next = span.visible.end;
    }
    next == visible.len()
}

pub fn serialize(visible: &str, spans: &[Span]) -> Option<String> {
    if !valid_spans(visible, spans)
        || visible.chars().any(char::is_control)
        || visible.contains('\n')
    {
        return None;
    }
    let mut output = String::with_capacity(visible.len());
    let mut open = Vec::new();
    for span in spans {
        let text = visible.get(span.visible.clone())?;
        let next = markers(&span.style);
        let shared = open
            .iter()
            .zip(&next)
            .take_while(|(left, right)| left == right)
            .count();
        for marker in open[shared..].iter().rev() {
            close_marker(&mut output, marker)?;
        }
        for marker in &next[shared..] {
            open_marker(&mut output, marker);
        }
        open = next;
        if span.style.code {
            output.push_str(&code_span(text)?);
        } else {
            output.push_str(&escape_inline(text));
        }
        if output.len() > MAX_BYTES {
            return None;
        }
    }
    for marker in open.iter().rev() {
        close_marker(&mut output, marker)?;
    }
    (output.len() <= MAX_BYTES).then_some(output)
}

fn style_at(spans: &[Span], offset: usize, len: usize) -> Option<Style> {
    spans
        .iter()
        .find(|span| span.visible.start <= offset && offset < span.visible.end)
        .or_else(|| (offset == len).then(|| spans.last()).flatten())
        .or_else(|| spans.iter().rev().find(|span| span.visible.start < offset))
        .map(|span| span.style.clone())
}

pub(crate) fn coalesce(spans: &mut Vec<Span>) {
    let mut merged: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(previous) = merged.last_mut() {
            if previous.visible.end == span.visible.start && previous.style == span.style {
                previous.visible.end = span.visible.end;
                continue;
            }
        }
        merged.push(span);
    }
    *spans = merged;
}

#[derive(Clone, PartialEq, Eq)]
enum Marker<'a> {
    Link(&'a str),
    Strong,
    Emphasis,
    Strike,
}

fn markers(style: &Style) -> Vec<Marker<'_>> {
    let mut output = Vec::with_capacity(4);
    if let Some(destination) = style.link.as_deref() {
        output.push(Marker::Link(destination));
    }
    if style.strong {
        output.push(Marker::Strong);
    }
    if style.emphasis {
        output.push(Marker::Emphasis);
    }
    if style.strike {
        output.push(Marker::Strike);
    }
    output
}

fn open_marker(output: &mut String, marker: &Marker<'_>) {
    match marker {
        Marker::Link(_) => output.push('['),
        Marker::Strong => output.push_str("**"),
        Marker::Emphasis => output.push('*'),
        Marker::Strike => output.push_str("~~"),
    }
}

fn close_marker(output: &mut String, marker: &Marker<'_>) -> Option<()> {
    match marker {
        Marker::Link(destination) => {
            output.push_str("](");
            output.push_str(&encode_destination(destination)?);
            output.push(')');
        }
        Marker::Strong => output.push_str("**"),
        Marker::Emphasis => output.push('*'),
        Marker::Strike => output.push_str("~~"),
    }
    Some(())
}

fn encode_destination(destination: &str) -> Option<String> {
    if destination.is_empty()
        || destination.chars().any(|character| {
            character.is_control() || character.is_whitespace() || matches!(character, '<' | '>')
        })
    {
        return None;
    }
    let mut encoded = String::with_capacity(destination.len());
    for character in destination.chars() {
        if matches!(character, '(' | ')' | '\\') {
            encoded.push('\\');
        }
        encoded.push(character);
    }
    Some(encoded)
}

fn code_span(text: &str) -> Option<String> {
    if text.is_empty() || text.contains('\n') || text.contains('\r') {
        return None;
    }
    let mut longest = 0usize;
    let mut current = 0usize;
    for character in text.chars() {
        if character == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    let marker = "`".repeat(longest + 1);
    let padding = text.starts_with(' ')
        && text.ends_with(' ')
        && text.chars().any(|character| character != ' ');
    if padding {
        Some(format!("{marker} {text} {marker}"))
    } else {
        Some(format!("{marker}{text}{marker}"))
    }
}

fn escape_inline(text: &str) -> String {
    let leading = text.len() - text.trim_start_matches(' ').len();
    let trailing_start = text.trim_end_matches(' ').len();
    let mut output = String::with_capacity(text.len());
    for (offset, character) in text.char_indices() {
        if character == ' ' && (offset < leading || offset >= trailing_start) {
            output.push_str("&#32;");
        } else {
            match character {
                '&' => output.push_str("&amp;"),
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                punctuation if punctuation.is_ascii_punctuation() => {
                    output.push('\\');
                    output.push(punctuation);
                }
                other => output.push(other),
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn span(style: Style, text: &str, start: usize) -> Span {
        Span {
            visible: start..start + text.len(),
            style,
        }
    }

    fn style(strong: bool, emphasis: bool, code: bool, link: Option<&str>) -> Style {
        Style {
            strong,
            emphasis,
            code,
            link: link.map(Arc::<str>::from),
            ..Style::default()
        }
    }

    #[test]
    fn serialize_round_trips_basic_styles_unicode_links_strike_code_and_punctuation() {
        let visible = "plain café bold * [`code`] link gone";
        let mut spans = Vec::new();
        let mut offset = 0;
        for (text, style) in [
            ("plain café ", Style::default()),
            ("bold", style(true, false, false, None)),
            (" * ", Style::default()),
            ("[`code`]", style(false, false, true, None)),
            (
                " link",
                style(false, false, false, Some("https://example.invalid/a(b)")),
            ),
            (
                " gone",
                Style {
                    strike: true,
                    ..Style::default()
                },
            ),
        ] {
            spans.push(span(style, text, offset));
            offset += text.len();
        }
        assert_eq!(offset, visible.len());
        let encoded = serialize(visible, &spans).unwrap();
        let blocks = crate::document::preview_model::parse(&encoded);
        assert_eq!(
            blocks[0]
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            visible
        );
        assert!(blocks[0].runs.iter().any(|run| run.style.strong));
        assert!(blocks[0].runs.iter().any(|run| run.style.code));
        assert!(blocks[0].runs.iter().any(|run| run.style.strike));
        assert!(blocks[0].runs.iter().any(|run| run.style.link.is_some()));
        let parsed = Projection::from_block(&blocks[0], &encoded).unwrap();
        assert!(parsed.same_presentation(&Projection {
            visible: visible.to_owned(),
            spans,
            source_segments: Vec::new(),
        }));
    }

    #[test]
    fn serialize_reconciles_nested_style_boundaries_without_losing_style() {
        let visible = "outer inner end";
        let strong = Style {
            strong: true,
            ..Style::default()
        };
        let nested = Style {
            strong: true,
            emphasis: true,
            ..Style::default()
        };
        let spans = vec![
            span(strong.clone(), "outer ", 0),
            span(nested, "inner", 6),
            span(strong, " end", 11),
        ];
        let encoded = serialize(visible, &spans).unwrap();
        let source = format!("{encoded}\n");
        let blocks = crate::document::preview_model::parse(&source);
        let parsed = Projection::from_block(&blocks[0], &source).unwrap();
        assert!(parsed.same_presentation(&Projection {
            visible: visible.to_owned(),
            spans,
            source_segments: Vec::new(),
        }));
    }

    #[test]
    fn serialize_preserves_leading_trailing_spaces_and_never_creates_blocks() {
        let visible = " *literal*  ";
        let spans = vec![span(Style::default(), visible, 0)];
        let encoded = serialize(visible, &spans).unwrap();
        let blocks = crate::document::preview_model::parse(&format!("{encoded}\n"));
        assert_eq!(
            blocks[0]
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            visible
        );
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn restyle_keeps_unicode_prefix_suffix_and_inherits_inserted_style() {
        let before = "é bold end";
        let strong = Style {
            strong: true,
            ..Style::default()
        };
        let spans = vec![
            span(Style::default(), "é ", 0),
            span(strong.clone(), "bold", 3),
            span(Style::default(), " end", 7),
        ];
        let after = "é brXold end";
        let next = restyle(before, after, &spans).unwrap();
        assert!(next
            .iter()
            .any(|span| span.style.strong && span.visible.start <= 4 && span.visible.end >= 5));
        assert!(valid_spans(after, &next));
        let deletion = restyle(after, "é bold end", &next).unwrap();
        assert!(valid_spans("é bold end", &deletion));
    }

    #[test]
    fn source_map_is_exact_inside_runs_and_explicitly_affine_at_hidden_markers() {
        let source = "plain **café🙂** tail &amp;";
        let blocks = crate::document::preview_model::parse(source);
        let projection = Projection::from_block(&blocks[0], source).unwrap();
        let styled = projection.visible.find("café🙂").unwrap();
        let styled_len = "café🙂".len();
        let raw_styled = source.find("café🙂").unwrap();
        assert_eq!(
            projection.source_range_for_visible(styled..styled + styled_len),
            Some(raw_styled..raw_styled + styled_len)
        );
        assert_eq!(
            projection.source_offset_for_visible(styled, SourceAffinity::Before),
            Some(source.find("**").unwrap())
        );
        assert_eq!(
            projection.source_offset_for_visible(styled, SourceAffinity::After),
            Some(raw_styled)
        );
        assert!(projection
            .source_range_for_visible(0..projection.visible.len())
            .is_none());
        assert!(projection
            .source_segments
            .iter()
            .any(|segment| !segment.exact));
    }

    #[test]
    fn target_authenticates_root_blocks_and_single_line_paragraph_leaves() {
        let source = "plain **bold**\n\n## title *here*\n\n> nested\n\n- list **item**\n";
        let blocks = crate::document::preview_model::parse(source);
        assert!(Target::for_block(&blocks[0], source, Snapshot::default()).is_some());
        assert!(Target::for_block(&blocks[1], source, Snapshot::default()).is_some());
        let quote_child = &blocks[2].children[0];
        assert!(Target::for_block(quote_child, source, Snapshot::default()).is_some());
        let list_child = &blocks[3].children[0].children[0];
        assert!(Target::for_block(list_child, source, Snapshot::default()).is_some());
        let setext = "title\n---\n";
        let block = &crate::document::preview_model::parse(setext)[0];
        assert!(Target::for_block(block, setext, Snapshot::default()).is_none());
    }
}
