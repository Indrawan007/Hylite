//! GFM table projection; the parser defines ragged-row normalization. Limits
//! bound eager table widgets, not parser time, source size or total memory.
use super::{inline, raw, skip, Block, Events, Kind};
use pulldown_cmark::{Event, Tag, TagEnd};
use std::ops::Range;

pub const MAX_COLUMNS: usize = crate::preview_table_edit::MAX_COLUMNS;
pub const MAX_ROWS: usize = 128; // Includes the header.
pub const MAX_CELLS: usize = 512; // Includes empty/padded cells and header.

pub use crate::preview_table_edit::Alignment;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub source: Range<usize>,
    pub runs: Vec<super::Run>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableRow {
    pub source: Range<usize>,
    pub header: bool,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub alignments: Vec<Alignment>,
    pub rows: Vec<TableRow>,
    /// Canonical root delimiter tokens only; nested/fallback tables stay inert.
    pub delimiters: Option<Vec<Range<usize>>>,
}

fn fallback(source: &str, range: Range<usize>) -> Block {
    super::node(
        Kind::Literal {
            label: "Table preview limit — source",
            text: raw(source, &range),
        },
        range,
        vec![],
        vec![],
    )
}

pub(super) fn parse(
    events: &mut Events<'_>,
    source: &str,
    range: Range<usize>,
    alignments: Vec<pulldown_cmark::Alignment>,
    root: bool,
) -> Block {
    let columns = alignments.len();
    if columns == 0 || columns > MAX_COLUMNS {
        skip(events); // Balanced subtree after Start(Table).
        return fallback(source, range);
    }
    let mut rows = Vec::new();
    while let Some((event, row_range)) = events.next() {
        match event {
            Event::End(TagEnd::Table) => break,
            Event::Start(tag @ (Tag::TableHead | Tag::TableRow)) => {
                if rows.len() >= MAX_ROWS || (rows.len() + 1) * columns > MAX_CELLS {
                    // Tables cannot nest in a GFM cell. Consume this row and
                    // all remaining rows through the matching table end.
                    for (event, _) in &mut *events {
                        if event == Event::End(TagEnd::Table) {
                            break;
                        }
                    }
                    return fallback(source, range);
                }
                let header = tag == Tag::TableHead;
                let end = tag.to_end();
                let mut cells = Vec::with_capacity(columns);
                while let Some((event, cell_range)) = events.next() {
                    if event == Event::End(end) {
                        break;
                    }
                    if event == Event::Start(Tag::TableCell) {
                        let content = inline(events, source, TagEnd::TableCell);
                        cells.push(Cell {
                            source: cell_range,
                            runs: content.runs,
                        });
                    }
                }
                debug_assert_eq!(cells.len(), columns);
                rows.push(TableRow {
                    source: row_range,
                    header,
                    cells,
                });
            }
            _ => {}
        }
    }
    let alignments: Vec<Alignment> = alignments
        .into_iter()
        .map(|alignment| match alignment {
            pulldown_cmark::Alignment::None => Alignment::Default,
            pulldown_cmark::Alignment::Left => Alignment::Left,
            pulldown_cmark::Alignment::Center => Alignment::Center,
            pulldown_cmark::Alignment::Right => Alignment::Right,
        })
        .collect();
    let delimiters = root
        .then(|| crate::preview_table_edit::delimiters(source, &range, columns))
        .flatten()
        .filter(|tokens| {
            tokens
                .iter()
                .map(|token| token.alignment)
                .eq(alignments.iter().copied())
        })
        .map(|tokens| tokens.into_iter().map(|token| token.source).collect());
    super::node(
        Kind::Table(Table {
            alignments,
            rows,
            delimiters,
        }),
        range,
        vec![],
        vec![],
    )
}
