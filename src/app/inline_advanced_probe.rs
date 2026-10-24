//! Controller for the isolated, read-only alternative-widget feasibility probe.
use super::Hylite;
use crate::commands::{InlineAltProbeEvent, Message};
use crate::document::preview_model::Kind;
use crate::preview_paragraph::Target;
use iced::Task;

impl Hylite {
    pub(super) fn open_first_inline_alt_probe(&mut self) -> Task<Message> {
        if !self.inline_alt_probe_enabled {
            return Task::none();
        }
        let source = self.document.buffer_text();
        let snapshot = self.document.snapshot();
        let target = self
            .document
            .preview_blocks()
            .iter()
            .find_map(|block| super::inline_spike::target_for_block(block, &source, snapshot));
        let Some(target) = target else {
            self.status_message =
                "No supported single-line formatted paragraph for the alternative probe".into();
            return Task::none();
        };
        self.open_inline_alt_probe(target)
    }

    pub(super) fn open_inline_alt_probe(&mut self, target: Target) -> Task<Message> {
        if !self.inline_alt_probe_enabled
            || self.inline_alt_probe.is_some()
            || self.inline_spike.is_some()
            || self.inline_probe.is_some()
            || self.mode == crate::editor::EditMode::Source
            || !self.navigation_allowed()
            || self.operations.active().is_some()
            || self.paragraph_draft.is_some()
            || self.multiline_draft.is_some()
            || self.cell_draft.is_some()
        {
            self.status_message =
                "Alternative inline probe is unavailable in the current editor state".into();
            return Task::none();
        }

        let source = self.document.buffer_text();
        let snapshot = self.document.snapshot();
        let projection = self
            .document
            .preview_blocks()
            .iter()
            .find(|block| block.source == target.block && matches!(block.kind, Kind::Paragraph))
            .and_then(|block| {
                super::inline_spike::authenticated_projection(block, &source, &target, snapshot)
            });
        let Some(projection) = projection else {
            self.status_message =
                "Alternative probe target is stale or outside the bounded inline subset; Source kept"
                    .into();
            return Task::none();
        };
        let Some(probe) = crate::ui::inline_advanced_probe::Probe::new(target, projection) else {
            self.status_message =
                "Alternative probe could not initialize; Source kept byte for byte".into();
            return Task::none();
        };
        self.inline_alt_probe = Some(probe);
        self.status_message =
            "Alternative custom-widget probe is read-only and test-only; Esc finishes".into();
        Task::none()
    }

    pub(super) fn inline_alt_probe_pointer(&mut self, event: InlineAltProbeEvent) -> Task<Message> {
        let snapshot = self.document.snapshot();
        let Some(probe) = self.inline_alt_probe.as_mut() else {
            return Task::none();
        };
        if probe.target.snapshot != snapshot {
            self.inline_alt_probe = None;
            self.inline_alt_probe_enabled = false;
            self.status_message =
                "Alternative probe target became stale; probe stopped without editing Source"
                    .into();
            return Task::none();
        }
        let valid_geometry = [
            event.client_x,
            event.client_y,
            event.local_x,
            event.local_y,
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
        ]
        .into_iter()
        .all(f32::is_finite);
        if !valid_geometry || event.hit.is_some_and(|hit| hit > probe.visible().len()) {
            self.inline_alt_probe = None;
            self.inline_alt_probe_enabled = false;
            self.status_message =
                "Alternative probe received invalid geometry; it stopped read-only".into();
            return Task::none();
        }
        probe.observe(event);
        Task::none()
    }

    pub(super) fn finish_inline_alt_probe(&mut self) -> Task<Message> {
        if self.inline_alt_probe.take().is_some() {
            self.inline_alt_probe_enabled = false;
            self.inline_alt_probe_pending = false;
            self.status_message =
                "Alternative custom-widget probe finished; Source was not edited".into();
        }
        Task::none()
    }
}
