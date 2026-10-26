//! Source-authenticated projection for the first production visual editor.
//!
//! A draft edits rendered text and style spans, then serializes one authenticated
//! root paragraph or ATX heading as a single atomic Source transaction. Markdown
//! syntax may be canonicalized; this is block editing, not a glyph-to-Source map.
use crate::document::preview_model::{Block, Kind, Style};
use crate::document::Snapshot;
use std::ops::Range;
use std::sync::Arc;

pub const MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading(u8),
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
pub struct Projection {
    pub visible: String,
    pub spans: Vec<Span>,
}

impl Target {
    /// Make a target only from parser-authenticated root paragraph/ATX metadata.
    pub fn for_block(block: &Block, source: &str, snapshot: Snapshot) -> Option<Self> {
        let (kind, replace, prefix, suffix) = match &block.kind {
            Kind::Paragraph => {
                let payload = block.paragraph_payload.clone()?;
                if block.source.start != payload.start
                    || payload.end.checked_add(1) != Some(block.source.end)
                    || source.get(payload.clone())?.contains('\n')
                    || source.get(payload.clone())?.contains('\r')
                {
                    return None;
                }
                (BlockKind::Paragraph, payload, String::new(), String::new())
            }
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

    pub fn replacement(&self, visible: &str, spans: &[Span]) -> Option<String> {
        Some(format!(
            "{}{}{}",
            self.prefix,
            serialize(visible, spans)?,
            self.suffix
        ))
    }
}

impl Projection {
    /// Flatten only ordinary inline runs. Images/math and multiline blocks stay
    /// on their existing dedicated Preview paths.
    pub fn from_block(block: &Block) -> Option<Self> {
        let mut visible = String::new();
        let mut spans: Vec<Span> = Vec::with_capacity(block.runs.len());
        for run in &block.runs {
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
        Some(Self { visible, spans })
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
        assert_eq!(
            Projection::from_block(&blocks[0]).unwrap(),
            Projection {
                visible: visible.to_owned(),
                spans,
            }
        );
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
        let blocks = crate::document::preview_model::parse(&format!("{encoded}\n"));
        assert_eq!(
            Projection::from_block(&blocks[0]).unwrap(),
            Projection {
                visible: visible.to_owned(),
                spans,
            }
        );
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
    fn target_only_authenticates_root_paragraphs_and_atx_headings() {
        let source = "plain **bold**\n\n## title *here*\n\n> nested\n";
        let blocks = crate::document::preview_model::parse(source);
        assert!(Target::for_block(&blocks[0], source, Snapshot::default()).is_some());
        assert!(Target::for_block(&blocks[1], source, Snapshot::default()).is_some());
        assert!(Target::for_block(&blocks[2], source, Snapshot::default()).is_none());
        let setext = "title\n---\n";
        let block = &crate::document::preview_model::parse(setext)[0];
        assert!(Target::for_block(block, setext, Snapshot::default()).is_none());
    }
}
