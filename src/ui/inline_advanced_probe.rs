//! Test-only custom Iced widget for the bounded post-Stage-9b hit-test probe.
//! It reports visible-text offsets only and never edits or maps to Source bytes.
use crate::commands::{InlineAltProbeEvent, InlineAltProbePhase, Message};
use crate::inline_edit::{Projection, Style};
use crate::preview_paragraph::Target;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{renderer, Clipboard, Shell};
use iced::{
    alignment, event, font, mouse, Color, Element, Font, Length, Pixels, Point, Rectangle, Size,
    Theme,
};
use std::ops::Range;

const WIDGET_HEIGHT: f32 = 38.0;
const PADDING_X: f32 = 8.0;
const PADDING_Y: f32 = 4.0;
const FONT_SIZE: f32 = 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleRun {
    visible: Range<usize>,
    style: Style,
}

/// State for the isolated, read-only alternative-widget experiment.
pub struct Probe {
    pub target: Target,
    visible: String,
    runs: Vec<VisibleRun>,
    pub events: u64,
    pub dragging: bool,
    pub anchor: Option<usize>,
    pub focus: Option<usize>,
    last_event: Option<InlineAltProbeEvent>,
}

impl Probe {
    pub fn new(target: Target, projection: Projection) -> Option<Self> {
        let visible = projection.visible().to_owned();
        if !projection.has_formatted_run()
            || visible.is_empty()
            || !visible.is_ascii()
            || visible.bytes().any(|byte| matches!(byte, b'\n' | b'\r'))
        {
            return None;
        }

        let mut next = 0;
        let runs: Vec<_> = projection
            .segments()
            .iter()
            .map(|segment| {
                let run = VisibleRun {
                    visible: segment.visible.clone(),
                    style: segment.style,
                };
                next = run.visible.end;
                run
            })
            .collect();
        if runs.is_empty()
            || runs[0].visible.start != 0
            || runs
                .iter()
                .any(|run| run.visible.start >= run.visible.end || run.visible.end > visible.len())
            || runs
                .windows(2)
                .any(|pair| pair[0].visible.end != pair[1].visible.start)
            || next != visible.len()
        {
            return None;
        }

        Some(Self {
            target,
            visible,
            runs,
            events: 0,
            dragging: false,
            anchor: None,
            focus: None,
            last_event: None,
        })
    }

    pub fn visible(&self) -> &str {
        &self.visible
    }

    pub fn observe(&mut self, event: InlineAltProbeEvent) {
        self.events = self.events.saturating_add(1);
        match event.phase {
            InlineAltProbePhase::Hover => {}
            InlineAltProbePhase::Press => {
                self.anchor = event.hit;
                self.focus = event.hit;
                self.dragging = event.hit.is_some();
            }
            InlineAltProbePhase::Drag => {
                if self.dragging {
                    if let Some(hit) = event.hit {
                        self.focus = Some(hit);
                    }
                }
            }
            InlineAltProbePhase::Release => {
                if self.dragging {
                    if let Some(hit) = event.hit {
                        self.focus = Some(hit);
                    }
                }
                self.dragging = false;
            }
        }
        self.last_event = Some(event);
    }

    pub fn title_marker(&self) -> String {
        let anchor = display_offset(self.anchor);
        let focus = display_offset(self.focus);
        let direction = match (self.anchor, self.focus) {
            (Some(anchor), Some(focus)) if anchor < focus => "forward",
            (Some(anchor), Some(focus)) if anchor > focus => "backward",
            _ => "none",
        };
        let Some(event) = self.last_event else {
            return format!(
                "InlineAlt event=0 phase=idle hit=none anchor={anchor} focus={focus} direction=none"
            );
        };
        format!(
            "InlineAlt event={} phase={} x={:.1} y={:.1} local={:.1},{:.1} hit={} anchor={anchor} focus={focus} direction={direction} bounds={:.1},{:.1},{:.1},{:.1} origin={:.1},{:.1} p0={:.1},{:.1} pN={:.1},{:.1} hit_y={:.1}",
            self.events,
            event.phase.label(),
            event.client_x,
            event.client_y,
            event.local_x,
            event.local_y,
            display_offset(event.hit),
            event.bounds_x,
            event.bounds_y,
            event.bounds_width,
            event.bounds_height,
            event.origin_x,
            event.origin_y,
            event.start_x,
            event.start_y,
            event.end_x,
            event.end_y,
            event.hit_y,
        )
    }
}

fn display_offset(offset: Option<usize>) -> String {
    offset.map_or_else(|| "none".to_owned(), |offset| offset.to_string())
}

pub fn widget<'a>(probe: &'a Probe, color: Color) -> Element<'a, Message> {
    Element::new(ParagraphHitWidget {
        visible: &probe.visible,
        runs: &probe.runs,
        color,
    })
}

struct ParagraphHitWidget<'a> {
    visible: &'a str,
    runs: &'a [VisibleRun],
    color: Color,
}

struct ParagraphState<P> {
    paragraph: P,
    origin: Point,
    start: Point,
    end: Point,
    hit_y: f32,
    dragging: bool,
}

impl<'a> ParagraphHitWidget<'a> {
    fn spans(&self) -> Vec<text::Span<'a, (), Font>> {
        self.runs
            .iter()
            .map(|run| {
                let font = match run.style {
                    Style::Plain => Font::DEFAULT,
                    Style::Strong => Font {
                        weight: font::Weight::Bold,
                        ..Font::DEFAULT
                    },
                    Style::Emphasis => Font {
                        style: font::Style::Italic,
                        ..Font::DEFAULT
                    },
                    Style::Code => Font::MONOSPACE,
                };
                text::Span::new(&self.visible[run.visible.clone()]).font(font)
            })
            .collect()
    }
}

impl<'a, Message, Renderer> Widget<Message, Theme, Renderer> for ParagraphHitWidget<'a>
where
    Renderer: text::Renderer<Font = Font>,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(WIDGET_HEIGHT))
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<ParagraphState<Renderer::Paragraph>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(ParagraphState::<Renderer::Paragraph> {
            paragraph: Renderer::Paragraph::default(),
            origin: Point::new(0.0, 0.0),
            start: Point::new(0.0, 0.0),
            end: Point::new(0.0, 0.0),
            hit_y: 0.0,
            dragging: false,
        })
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(
            Length::Fill,
            Length::Fixed(WIDGET_HEIGHT),
            Size::new(320.0, WIDGET_HEIGHT),
        );
        let paragraph_size = Size::new(
            (size.width - PADDING_X * 2.0).max(1.0),
            (size.height - PADDING_Y * 2.0).max(1.0),
        );
        let spans = self.spans();
        let paragraph_text = text::Text {
            content: spans.as_slice(),
            bounds: paragraph_size,
            size: Pixels(FONT_SIZE),
            line_height: text::LineHeight::Relative(1.3),
            font: Font::DEFAULT,
            horizontal_alignment: alignment::Horizontal::Left,
            vertical_alignment: alignment::Vertical::Top,
            shaping: text::Shaping::Basic,
            wrapping: text::Wrapping::None,
        };
        let state = tree
            .state
            .downcast_mut::<ParagraphState<Renderer::Paragraph>>();
        state.paragraph = Renderer::Paragraph::with_spans(paragraph_text);
        state.origin = Point::new(PADDING_X, PADDING_Y);
        state.start = state
            .paragraph
            .grapheme_position(0, 0)
            .unwrap_or(Point::new(0.0, 0.0));
        let minimum = state.paragraph.min_bounds();
        state.end = state
            .paragraph
            .grapheme_position(0, self.visible.len())
            .unwrap_or(Point::new(minimum.width, state.start.y));
        state.hit_y = (minimum.height / 2.0).max(1.0);

        layout::Node::new(size)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree
            .state
            .downcast_ref::<ParagraphState<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        renderer.fill_paragraph(
            &state.paragraph,
            Point::new(bounds.x + state.origin.x, bounds.y + state.origin.y),
            self.color,
            *viewport,
        );
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: event::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let bounds = layout.bounds();
        let state = tree
            .state
            .downcast_mut::<ParagraphState<Renderer::Paragraph>>();
        let (phase, position) = match event {
            event::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(bounds) else {
                    return event::Status::Ignored;
                };
                (InlineAltProbePhase::Press, position)
            }
            event::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let Some(position) = cursor.position() else {
                    return event::Status::Ignored;
                };
                if !state.dragging && !bounds.contains(position) {
                    return event::Status::Ignored;
                }
                let phase = if state.dragging {
                    InlineAltProbePhase::Drag
                } else {
                    InlineAltProbePhase::Hover
                };
                (phase, position)
            }
            event::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if !state.dragging {
                    return event::Status::Ignored;
                }
                let Some(position) = cursor.position() else {
                    state.dragging = false;
                    return event::Status::Captured;
                };
                (InlineAltProbePhase::Release, position)
            }
            _ => return event::Status::Ignored,
        };

        let local_x = position.x - bounds.x;
        let local_y = position.y - bounds.y;
        let paragraph_point = Point::new(local_x - state.origin.x, local_y - state.origin.y);
        let hit = state
            .paragraph
            .hit_test(paragraph_point)
            .map(|hit| hit.cursor());
        let pointer_event = InlineAltProbeEvent {
            phase,
            client_x: position.x,
            client_y: position.y,
            local_x,
            local_y,
            bounds_x: bounds.x,
            bounds_y: bounds.y,
            bounds_width: bounds.width,
            bounds_height: bounds.height,
            origin_x: state.origin.x,
            origin_y: state.origin.y,
            start_x: state.start.x,
            start_y: state.start.y,
            end_x: state.end.x,
            end_y: state.end.y,
            hit_y: state.hit_y,
            hit,
        };

        match phase {
            InlineAltProbePhase::Press => state.dragging = hit.is_some(),
            InlineAltProbePhase::Release => state.dragging = false,
            InlineAltProbePhase::Hover | InlineAltProbePhase::Drag => {}
        }
        shell.publish(Message::InlineAltProbePointer(pointer_event));
        if matches!(phase, InlineAltProbePhase::Hover) {
            event::Status::Ignored
        } else {
            event::Status::Captured
        }
    }

    fn mouse_interaction(
        &self,
        _state: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Snapshot;
    use crate::preview_paragraph::Target;

    fn target() -> Target {
        Target {
            snapshot: Snapshot::default(),
            block: 0..16,
            payload: 0..15,
        }
    }

    fn projection() -> Projection {
        crate::inline_edit::project("ab**CDEFGH**ijk\n", 0..15).unwrap()
    }

    fn pointer(phase: InlineAltProbePhase, hit: Option<usize>) -> InlineAltProbeEvent {
        InlineAltProbeEvent {
            phase,
            client_x: 100.0,
            client_y: 200.0,
            local_x: 20.0,
            local_y: 12.0,
            bounds_x: 80.0,
            bounds_y: 188.0,
            bounds_width: 500.0,
            bounds_height: WIDGET_HEIGHT,
            origin_x: PADDING_X,
            origin_y: PADDING_Y,
            start_x: 0.0,
            start_y: 0.0,
            end_x: 80.0,
            end_y: 0.0,
            hit_y: 10.0,
            hit,
        }
    }

    #[test]
    fn probe_copies_visible_styles_without_retaining_source_offsets() {
        let probe = Probe::new(target(), projection()).unwrap();
        assert_eq!(probe.visible(), "abCDEFGHijk");
        assert_eq!(probe.runs.len(), 3);
        assert_eq!(probe.runs[0].style, Style::Plain);
        assert_eq!(probe.runs[1].style, Style::Strong);
        assert_eq!(probe.runs[2].style, Style::Plain);
        assert_eq!(probe.runs[0].visible, 0..2);
        assert_eq!(probe.runs[1].visible, 2..8);
        assert_eq!(probe.runs[2].visible, 8..11);
    }

    #[test]
    fn probe_keeps_offsets_in_visible_text_and_tracks_both_drag_directions() {
        let mut forward = Probe::new(target(), projection()).unwrap();
        forward.observe(pointer(InlineAltProbePhase::Press, Some(3)));
        forward.observe(pointer(InlineAltProbePhase::Drag, Some(7)));
        forward.observe(pointer(InlineAltProbePhase::Release, Some(7)));
        assert_eq!(forward.anchor, Some(3));
        assert_eq!(forward.focus, Some(7));
        assert!(!forward.dragging);
        assert!(forward.title_marker().contains("direction=forward"));

        let mut backward = Probe::new(target(), projection()).unwrap();
        backward.observe(pointer(InlineAltProbePhase::Press, Some(7)));
        backward.observe(pointer(InlineAltProbePhase::Drag, Some(3)));
        backward.observe(pointer(InlineAltProbePhase::Release, Some(3)));
        assert_eq!(backward.anchor, Some(7));
        assert_eq!(backward.focus, Some(3));
        assert!(backward.title_marker().contains("direction=backward"));
    }

    #[test]
    fn hover_does_not_change_anchor_or_start_drag_and_plain_text_is_rejected() {
        let mut probe = Probe::new(target(), projection()).unwrap();
        probe.observe(pointer(InlineAltProbePhase::Hover, Some(4)));
        assert_eq!(probe.events, 1);
        assert!(probe.anchor.is_none() && probe.focus.is_none() && !probe.dragging);
        let plain = crate::inline_edit::project("plain\n", 0..5).unwrap();
        assert!(Probe::new(target(), plain).is_none());
    }
}
