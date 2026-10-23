//! Dependency-free PDF writer for the export path.
//!
//! This is deliberately not a typesetting engine: it emits a small, valid
//! PDF 1.4 file with the base-14 Helvetica/Courier faces, WinAnsiEncoding text,
//! filled rectangles and stroked lines. There is no font embedding, no image
//! embedding, no compression and no Unicode beyond WinAnsi — a character that
//! WinAnsi cannot represent becomes `?`. Advance widths come from one Helvetica
//! table, so bold/oblique lines may wrap a few characters early or late.
use std::fmt::Write as _;

pub const PAGE_WIDTH: f32 = 595.0;
pub const PAGE_HEIGHT: f32 = 842.0;
pub const MARGIN: f32 = 56.0;

const BODY_SIZE: f32 = 11.0;
const BODY_LINE: f32 = 15.6;
const CODE_SIZE: f32 = 8.5;
const CODE_LINE: f32 = 11.5;
const TABLE_SIZE: f32 = 9.5;
const TABLE_LINE: f32 = 12.5;
const TABLE_PADDING: f32 = 4.0;
const LIST_INDENT: f32 = 18.0;

/// Helvetica advance widths in 1/1000 em for ASCII 32..=126.
const HELVETICA_WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];
/// Every proportional glyph outside that table is measured as this wide.
const DEFAULT_WIDTH: u16 = 556;
/// Courier is monospaced, so its width is exact.
const MONO_WIDTH: u16 = 600;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub style: Style,
}

impl Run {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
    }

    pub fn with(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Default for Align {
    fn default() -> Self {
        Self::Left
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Heading {
        level: u8,
        runs: Vec<Run>,
    },
    Paragraph(Vec<Run>),
    Code(String),
    Quote(Vec<Item>),
    List {
        start: Option<u64>,
        items: Vec<Vec<Run>>,
    },
    Rule,
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
        aligns: Vec<Align>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Doc {
    pub blocks: Vec<Item>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub background: [f32; 3],
    pub text: [f32; 3],
    pub accent: [f32; 3],
    pub surface: [f32; 3],
}

impl Default for Palette {
    /// Light page, dark ink: a print default rather than the editor theme.
    fn default() -> Self {
        Self {
            background: [1.0, 1.0, 1.0],
            text: [0.12, 0.12, 0.14],
            accent: [0.20, 0.28, 0.55],
            surface: [0.94, 0.94, 0.96],
        }
    }
}

impl Palette {
    /// Derive a page palette from an editor theme: the page keeps the theme
    /// background and ink, while tables and code blocks get a surface mixed
    /// 6% towards the ink so they stay visible on light and dark themes.
    pub fn from_colors(background: [f32; 3], text: [f32; 3], accent: [f32; 3]) -> Self {
        let mix = |background: f32, ink: f32| background * 0.94 + ink * 0.06;
        Self {
            background,
            text,
            accent,
            surface: [
                mix(background[0], text[0]),
                mix(background[1], text[1]),
                mix(background[2], text[2]),
            ],
        }
    }
}

/// Clamp arbitrary theme floats into the PDF's 0..=1 colour range.
pub fn clamp_rgb(rgb: [f32; 3]) -> [f32; 3] {
    let clamp = |value: f32| {
        if value.is_nan() {
            0.0
        } else {
            value.clamp(0.0, 1.0)
        }
    };
    [clamp(rgb[0]), clamp(rgb[1]), clamp(rgb[2])]
}

/// One line of already-wrapped, style-segmented text.
type Line = Vec<(String, Style)>;

struct Canvas<'a> {
    palette: &'a Palette,
    pages: Vec<String>,
    stream: String,
    /// Distance from the top of the page to the next content row.
    cursor: f32,
}

impl<'a> Canvas<'a> {
    fn new(palette: &'a Palette) -> Self {
        let mut canvas = Self {
            palette,
            pages: Vec::new(),
            stream: String::new(),
            cursor: MARGIN,
        };
        canvas.begin_page();
        canvas
    }

    fn begin_page(&mut self) {
        let [r, g, b] = clamp_rgb(self.palette.background);
        self.stream =
            format!("{r:.3} {g:.3} {b:.3} rg 0 0 {PAGE_WIDTH:.1} {PAGE_HEIGHT:.1} re f\n");
        self.cursor = MARGIN;
    }

    fn end_page(&mut self) {
        let page = std::mem::take(&mut self.stream);
        self.pages.push(page);
    }

    /// True when `height` would cross the bottom margin on this page.
    fn would_break(&self, height: f32) -> bool {
        self.cursor + height > PAGE_HEIGHT - MARGIN && self.cursor > MARGIN
    }

    /// Start a new page when `height` would cross the bottom margin.
    fn ensure(&mut self, height: f32) {
        if self.cursor + height > PAGE_HEIGHT - MARGIN && self.cursor > MARGIN {
            self.end_page();
            self.begin_page();
        }
    }

    fn gap(&mut self, height: f32) {
        self.cursor += height;
    }

    fn fill_rect(&mut self, x: f32, top: f32, width: f32, height: f32, rgb: [f32; 3]) {
        let [r, g, b] = clamp_rgb(rgb);
        let y = PAGE_HEIGHT - top - height;
        let _ = writeln!(
            self.stream,
            "{r:.3} {g:.3} {b:.3} rg {x:.2} {y:.2} {width:.2} {height:.2} re f"
        );
    }

    fn stroke_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, rgb: [f32; 3]) {
        let [r, g, b] = clamp_rgb(rgb);
        let _ = writeln!(
            self.stream,
            "{width:.2} w {r:.3} {g:.3} {b:.3} RG {x1:.2} {:.2} m {x2:.2} {:.2} l S",
            PAGE_HEIGHT - y1,
            PAGE_HEIGHT - y2
        );
    }

    fn text_line(
        &mut self,
        x: f32,
        baseline_top: f32,
        text: &str,
        style: Style,
        size: f32,
        rgb: [f32; 3],
    ) {
        if text.is_empty() {
            return;
        }
        let font = font_name(style);
        let [r, g, b] = clamp_rgb(rgb);
        let encoded = pdf_string(text);
        if encoded.is_empty() {
            return;
        }
        let _ = writeln!(
            self.stream,
            "BT /{font} {size:.2} Tf {r:.3} {g:.3} {b:.3} rg {x:.2} {:.2} Td ({encoded}) Tj ET",
            PAGE_HEIGHT - baseline_top
        );
    }
}

fn font_name(style: Style) -> &'static str {
    match (style.mono, style.bold, style.italic) {
        (true, _, _) => "F4",
        (false, true, _) => "F2",
        (false, false, true) => "F3",
        _ => "F1",
    }
}

/// Escape a PDF literal string and transliterate to WinAnsi. Unsupported
/// characters become `?`, which is the only lossy step in the export.
pub fn pdf_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        let byte = match character {
            '\\' => {
                out.push_str("\\\\");
                continue;
            }
            '(' => {
                out.push_str("\\(");
                continue;
            }
            ')' => {
                out.push_str("\\)");
                continue;
            }
            '\n' => {
                out.push(' ');
                continue;
            }
            '\t' => {
                out.push(' ');
                continue;
            }
            c if (c as u32) < 0x20 => continue,
            c if (c as u32) < 0x7F => c as u8,
            c => match c {
                '\u{2014}' => 151,
                '\u{2013}' => 150,
                '\u{2018}' => 145,
                '\u{2019}' => 146,
                '\u{201C}' => 147,
                '\u{201D}' => 148,
                '\u{2022}' => 149,
                '\u{2026}' => 133,
                _ => {
                    let code = c as u32;
                    if (0xA0..=0xFF).contains(&code) {
                        code as u8
                    } else {
                        b'?'
                    }
                }
            },
        };
        out.push(byte as char);
    }
    out
}

/// Approximate advance width of one line in points.
pub fn text_width(text: &str, style: Style, size: f32) -> f32 {
    let units: u32 = pdf_string(text)
        .chars()
        .map(|character| {
            if style.mono {
                u32::from(MONO_WIDTH)
            } else {
                let code = character as u32;
                if (32..=126).contains(&code) {
                    u32::from(HELVETICA_WIDTHS[(code - 32) as usize])
                } else {
                    u32::from(DEFAULT_WIDTH)
                }
            }
        })
        .sum();
    units as f32 * size / 1000.0
}

/// Split runs into word pieces, keeping the trailing space so wrapping is fair.
fn pieces(runs: &[Run]) -> Vec<(String, Style)> {
    let mut out = Vec::new();
    for run in runs {
        let mut current = String::new();
        for character in run.text.chars() {
            current.push(character);
            if character.is_whitespace() {
                out.push((std::mem::take(&mut current), run.style));
            }
        }
        if !current.is_empty() {
            out.push((current, run.style));
        }
    }
    out
}

fn push_segment(line: &mut Line, text: &str, style: Style) {
    if let Some((last, last_style)) = line.last_mut() {
        if *last_style == style {
            last.push_str(text);
            return;
        }
    }
    line.push((text.to_string(), style));
}

/// Break one over-wide word by character so it can never overflow the margin.
fn hard_split(
    lines: &mut Vec<Line>,
    line: &mut Line,
    width: &mut f32,
    text: &str,
    style: Style,
    size: f32,
    max: f32,
) {
    let mut chunk = String::new();
    for character in text.chars() {
        let candidate = format!("{chunk}{character}");
        if !chunk.is_empty() && text_width(&candidate, style, size) > max {
            push_segment(line, &chunk, style);
            lines.push(std::mem::take(line));
            *width = 0.0;
            chunk = character.to_string();
        } else {
            chunk = candidate;
        }
    }
    if !chunk.is_empty() {
        *width += text_width(&chunk, style, size);
        push_segment(line, &chunk, style);
    }
}

/// Greedy wrap: fill each line until the next word would cross `max_width`.
fn wrap(runs: &[Run], max_width: f32, size: f32) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut line: Line = Vec::new();
    let mut width = 0.0_f32;

    for (text, style) in pieces(runs) {
        let piece_width = text_width(&text, style, size);
        let trimmed = text.trim_end();
        if width + piece_width > max_width && !line.is_empty() && !trimmed.is_empty() {
            lines.push(std::mem::take(&mut line));
            width = 0.0;
        }
        if piece_width > max_width && trimmed.len() > 1 {
            hard_split(
                &mut lines, &mut line, &mut width, &text, style, size, max_width,
            );
        } else {
            width += piece_width;
            push_segment(&mut line, &text, style);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

fn trim_line(line: &mut Line) {
    while let Some((text, _)) = line.last_mut() {
        let trimmed = text.trim_end().to_string();
        if trimmed.is_empty() {
            line.pop();
        } else {
            *text = trimmed;
            break;
        }
    }
}

struct Writer<'a> {
    canvas: Canvas<'a>,
    palette: &'a Palette,
    /// Left edge of the current content column; quotes and lists indent it.
    indent: f32,
}

impl<'a> Writer<'a> {
    fn content_width(&self) -> f32 {
        PAGE_WIDTH - 2.0 * MARGIN - self.indent
    }

    fn left(&self) -> f32 {
        MARGIN + self.indent
    }

    fn write_runs(&mut self, runs: &[Run], size: f32, line_height: f32, rgb: [f32; 3]) {
        let width = self.content_width();
        let left = self.left();
        let mut lines = wrap(runs, width, size);
        for line in &mut lines {
            trim_line(line);
        }
        for line in lines {
            self.canvas.ensure(line_height);
            let mut x = left;
            let baseline = self.canvas.cursor + size;
            for (text, style) in line {
                self.canvas.text_line(x, baseline, &text, style, size, rgb);
                x += text_width(&text, style, size);
            }
            self.canvas.gap(line_height);
        }
    }

    fn blocks(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Heading { level, runs } => {
                    let size = match level {
                        1 => 21.0,
                        2 => 17.0,
                        3 => 14.5,
                        _ => 12.5,
                    };
                    self.canvas.gap(if self.canvas.cursor > MARGIN {
                        8.0
                    } else {
                        0.0
                    });
                    let bold: Vec<Run> = runs
                        .iter()
                        .map(|run| {
                            Run::with(
                                run.text.clone(),
                                Style {
                                    bold: true,
                                    ..run.style
                                },
                            )
                        })
                        .collect();
                    self.write_runs(&bold, size, size * 1.35, self.palette.accent);
                    self.canvas.gap(4.0);
                }
                Item::Paragraph(runs) => {
                    self.write_runs(runs, BODY_SIZE, BODY_LINE, self.palette.text);
                    self.canvas.gap(5.0);
                }
                Item::Code(text) => self.code(text),
                Item::Quote(items) => {
                    self.canvas.gap(2.0);
                    let top = self.canvas.cursor;
                    let outer = self.indent;
                    self.indent += 14.0;
                    self.blocks(items);
                    let bottom = self.canvas.cursor;
                    self.canvas.stroke_line(
                        MARGIN + outer + 4.0,
                        top,
                        MARGIN + outer + 4.0,
                        bottom,
                        2.0,
                        self.palette.accent,
                    );
                    self.indent = outer;
                    self.canvas.gap(4.0);
                }
                Item::List { start, items } => {
                    for (index, runs) in items.iter().enumerate() {
                        let marker = match start {
                            Some(start) => format!("{}.", start + index as u64),
                            None => "\u{2022}".to_string(),
                        };
                        let outer = self.indent;
                        self.canvas.ensure(BODY_LINE);
                        let marker_baseline = self.canvas.cursor + BODY_SIZE;
                        self.canvas.text_line(
                            self.left(),
                            marker_baseline,
                            &marker,
                            Style::default(),
                            BODY_SIZE,
                            self.palette.text,
                        );
                        self.indent += LIST_INDENT;
                        self.write_runs(runs, BODY_SIZE, BODY_LINE, self.palette.text);
                        self.indent = outer;
                    }
                    self.canvas.gap(4.0);
                }
                Item::Rule => {
                    self.canvas.gap(6.0);
                    self.canvas.ensure(2.0);
                    let y = self.canvas.cursor;
                    self.canvas.stroke_line(
                        self.left(),
                        y,
                        PAGE_WIDTH - MARGIN,
                        y,
                        0.7,
                        self.palette.text,
                    );
                    self.canvas.gap(8.0);
                }
                Item::Table {
                    header,
                    rows,
                    aligns,
                } => self.table(header, rows, aligns),
            }
        }
    }

    fn code(&mut self, text: &str) {
        let style = Style {
            mono: true,
            ..Style::default()
        };
        let width = self.content_width() - 2.0 * 6.0;
        let mut lines: Vec<String> = Vec::new();
        for raw in text.lines() {
            let mut remaining = raw.to_string();
            if remaining.trim_end().is_empty() {
                lines.push(String::new());
                continue;
            }
            while text_width(&remaining, style, CODE_SIZE) > width {
                let mut take = 1;
                for (index, _) in remaining.char_indices().skip(1) {
                    if text_width(&remaining[..index], style, CODE_SIZE) > width {
                        break;
                    }
                    take = index;
                }
                let split = remaining
                    .char_indices()
                    .nth(take)
                    .map_or(remaining.len(), |(index, _)| index);
                lines.push(remaining[..split].to_string());
                remaining = remaining[split..].to_string();
            }
            lines.push(remaining);
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        let height = lines.len() as f32 * CODE_LINE + 2.0 * 6.0;
        self.canvas.ensure(height);
        let top = self.canvas.cursor;
        self.canvas.fill_rect(
            self.left(),
            top,
            self.content_width(),
            height,
            self.palette.surface,
        );
        self.canvas.gap(6.0);
        for line in lines {
            self.canvas.ensure(CODE_LINE);
            let baseline = self.canvas.cursor + CODE_SIZE;
            self.canvas.text_line(
                self.left() + 6.0,
                baseline,
                &line,
                style,
                CODE_SIZE,
                self.palette.text,
            );
            self.canvas.gap(CODE_LINE);
        }
        self.canvas.gap(6.0 + 5.0);
    }

    fn table(&mut self, header: &[String], rows: &[Vec<String>], aligns: &[Align]) {
        let columns = header
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if columns == 0 {
            return;
        }
        let available = self.content_width();
        let mut natural = vec![40.0_f32; columns];
        let measure =
            |text: &str| text_width(text, Style::default(), TABLE_SIZE) + 2.0 * TABLE_PADDING + 6.0;
        for (index, cell) in header.iter().enumerate() {
            natural[index] = natural[index].max(measure(cell));
        }
        for row in rows {
            for (index, cell) in row.iter().enumerate().take(columns) {
                natural[index] = natural[index].max(measure(cell));
            }
        }
        let total: f32 = natural.iter().sum();
        let widths: Vec<f32> = if total > available {
            natural
                .iter()
                .map(|width| width * available / total)
                .collect()
        } else {
            let extra = (available - total) / columns as f32;
            natural.iter().map(|width| width + extra).collect()
        };

        let header_cells: Vec<String> = (0..columns)
            .map(|index| header.get(index).cloned().unwrap_or_default())
            .collect();
        self.table_row(&header_cells, &widths, aligns, true);
        for row in rows {
            let cells: Vec<String> = (0..columns)
                .map(|index| row.get(index).cloned().unwrap_or_default())
                .collect();
            // Header repeat keeps a table readable across a page break.
            let row_height = self.row_height(&cells, &widths);
            if self.canvas.would_break(row_height) {
                self.canvas.ensure(row_height);
                self.table_row(&header_cells, &widths, aligns, true);
            }
            self.table_row(&cells, &widths, aligns, false);
        }
        self.canvas.gap(6.0);
    }

    fn row_height(&self, cells: &[String], widths: &[f32]) -> f32 {
        let lines = cells
            .iter()
            .zip(widths)
            .map(|(cell, width)| {
                wrap(
                    &[Run::plain(cell.clone())],
                    width - 2.0 * TABLE_PADDING,
                    TABLE_SIZE,
                )
                .len()
            })
            .max()
            .unwrap_or(1)
            .max(1);
        lines as f32 * TABLE_LINE + 2.0 * TABLE_PADDING
    }

    fn table_row(&mut self, cells: &[String], widths: &[f32], aligns: &[Align], header: bool) {
        let style = Style {
            bold: header,
            ..Style::default()
        };
        let wrapped: Vec<Vec<String>> = cells
            .iter()
            .zip(widths)
            .map(|(cell, width)| {
                let runs = vec![Run::with(cell.clone(), style)];
                wrap(&runs, width - 2.0 * TABLE_PADDING, TABLE_SIZE)
                    .into_iter()
                    .map(|line| {
                        line.into_iter()
                            .map(|(text, _)| text)
                            .collect::<Vec<_>>()
                            .join("")
                            .trim_end()
                            .to_string()
                    })
                    .collect()
            })
            .collect();
        let lines = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let height = lines as f32 * TABLE_LINE + 2.0 * TABLE_PADDING;
        self.canvas.ensure(height);
        let top = self.canvas.cursor;
        if header {
            self.canvas.fill_rect(
                self.left(),
                top,
                widths.iter().sum(),
                height,
                self.palette.surface,
            );
        }
        let mut x = self.left();
        for (index, width) in widths.iter().enumerate() {
            let align = aligns.get(index).copied().unwrap_or_default();
            for (line_index, line) in wrapped[index].iter().enumerate() {
                let line_width = text_width(line, style, TABLE_SIZE);
                let offset = match align {
                    Align::Left => 0.0,
                    Align::Center => (*width - 2.0 * TABLE_PADDING - line_width) / 2.0,
                    Align::Right => *width - 2.0 * TABLE_PADDING - line_width,
                }
                .max(0.0);
                let baseline = top + TABLE_PADDING + line_index as f32 * TABLE_LINE + TABLE_SIZE;
                self.canvas.text_line(
                    x + TABLE_PADDING + offset,
                    baseline,
                    line,
                    style,
                    TABLE_SIZE,
                    self.palette.text,
                );
            }
            x += width;
        }
        let mut x = self.left();
        let right = x + widths.iter().sum::<f32>();
        for width in widths {
            self.canvas
                .stroke_line(x, top, x, top + height, 0.5, self.palette.text);
            x += width;
        }
        self.canvas
            .stroke_line(right, top, right, top + height, 0.5, self.palette.text);
        self.canvas
            .stroke_line(self.left(), top, right, top, 0.7, self.palette.text);
        self.canvas.stroke_line(
            self.left(),
            top + height,
            right,
            top + height,
            0.7,
            self.palette.text,
        );
        self.canvas.gap(height);
    }
}

/// Render a fully laid-out document into PDF bytes. Deterministic: the same
/// `Doc` and `Palette` always produce identical bytes.
pub fn render(doc: &Doc, palette: &Palette) -> Vec<u8> {
    let mut writer = Writer {
        canvas: Canvas::new(palette),
        palette,
        indent: 0.0,
    };
    writer.blocks(&doc.blocks);
    writer.canvas.end_page();
    let pages = writer.canvas.pages;

    // Objects: 1 catalog, 2 pages, 3..6 fonts, then page/content pairs.
    let page_objects: Vec<usize> = (0..pages.len()).map(|index| 7 + index * 2).collect();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids = page_objects
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes());
    for base in [
        "Helvetica",
        "Helvetica-Bold",
        "Helvetica-Oblique",
        "Courier",
    ] {
        objects.push(
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{base} /Encoding /WinAnsiEncoding >>"
            )
            .into_bytes(),
        );
    }
    for (index, content) in pages.iter().enumerate() {
        let content_id = page_objects[index] + 1;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH:.1} {PAGE_HEIGHT:.1}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .into_bytes(),
        );
        objects.push(
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                content.len(),
                content
            )
            .into_bytes(),
        );
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_palette() -> Palette {
        Palette::default()
    }

    /// One char per byte, so string indices stay byte offsets even though the
    /// file starts with a non-UTF-8 binary comment.
    fn content_of(pdf: &[u8]) -> String {
        pdf.iter().map(|byte| char::from(*byte)).collect()
    }

    fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .rposition(|window| window == needle)
    }

    fn lines_of(bytes: &[u8]) -> Vec<&[u8]> {
        bytes.split(|byte| *byte == b'\n').collect()
    }

    /// Parse the xref table and verify every offset points at its object header.
    /// Byte offsets, not character offsets: the file starts with a binary comment.
    fn assert_valid_xref(pdf: &[u8]) {
        let start = find_last(pdf, b"startxref\n").expect("startxref");
        let digits = &pdf[start + 10..];
        let end = digits
            .iter()
            .position(|byte| !byte.is_ascii_digit())
            .expect("offset digits");
        let offset: usize = std::str::from_utf8(&digits[..end])
            .unwrap()
            .parse()
            .unwrap();
        assert!(
            pdf[offset..].starts_with(b"xref\n"),
            "startxref {offset} points at {:?}",
            std::str::from_utf8(&pdf[offset..offset + 8]).unwrap_or("<binary>")
        );
        let lines = lines_of(&pdf[offset..]);
        assert_eq!(lines[0], b"xref");
        let count: usize = std::str::from_utf8(lines[1])
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(lines[2], b"0000000000 65535 f ");
        for object in 1..count {
            let entry = lines[2 + object];
            assert_eq!(entry.len(), 19, "xref entries are 20 bytes including LF");
            let at: usize = std::str::from_utf8(&entry[..10]).unwrap().parse().unwrap();
            let expected = format!("{object} 0 obj\n");
            assert!(
                pdf[at..].starts_with(expected.as_bytes()),
                "object {object} at {at}"
            );
        }
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    fn assert_stream_lengths(pdf: &[u8]) {
        let mut rest = pdf;
        while let Some(index) = rest
            .windows(b"<< /Length ".len())
            .position(|window| window == b"<< /Length ")
        {
            let after = &rest[index + 11..];
            let digits = after
                .iter()
                .position(|byte| !byte.is_ascii_digit())
                .unwrap();
            let length: usize = std::str::from_utf8(&after[..digits])
                .unwrap()
                .parse()
                .unwrap();
            let stream_start = after
                .windows(7)
                .position(|window| window == b"stream\n")
                .unwrap()
                + 7;
            let body = &after[stream_start..stream_start + length];
            assert!(after[stream_start + length..].starts_with(b"endstream"));
            rest = body;
        }
    }

    #[test]
    fn an_empty_document_is_a_valid_single_page_pdf() {
        let pdf = render(&Doc::default(), &sample_palette());
        assert_valid_xref(&pdf);
        assert_stream_lengths(&pdf);
        let text = content_of(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/Count 1"));
        assert!(text.contains("/BaseFont /Helvetica"));
        assert!(!text.contains(" Tj"), "no text in an empty document");
    }

    #[test]
    fn identical_input_produces_identical_bytes() {
        let doc = Doc {
            blocks: vec![
                Item::Heading {
                    level: 1,
                    runs: vec![Run::plain("Judul")],
                },
                Item::Paragraph(vec![Run::plain("Isi dokumen.")]),
            ],
        };
        let first = render(&doc, &sample_palette());
        let second = render(&doc, &sample_palette());
        assert_eq!(first, second);
    }

    #[test]
    fn headings_paragraphs_and_code_reach_the_content_stream() {
        let doc = Doc {
            blocks: vec![
                Item::Heading {
                    level: 1,
                    runs: vec![Run::plain("Catatan")],
                },
                Item::Paragraph(vec![
                    Run::plain("Teks "),
                    Run::with(
                        "tebal",
                        Style {
                            bold: true,
                            ..Style::default()
                        },
                    ),
                    Run::plain(" dan "),
                    Run::with(
                        "kode",
                        Style {
                            mono: true,
                            ..Style::default()
                        },
                    ),
                ]),
                Item::Code("fn main() {\n    println!(\"hi\");\n}".into()),
                Item::Rule,
            ],
        };
        let pdf = render(&doc, &sample_palette());
        assert_valid_xref(&pdf);
        assert_stream_lengths(&pdf);
        let text = content_of(&pdf);
        assert!(text.contains("(Catatan) Tj"));
        assert!(text.contains("/F2 21.00 Tf"), "h1 is bold and 21pt");
        assert!(text.contains("(tebal) Tj"));
        assert!(text.contains("/F4 8.50 Tf"), "code uses Courier");
        assert!(text.contains("fn main\\(\\)"));
        assert!(text.contains(" re f"));
        assert!(text.contains(" l S"));
    }

    #[test]
    fn text_is_escaped_and_non_winansi_becomes_a_question_mark() {
        assert_eq!(pdf_string("a(b)c\\d"), "a\\(b\\)c\\\\d");
        assert_eq!(pdf_string("café"), "café");
        assert_eq!(pdf_string("tab\tnewline\njoin"), "tab newline join");
        assert_eq!(pdf_string("emoji 🙂 dan 日本"), "emoji ? dan ??");
        assert_eq!(
            pdf_string("— – “quote” • …"),
            "\u{97} \u{96} \u{93}quote\u{94} \u{95} \u{85}"
        );
        assert_eq!(pdf_string("\u{7}bell"), "bell", "control chars are dropped");
    }

    #[test]
    fn a_long_paragraph_wraps_inside_the_margins() {
        let text = "kata ".repeat(2000);
        let doc = Doc {
            blocks: vec![Item::Paragraph(vec![Run::plain(text)])],
        };
        let pdf = render(&doc, &sample_palette());
        assert_valid_xref(&pdf);
        let body = content_of(&pdf);
        // Every Tj chunk must fit the content width at body size.
        let mut checked = 0;
        for line in body.lines() {
            if let Some(start) = line.find(" Td (") {
                let chunk = &line[start + 5..];
                let end = chunk.rfind(") Tj").unwrap();
                let width = text_width(&chunk[..end], Style::default(), BODY_SIZE);
                assert!(width <= PAGE_WIDTH - 2.0 * MARGIN + 0.5, "{width}");
                checked += 1;
            }
        }
        assert!(checked > 20, "wrapped into many lines: {checked}");
        assert!(
            body.matches("/Type /Page /Parent").count() > 1,
            "long text spans a second page"
        );
    }

    #[test]
    fn one_unbreakable_word_is_hard_split_instead_of_overflowing() {
        let doc = Doc {
            blocks: vec![Item::Paragraph(vec![Run::plain("x".repeat(400))])],
        };
        let body = content_of(&render(&doc, &sample_palette()));
        for line in body.lines() {
            if let Some(start) = line.find(" Td (") {
                let chunk = &line[start + 5..];
                let end = chunk.rfind(") Tj").unwrap();
                assert!(
                    text_width(&chunk[..end], Style::default(), BODY_SIZE)
                        <= PAGE_WIDTH - 2.0 * MARGIN + 0.5
                );
            }
        }
    }

    #[test]
    fn tables_draw_borders_cells_and_repeat_the_header_across_pages() {
        let header = vec!["Nama".to_string(), "Nilai".to_string()];
        let rows: Vec<Vec<String>> = (0..40)
            .map(|index| vec![format!("baris {index}"), format!("{}", index * 3)])
            .collect();
        let doc = Doc {
            blocks: vec![Item::Table {
                header,
                rows,
                aligns: vec![Align::Center, Align::Right],
            }],
        };
        let pdf = render(&doc, &sample_palette());
        assert_valid_xref(&pdf);
        assert_stream_lengths(&pdf);
        let body = content_of(&pdf);
        assert!(body.contains("(baris 0) Tj"));
        assert!(body.contains("(baris 39) Tj"));
        assert!(body.contains("(Nama) Tj"));
        let header_repeats = body.matches("(Nama) Tj").count();
        let pages: usize = body
            .split("/Count ")
            .nth(1)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(pages > 1, "the table paginates");
        assert_eq!(header_repeats, pages, "the header repeats on each page");
        assert!(body.contains("0.50 w"), "cell borders are stroked");
        // Centred and right-aligned cells still place their text inside the
        // column: every cell line starts at or after the left margin.
        assert!(body.contains("(baris 1) Tj"));
    }

    #[test]
    fn left_aligned_cells_and_wide_tables_still_fit_the_content_width() {
        let header = vec![
            "kolom yang sangat panjang sekali".to_string(),
            "b".to_string(),
        ];
        let rows = vec![vec![
            "nilai yang juga panjang sekali di sini".to_string(),
            "2".to_string(),
        ]];
        let doc = Doc {
            blocks: vec![Item::Table {
                header,
                rows,
                aligns: vec![Align::Left, Align::Left],
            }],
        };
        let pdf = render(&doc, &sample_palette());
        assert_valid_xref(&pdf);
        let body = content_of(&pdf);
        assert!(body.contains("(b) Tj") && body.contains("(2) Tj"));
    }

    #[test]
    fn quotes_indent_and_lists_emit_markers() {
        let doc = Doc {
            blocks: vec![
                Item::Quote(vec![Item::Paragraph(vec![Run::plain("dikutip")])]),
                Item::List {
                    start: Some(3),
                    items: vec![vec![Run::plain("ketiga")], vec![Run::plain("keempat")]],
                },
            ],
        };
        let body = content_of(&render(&doc, &sample_palette()));
        assert!(body.contains("(dikutip) Tj"));
        assert!(body.contains("(3.) Tj") && body.contains("(4.) Tj"));
        assert!(body.contains(" l S"), "the quote rule is stroked");
    }

    #[test]
    fn a_theme_palette_mixes_a_surface_towards_the_ink() {
        let palette = Palette::from_colors([1.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.2, 0.3, 0.4]);
        assert_eq!(palette.background, [1.0, 1.0, 1.0]);
        assert_eq!(palette.accent, [0.2, 0.3, 0.4]);
        assert!((palette.surface[0] - 0.94).abs() < 1e-6);
        assert!(palette.surface[0] < palette.background[0]);
    }

    #[test]
    fn the_palette_colours_are_clamped_and_drive_the_page_background() {
        let palette = Palette {
            background: [2.0, -1.0, f32::NAN],
            ..Palette::default()
        };
        let body = content_of(&render(&Doc::default(), &palette));
        assert!(body.contains("1.000 0.000 0.000 rg 0 0 595.0 842.0 re f"));
    }

    #[test]
    fn many_paragraphs_paginate_and_keep_every_margin_line_inside_the_page() {
        let doc = Doc {
            blocks: (0..60)
                .map(|index| Item::Paragraph(vec![Run::plain(format!("paragraf {index}"))]))
                .collect(),
        };
        let pdf = render(&doc, &sample_palette());
        assert_valid_xref(&pdf);
        let body = content_of(&pdf);
        let pages = body.matches("/Type /Page /Parent").count();
        assert!(pages >= 2, "60 paragraphs need more than one page: {pages}");
        for line in body.lines() {
            if let Some(start) = line.find(" Td (") {
                let y: f32 = line[..start]
                    .split_whitespace()
                    .last()
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!((MARGIN - 0.5..=PAGE_HEIGHT - MARGIN).contains(&y), "{y}");
            }
        }
    }
}
