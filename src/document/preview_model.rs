//! Owned, read-only preview projection. Source ranges are provenance, NOT a
//! bidirectional caret map (entities, escapes and soft breaks change text).
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;
use std::sync::Arc;

#[path = "preview_table.rs"]
mod table;
pub use table::{Alignment, Cell, Table};

pub const MAX_NESTING: usize = 24;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub strong: bool,
    pub emphasis: bool,
    pub strike: bool,
    pub code: bool,
    pub link: Option<Arc<str>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub source: Range<usize>,
    pub style: Style,
    /// Math keeps its original delimited Markdown alongside its read-only
    /// Unicode projection. Failed subset parses stay visibly marked source.
    pub math: Option<crate::math::Projection>,
    /// Set only on the single literal run produced by `![alt](src)`. The text is
    /// still the inert `[image not loaded: …]` marker; the renderer decides
    /// whether that one run becomes a picture or stays marker text.
    pub image: Option<crate::images::Spec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMarker {
    pub checked: bool,
    /// Canonical editable token only; unsupported parser markers stay disabled.
    pub source: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Paragraph,
    Heading(u8),
    Quote,
    List(Option<u64>),
    Item(Option<TaskMarker>),
    Code { info: String, text: String },
    Math { display: bool },
    Rule,
    Table(Table),
    Literal { label: &'static str, text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: Kind,
    pub source: Range<usize>,
    pub runs: Vec<Run>,
    pub children: Vec<Block>,
    /// Root ATX markers only. Setext uses its separate underline metadata.
    pub heading_marker: Option<Range<usize>>,
    /// Root Setext underline run, separate from the ATX opening marker.
    pub setext_marker: Option<Range<usize>>,
    /// Single physical-line unindented root paragraphs only; not a glyph map.
    pub paragraph_payload: Option<Range<usize>>,
    pub multiline_paragraph_payload: Option<Range<usize>>,
    /// Root plain-text paragraphs editable as a whole block in Preview. This
    /// identifies Source bytes, not character or caret positions.
    pub inline_text_payload: Option<Range<usize>>,
}

impl Block {
    /// Containers have child targets; tables navigate as one block, not cells.
    pub fn source_navigable(&self) -> bool {
        !matches!(self.kind, Kind::Quote | Kind::List(_) | Kind::Item(_))
            && self.source.start < self.source.end
    }
}

pub fn parse(source: &str) -> Vec<Block> {
    // Explicit extensions: do not silently enable new syntax when the parser's
    // Options::all grows. Export remains independent and is not changed here.
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_MATH;
    blocks(
        &mut Parser::new_ext(source, options).into_offset_iter(),
        source,
        None,
        0,
    )
    .0
}

type Events<'a> = dyn Iterator<Item = (Event<'a>, Range<usize>)> + 'a;

fn raw(source: &str, range: &Range<usize>) -> String {
    source
        .get(range.clone())
        .unwrap_or("[preview source range unavailable]")
        .to_owned()
}

fn math_source(source: &str, range: &Range<usize>, expression: &str, display: bool) -> String {
    let delimiter = if display { "$$" } else { "$" };
    let original = raw(source, range);
    if original.starts_with(delimiter)
        && original.ends_with(delimiter)
        && original.len() >= delimiter.len() * 2
    {
        original
    } else {
        format!("{delimiter}{expression}{delimiter}")
    }
}

fn skip(events: &mut Events<'_>) {
    let mut depth = 1usize;
    for (event, _) in events {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            break;
        }
    }
}

/// Consume the image subtree and keep its alt text (Text/Code only).
fn image_alt(events: &mut Events<'_>) -> String {
    let mut depth = 1;
    let mut alt = String::new();
    for (event, _) in events {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Event::Text(text) | Event::Code(text) => alt.push_str(&text),
            _ => {}
        }
    }
    alt
}

fn node(kind: Kind, source: Range<usize>, runs: Vec<Run>, children: Vec<Block>) -> Block {
    Block {
        kind,
        source,
        runs,
        children,
        heading_marker: None,
        setext_marker: None,
        paragraph_payload: None,
        multiline_paragraph_payload: None,
        inline_text_payload: None,
    }
}

fn inline_text_payload(source: &str, block: &Block) -> Option<Range<usize>> {
    if block.kind != Kind::Paragraph
        || !block.children.is_empty()
        || block.multiline_paragraph_payload.is_some()
    {
        return None;
    }
    let payload = block.paragraph_payload.as_ref()?;
    if block.source.start != payload.start
        || payload.end.checked_add(1) != Some(block.source.end)
        || block.setext_marker.is_some()
    {
        return None;
    }
    let raw = source.get(payload.clone())?;
    if raw.contains('\n') || has_unescaped_square_bracket(raw) || block.runs.is_empty() {
        return None;
    }
    let before = source.get(..block.source.start)?;
    let after = source.get(block.source.end..)?;
    if (block.source.start != 0 && !before.ends_with("\n\n"))
        || (block.source.end != source.len() && !after.starts_with('\n'))
    {
        return None;
    }
    if block
        .runs
        .iter()
        .any(|run| run.style != Style::default() || run.math.is_some() || run.image.is_some())
    {
        return None;
    }
    let visible: String = block.runs.iter().map(|run| run.text.as_str()).collect();
    crate::preview_paragraph::valid_value(&visible).then(|| payload.clone())
}

fn has_unescaped_square_bracket(source: &str) -> bool {
    let mut backslashes = 0;
    for byte in source.bytes() {
        match byte {
            b'\\' => backslashes += 1,
            b'[' | b']' => {
                if backslashes % 2 == 0 {
                    return true;
                }
                backslashes = 0;
            }
            _ => backslashes = 0,
        }
    }
    false
}

#[derive(Default)]
struct Inline {
    runs: Vec<Run>,
    style: Style,
    stack: Vec<Style>,
    task: Option<TaskMarker>,
}

impl Inline {
    fn push(&mut self, text: String, range: Range<usize>, literal: bool) {
        let mut style = self.style.clone();
        style.code |= literal;
        self.runs.push(Run {
            text,
            source: range,
            style,
            math: None,
            image: None,
        });
    }

    /// An image stays one literal run so unrelated preview behaviour (tasks,
    /// tables, reveal, history) is unchanged; only a paragraph that is *just*
    /// this run is rendered as a picture.
    fn push_image(&mut self, marker: String, range: Range<usize>, spec: crate::images::Spec) {
        let mut style = self.style.clone();
        style.code = true;
        self.runs.push(Run {
            text: marker,
            source: range,
            style,
            math: None,
            image: Some(spec),
        });
    }

    fn push_math(&mut self, expression: String, range: Range<usize>, source: &str, display: bool) {
        let marked_source = math_source(source, &range, &expression, display);
        let (math, text) = crate::math::Projection::new(&expression, marked_source, display);
        let mut style = self.style.clone();
        style.code |= math.error.is_some();
        self.runs.push(Run {
            text,
            source: range,
            style,
            math: Some(math),
            image: None,
        });
    }

    fn event(
        &mut self,
        event: Event<'_>,
        range: Range<usize>,
        events: &mut Events<'_>,
        source: &str,
    ) {
        match event {
            Event::Start(Tag::Image { dest_url, .. }) => {
                // Alt text is the only part of the skipped subtree worth
                // keeping: it becomes the picture's tooltip.
                let alt = image_alt(events);
                self.push_image(
                    format!("[image not loaded: {}]", raw(source, &range)),
                    range,
                    crate::images::Spec {
                        src: dest_url.to_string(),
                        alt,
                    },
                );
            }
            Event::Start(tag) => {
                if self.stack.len() >= MAX_NESTING {
                    skip(events);
                    self.push(
                        format!("[inline nesting limit: {}]", raw(source, &range)),
                        range,
                        true,
                    );
                    return;
                }
                self.stack.push(self.style.clone());
                match tag {
                    Tag::Strong => self.style.strong = true,
                    Tag::Emphasis => self.style.emphasis = true,
                    Tag::Strikethrough => self.style.strike = true,
                    Tag::Link { dest_url, .. } => {
                        self.style.link = Some(Arc::from(dest_url.as_ref()))
                    }
                    _ => {}
                }
            }
            Event::End(_) => {
                self.style = self.stack.pop().unwrap_or_default();
            }
            Event::Text(text) => self.push(text.into_string(), range, false),
            Event::Code(text) => self.push(text.into_string(), range, true),
            Event::SoftBreak => self.push(" ".into(), range, false),
            Event::HardBreak => self.push("\n".into(), range, false),
            Event::TaskListMarker(checked) => {
                self.task = Some(TaskMarker {
                    checked,
                    source: crate::preview_task::marker_range(source, range, checked),
                })
            }
            Event::InlineMath(expression) => {
                self.push_math(expression.into_string(), range, source, false)
            }
            Event::DisplayMath(expression) => {
                self.push_math(expression.into_string(), range, source, true)
            }
            Event::Html(_) | Event::InlineHtml(_) | Event::FootnoteReference(_) => {
                self.push(raw(source, &range), range, true);
            }
            Event::Rule => {}
        }
    }

    fn flush(&mut self, out: &mut Vec<Block>) {
        if let (Some(first), Some(last)) = (self.runs.first(), self.runs.last()) {
            out.push(node(
                Kind::Paragraph,
                first.source.start..last.source.end,
                std::mem::take(&mut self.runs),
                vec![],
            ));
        }
    }
}

fn inline(events: &mut Events<'_>, source: &str, end: TagEnd) -> Inline {
    let mut content = Inline::default();
    while let Some((event, range)) = events.next() {
        if event == Event::End(end) {
            break;
        }
        content.event(event, range, events, source);
    }
    content
}

fn blocks(
    events: &mut Events<'_>,
    source: &str,
    end: Option<TagEnd>,
    depth: usize,
) -> (Vec<Block>, Option<TaskMarker>) {
    let mut out = Vec::new();
    let mut pending = Inline::default();
    let mut task = None;
    while let Some((event, range)) = events.next() {
        if matches!(&event, Event::End(tag) if Some(*tag) == end) {
            break;
        }
        match event {
            Event::Start(
                tag @ (Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough
                | Tag::Link { .. }
                | Tag::Image { .. }),
            ) => {
                pending.event(Event::Start(tag), range, events, source);
            }
            Event::Start(tag) => {
                pending.flush(&mut out);
                let tag_end = tag.to_end();
                if depth >= MAX_NESTING {
                    skip(events);
                    out.push(node(
                        Kind::Literal {
                            label: "Preview nesting limit — source",
                            text: raw(source, &range),
                        },
                        range,
                        vec![],
                        vec![],
                    ));
                    continue;
                }
                let block = match tag {
                    Tag::Paragraph | Tag::Heading { .. } => {
                        let kind = match tag {
                            Tag::Heading { level, .. } => Kind::Heading(level as u8),
                            _ => Kind::Paragraph,
                        };
                        let content = inline(events, source, tag_end);
                        let display_math = content.runs.len() == 1
                            && content.runs[0]
                                .math
                                .as_ref()
                                .is_some_and(|math| math.display);
                        task = task.or(content.task);
                        let kind = if kind == Kind::Paragraph && display_math {
                            Kind::Math { display: true }
                        } else {
                            kind
                        };
                        let mut block = node(kind, range, content.runs, vec![]);
                        if depth == 0 {
                            if block.kind == Kind::Paragraph {
                                block.paragraph_payload =
                                    crate::preview_paragraph::payload_range(source, &block.source);
                                block.multiline_paragraph_payload =
                                    crate::preview_paragraph_multiline::payload_range(
                                        source,
                                        &block.source,
                                    );
                                block.inline_text_payload = inline_text_payload(source, &block);
                            }
                            if let Kind::Heading(level) = block.kind {
                                block.heading_marker = crate::preview_heading::marker_range(
                                    source,
                                    &block.source,
                                    level,
                                );
                                if block.heading_marker.is_none() {
                                    block.setext_marker = crate::preview_setext::marker_range(
                                        source,
                                        &block.source,
                                        level,
                                    );
                                }
                            }
                        }
                        block
                    }
                    Tag::BlockQuote(_) | Tag::List(_) | Tag::Item => {
                        let (children, checked) = blocks(events, source, Some(tag_end), depth + 1);
                        let kind = match tag {
                            Tag::BlockQuote(_) => Kind::Quote,
                            Tag::List(start) => Kind::List(start),
                            _ => Kind::Item(checked),
                        };
                        node(kind, range, vec![], children)
                    }
                    Tag::Table(alignments) => {
                        table::parse(events, source, range, alignments, depth == 0)
                    }
                    Tag::CodeBlock(kind) => {
                        let info = match kind {
                            CodeBlockKind::Fenced(info) => info.into_string(),
                            CodeBlockKind::Indented => String::new(),
                        };
                        let mut text = String::new();
                        for (event, _) in &mut *events {
                            match event {
                                Event::End(TagEnd::CodeBlock) => break,
                                Event::Text(part) => text.push_str(&part),
                                _ => {}
                            }
                        }
                        node(Kind::Code { info, text }, range, vec![], vec![])
                    }
                    tag => {
                        let label = match tag {
                            Tag::HtmlBlock => "Raw HTML — inert source",
                            Tag::FootnoteDefinition(_) => "Footnote — source fallback",
                            _ => "Unsupported block — source fallback",
                        };
                        skip(events);
                        node(
                            Kind::Literal {
                                label,
                                text: raw(source, &range),
                            },
                            range,
                            vec![],
                            vec![],
                        )
                    }
                };
                out.push(block);
            }
            Event::DisplayMath(expression) => {
                pending.flush(&mut out);
                let mut content = Inline::default();
                content.push_math(expression.into_string(), range.clone(), source, true);
                out.push(node(
                    Kind::Math { display: true },
                    range,
                    content.runs,
                    vec![],
                ));
            }
            Event::Rule => {
                pending.flush(&mut out);
                out.push(node(Kind::Rule, range, vec![], vec![]));
            }
            event => pending.event(event, range, events, source),
        }
    }
    task = task.or(pending.task.take());
    pending.flush(&mut out);
    (out, task)
}

#[cfg(test)]
#[path = "preview_model_tests.rs"]
mod tests;
