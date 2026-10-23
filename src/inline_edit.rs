//! A deliberately narrow Stage 9a projection: one ASCII root paragraph line,
//! with source-preserving `**strong**`, `*emphasis*`, and `` `code` `` spans.
//! This is an experiment, not a general Markdown or glyph/selection mapper.
use crate::text_edit::TextChange;
use std::ops::Range;

pub const MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Plain,
    Strong,
    Emphasis,
    Code,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub visible: Range<usize>,
    /// The visible bytes' exact raw Source bytes; delimiters are excluded.
    pub source: Range<usize>,
    pub style: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidRange,
    Empty,
    TooLarge,
    MultipleLines,
    NonAscii,
    ControlCharacter,
    UnsupportedMarkup,
    UnsupportedEdit,
    UnsupportedCharacter,
    InvalidCaret,
    CrossesStyleBoundary,
}

impl Error {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidRange => "source range is invalid",
            Self::Empty => "empty paragraphs are not part of the spike",
            Self::TooLarge => "line exceeds the 16 KiB spike bound",
            Self::MultipleLines => "only one physical line is supported",
            Self::NonAscii => "this spike currently accepts ASCII only",
            Self::ControlCharacter => "control characters are unsupported",
            Self::UnsupportedMarkup => "inline syntax is outside the Stage 9a subset",
            Self::UnsupportedEdit => "only one-character insert/delete edits are supported",
            Self::UnsupportedCharacter => "that character is outside the safe spike subset",
            Self::InvalidCaret => "the editor caret has no verified byte mapping",
            Self::CrossesStyleBoundary => "an edit may not cross an inline-style boundary",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    visible: String,
    segments: Vec<Segment>,
    /// Visible byte boundary -> raw Source byte offset. The accepted alphabet is
    /// ASCII so every TextEditor column is a valid entry in this table.
    source_offsets: Vec<usize>,
    /// A formatted run owns both of its visible edge boundaries in this spike.
    /// Stage 9b will add explicit caret affinity instead of this deterministic rule.
    caret_styles: Vec<Style>,
}

impl Projection {
    pub fn visible(&self) -> &str {
        &self.visible
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn source_offset_at(&self, visible_byte: usize) -> Option<usize> {
        self.source_offsets.get(visible_byte).copied()
    }

    pub fn caret_style_at(&self, visible_byte: usize) -> Option<Style> {
        self.caret_styles.get(visible_byte).copied()
    }

    pub fn style_at(&self, visible_byte: usize) -> Option<Style> {
        self.segments
            .iter()
            .find(|segment| {
                segment.visible.start <= visible_byte && visible_byte < segment.visible.end
            })
            .map(|segment| segment.style)
    }

    pub fn has_formatted_run(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.style != Style::Plain)
    }

    /// Translate one native TextEditor insert/delete into a Source byte splice.
    /// The caller must still preflight the resulting Markdown model before commit.
    pub fn plan_edit(
        &self,
        next_visible: &str,
        before_caret: usize,
        after_caret: usize,
    ) -> Result<TextChange, Error> {
        if next_visible == self.visible {
            return Err(Error::UnsupportedEdit);
        }
        if !next_visible.is_ascii()
            || next_visible
                .bytes()
                .any(|byte| matches!(byte, b'\n' | b'\r'))
        {
            return Err(Error::UnsupportedCharacter);
        }

        let before = self.visible.as_bytes();
        let after = next_visible.as_bytes();
        if before_caret > before.len() {
            return Err(Error::InvalidCaret);
        }

        if after.len() == before.len() + 1 && after_caret == before_caret + 1 {
            if after.get(..before_caret) != before.get(..before_caret)
                || after.get(before_caret + 1..) != before.get(before_caret..)
            {
                return Err(Error::UnsupportedEdit);
            }
            let byte = *after.get(before_caret).ok_or(Error::InvalidCaret)?;
            let ch = char::from(byte);
            let style = self
                .caret_style_at(before_caret)
                .ok_or(Error::InvalidCaret)?;
            if !safe_insert(ch, style) {
                return Err(Error::UnsupportedCharacter);
            }
            let source = self
                .source_offset_at(before_caret)
                .ok_or(Error::InvalidCaret)?;
            return Ok(TextChange {
                range: source..source,
                replacement: ch.to_string(),
                caret: 1,
            });
        }

        if before.len() == after.len() + 1 {
            let deleted = if after_caret.checked_add(1) == Some(before_caret) {
                after_caret
            } else if after_caret == before_caret {
                before_caret
            } else {
                return Err(Error::UnsupportedEdit);
            };
            let deleted_end = deleted.checked_add(1).ok_or(Error::InvalidCaret)?;
            if deleted_end > before.len()
                || before.get(..deleted) != after.get(..deleted)
                || before.get(deleted_end..) != after.get(deleted..)
            {
                return Err(Error::UnsupportedEdit);
            }

            let segment = self
                .segments
                .iter()
                .find(|segment| {
                    segment.visible.start <= deleted && deleted_end <= segment.visible.end
                })
                .ok_or(Error::CrossesStyleBoundary)?;
            let start = segment
                .source
                .start
                .checked_add(deleted - segment.visible.start)
                .ok_or(Error::InvalidCaret)?;
            let end = start.checked_add(1).ok_or(Error::InvalidCaret)?;
            if end > segment.source.end {
                return Err(Error::InvalidCaret);
            }
            return Ok(TextChange {
                range: start..end,
                replacement: String::new(),
                caret: 0,
            });
        }

        Err(Error::UnsupportedEdit)
    }

    /// Whether the character at this caret can be inserted without letting the
    /// spike synthesize new Markdown structure or a delimiter.
    pub fn can_insert(&self, caret: usize, ch: char) -> bool {
        self.caret_style_at(caret)
            .is_some_and(|style| safe_insert(ch, style))
    }
}

/// Build a visible-text projection for the given absolute Source payload range.
/// Root-paragraph membership is authenticated by the app/controller, not here.
pub fn project(source: &str, range: Range<usize>) -> Result<Projection, Error> {
    if !crate::text_edit::valid_range(source, &range) {
        return Err(Error::InvalidRange);
    }
    if range.is_empty() {
        return Err(Error::Empty);
    }
    if range.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let raw = source.get(range.clone()).ok_or(Error::InvalidRange)?;
    if raw.bytes().any(|byte| matches!(byte, b'\n' | b'\r')) {
        return Err(Error::MultipleLines);
    }
    if !raw.is_ascii() {
        return Err(Error::NonAscii);
    }
    if raw.bytes().any(|byte| byte < b' ' || byte == 0x7f) {
        return Err(Error::ControlCharacter);
    }

    let runs = scan_runs(raw)?;
    let mut projection = Projection {
        visible: String::new(),
        segments: Vec::with_capacity(runs.len()),
        source_offsets: Vec::new(),
        caret_styles: Vec::new(),
    };
    for (local, style) in runs {
        let absolute = range.start + local.start..range.start + local.end;
        append_segment(&mut projection, &source[absolute.clone()], absolute, style);
    }
    if projection.visible.is_empty() {
        return Err(Error::Empty);
    }
    debug_assert_eq!(projection.source_offsets.len(), projection.visible.len() + 1);
    debug_assert_eq!(projection.caret_styles.len(), projection.visible.len() + 1);
    Ok(projection)
}

fn scan_runs(raw: &str) -> Result<Vec<(Range<usize>, Style)>, Error> {
    let bytes = raw.as_bytes();
    let mut runs = Vec::new();
    let mut cursor = 0;
    let mut plain_start = 0;
    let mut previous_was_formatted = false;

    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte != b'*' && byte != b'`' {
            if unsupported_plain(byte) {
                return Err(Error::UnsupportedMarkup);
            }
            cursor += 1;
            continue;
        }

        if plain_start < cursor {
            runs.push((plain_start..cursor, Style::Plain));
            previous_was_formatted = false;
        }
        if previous_was_formatted {
            return Err(Error::UnsupportedMarkup);
        }

        let (style, content, close_end) = if byte == b'`' {
            let close = bytes[cursor + 1..]
                .iter()
                .position(|candidate| *candidate == b'`')
                .map(|offset| cursor + 1 + offset)
                .ok_or(Error::UnsupportedMarkup)?;
            (Style::Code, cursor + 1..close, close + 1)
        } else {
            let strong = bytes.get(cursor + 1) == Some(&b'*');
            let marker = if strong { 2 } else { 1 };
            if strong && bytes.get(cursor + 2) == Some(&b'*') {
                return Err(Error::UnsupportedMarkup);
            }
            let style = if strong { Style::Strong } else { Style::Emphasis };
            let content_start = cursor + marker;
            let Some(relative) = bytes[content_start..]
                .iter()
                .position(|candidate| *candidate == b'*')
            else {
                return Err(Error::UnsupportedMarkup);
            };
            let close = content_start + relative;
            let run = bytes[close..]
                .iter()
                .take_while(|candidate| **candidate == b'*')
                .count();
            if run != marker {
                // Nesting, longer delimiter runs and ambiguous close tokens are
                // intentionally left as regular Preview source, not partially mapped.
                return Err(Error::UnsupportedMarkup);
            }
            (style, content_start..close, close + marker)
        };

        if content.is_empty() {
            return Err(Error::UnsupportedMarkup);
        }
        let visible = &raw[content.clone()];
        if visible.starts_with(' ') || visible.ends_with(' ') {
            return Err(Error::UnsupportedMarkup);
        }
        if style != Style::Code && visible.bytes().any(unsupported_plain) {
            return Err(Error::UnsupportedMarkup);
        }
        runs.push((content, style));
        cursor = close_end;
        plain_start = cursor;
        previous_was_formatted = true;

        if cursor < bytes.len() && (bytes[cursor] == b'*' || bytes[cursor] == b'`') {
            return Err(Error::UnsupportedMarkup);
        }
    }

    if plain_start < bytes.len() {
        runs.push((plain_start..bytes.len(), Style::Plain));
    }
    if runs.is_empty() {
        return Err(Error::Empty);
    }
    Ok(runs)
}

fn append_segment(
    projection: &mut Projection,
    text: &str,
    source: Range<usize>,
    style: Style,
) {
    let visible_start = projection.visible.len();
    if projection.source_offsets.is_empty() {
        projection.source_offsets.push(source.start);
        projection.caret_styles.push(style);
    } else if style != Style::Plain
        || projection.caret_styles[visible_start] == Style::Plain
    {
        // At a delimiter boundary, prefer the formatted run. Inserting at a
        // bold span's visible edge therefore stays inside its source markers.
        projection.source_offsets[visible_start] = source.start;
        projection.caret_styles[visible_start] = style;
    }

    projection.visible.push_str(text);
    for offset in 1..=text.len() {
        projection.source_offsets.push(source.start + offset);
        projection.caret_styles.push(style);
    }
    projection.segments.push(Segment {
        visible: visible_start..projection.visible.len(),
        source,
        style,
    });
}

fn unsupported_plain(byte: u8) -> bool {
    matches!(
        byte,
        b'*' | b'_' | b'~' | b'`' | b'\\' | b'[' | b']' | b'!' | b'<' | b'>' | b'&' | b'$' | b'#'
    )
}

fn safe_insert(ch: char, style: Style) -> bool {
    if !ch.is_ascii() || (ch != ' ' && !ch.is_ascii_graphic()) {
        return false;
    }
    if style == Style::Code {
        return ch != '`';
    }
    !unsupported_plain(ch as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(source: &str, change: TextChange) -> String {
        let mut result = source.to_owned();
        result.replace_range(change.range, &change.replacement);
        result
    }

    #[test]
    fn projection_hides_only_declared_markers_and_maps_visible_bytes_to_source() {
        let source = "left **bold** / *ital* / `code` right\n";
        let payload = 0..source.len() - 1;
        let projection = project(source, payload).unwrap();
        assert_eq!(projection.visible(), "left bold / ital / code right");
        assert!(projection.has_formatted_run());

        let bold = projection.visible().find("bold").unwrap();
        let raw_bold = source.find("bold").unwrap();
        assert_eq!(projection.source_offset_at(bold), Some(raw_bold));
        assert_eq!(projection.source_offset_at(bold + 2), Some(raw_bold + 2));
        assert_eq!(projection.source_offset_at(bold + 4), Some(raw_bold + 4));
        assert_eq!(projection.caret_style_at(bold), Some(Style::Strong));
        assert_eq!(projection.style_at(bold + 1), Some(Style::Strong));

        let italic = projection.visible().find("ital").unwrap();
        let code = projection.visible().find("code").unwrap();
        assert_eq!(projection.style_at(italic), Some(Style::Emphasis));
        assert_eq!(projection.style_at(code), Some(Style::Code));
    }

    #[test]
    fn typing_inside_bold_creates_the_exact_source_byte_delta() {
        let source = "before **bold** after\n";
        let payload = 0..source.len() - 1;
        let projection = project(source, payload.clone()).unwrap();
        let caret = projection.visible().find("bo").unwrap() + 2;
        let mut next_visible = projection.visible().to_owned();
        next_visible.insert(caret, 'X');

        let change = projection
            .plan_edit(&next_visible, caret, caret + 1)
            .unwrap();
        let raw_bold = source.find("bold").unwrap();
        assert_eq!(change.range, raw_bold + 2..raw_bold + 2);
        assert_eq!(change.replacement, "X");
        let changed = apply(source, change);
        assert_eq!(changed, "before **boXld** after\n");

        let updated = project(&changed, payload.start..payload.end + 1).unwrap();
        assert_eq!(updated.visible(), next_visible);
        assert_eq!(updated.style_at(caret), Some(Style::Strong));
    }

    #[test]
    fn deleting_one_bold_glyph_removes_only_its_source_byte() {
        let source = "a **bold** z";
        let projection = project(source, 0..source.len()).unwrap();
        let bold = projection.visible().find("bold").unwrap();
        let mut next_visible = projection.visible().to_owned();
        next_visible.remove(bold + 1);
        let change = projection
            .plan_edit(&next_visible, bold + 2, bold + 1)
            .unwrap();
        let raw_bold = source.find("bold").unwrap();
        assert_eq!(change.range, raw_bold + 1..raw_bold + 2);
        assert_eq!(apply(source, change), "a **bld** z");
    }

    #[test]
    fn repeated_glyph_edits_are_anchored_to_the_native_caret() {
        let source = "**aaaa**";
        let projection = project(source, 0..source.len()).unwrap();

        let caret = 2;
        let mut inserted = projection.visible().to_owned();
        inserted.insert(caret, 'a');
        let change = projection
            .plan_edit(&inserted, caret, caret + 1)
            .unwrap();
        assert_eq!(change.range, 4..4);
        assert_eq!(change.replacement, "a");

        let mut deleted = projection.visible().to_owned();
        deleted.remove(2);
        let change = projection.plan_edit(&deleted, 3, 2).unwrap();
        assert_eq!(change.range, 4..5);
        assert!(change.replacement.is_empty());
    }

    #[test]
    fn formatted_edge_carets_choose_the_formatted_source_side() {
        let source = "x **bold** y";
        let projection = project(source, 0..source.len()).unwrap();
        let bold = projection.visible().find("bold").unwrap();
        assert_eq!(projection.source_offset_at(bold), Some(source.find("bold").unwrap()));
        assert_eq!(projection.caret_style_at(bold), Some(Style::Strong));
        assert_eq!(projection.source_offset_at(bold + 4), Some(source.find("bold").unwrap() + 4));
        assert_eq!(projection.caret_style_at(bold + 4), Some(Style::Strong));
    }

    #[test]
    fn code_character_edits_are_literal_but_markup_characters_are_rejected_elsewhere() {
        let source = "a `xy` z";
        let projection = project(source, 0..source.len()).unwrap();
        let caret = projection.visible().find('y').unwrap();
        let mut next_visible = projection.visible().to_owned();
        next_visible.insert(caret, '*');
        let change = projection
            .plan_edit(&next_visible, caret, caret + 1)
            .unwrap();
        assert_eq!(apply(source, change), "a `x*y` z");

        let source = "a **xy** z";
        let projection = project(source, 0..source.len()).unwrap();
        let caret = projection.visible().find('y').unwrap();
        let mut next_visible = projection.visible().to_owned();
        next_visible.insert(caret, '*');
        assert!(matches!(
            projection.plan_edit(&next_visible, caret, caret + 1),
            Err(Error::UnsupportedCharacter)
        ));
    }

    #[test]
    fn unsupported_or_ambiguous_markdown_stays_out_of_the_projection() {
        for source in [
            "plain &amp; entity",
            "a [link](https://example.invalid)",
            "nested **bold *italic***",
            "unclosed **bold",
            "empty ****",
            "a ~~strike~~ span",
            "a $x$ formula",
            "a \\*escaped\\* marker",
            "a <span> literal",
            "a 😀 glyph",
        ] {
            assert!(project(source, 0..source.len()).is_err(), "{source:?}");
        }
    }

    #[test]
    fn malformed_ranges_lines_controls_and_byte_budget_are_rejected() {
        assert_eq!(project("text", 1..usize::MAX), Err(Error::InvalidRange));
        assert_eq!(project("", 0..0), Err(Error::Empty));
        assert_eq!(project("one\ntwo", 0..7), Err(Error::MultipleLines));
        assert_eq!(project("a\tb", 0..3), Err(Error::ControlCharacter));
        let oversized = "x".repeat(MAX_BYTES + 1);
        assert_eq!(project(&oversized, 0..oversized.len()), Err(Error::TooLarge));
    }

    #[test]
    fn only_one_safe_ascii_insert_or_delete_is_planned() {
        let source = "a **bold** z";
        let projection = project(source, 0..source.len()).unwrap();
        let bold = projection.visible().find("bold").unwrap();
        assert!(projection.can_insert(bold + 2, 'X'));
        assert!(!projection.can_insert(bold + 2, '`'));
        assert!(!projection.can_insert(bold + 2, '🙂'));
        assert!(matches!(
            projection.plan_edit(projection.visible(), bold + 2, bold + 2),
            Err(Error::UnsupportedEdit)
        ));

        let mut multi = projection.visible().to_owned();
        multi.insert_str(bold + 2, "XY");
        assert!(matches!(
            projection.plan_edit(&multi, bold + 2, bold + 4),
            Err(Error::UnsupportedEdit)
        ));
    }
}
