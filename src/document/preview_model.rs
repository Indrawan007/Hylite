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

fn node(kind: Kind, source: Range<usize>, runs: Vec<Run>, children: Vec<Block>) -> Block {
    Block {
        kind,
        source,
        runs,
        children,
        heading_marker: None,
        setext_marker: None,
    }
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
            Event::Start(Tag::Image { .. }) => {
                skip(events);
                self.push(
                    format!("[image not loaded: {}]", raw(source, &range)),
                    range,
                    true,
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
            Event::Html(_)
            | Event::InlineHtml(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_)
            | Event::FootnoteReference(_) => {
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
                        task = task.or(content.task);
                        let mut block = node(kind, range, content.runs, vec![]);
                        if depth == 0 {
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
                    Tag::Table(alignments) => table::parse(events, source, range, alignments),
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
