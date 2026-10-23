//! Controller for the opt-in Stage 9a inline-edit experiment.
use super::Hylite;
use crate::commands::Message;
use crate::document::preview_model::{self, Block, Kind};
use crate::document::Snapshot;
use crate::inline_edit::{self, Projection, Style};
use crate::preview_paragraph::Target;
use iced::widget::text_editor::{Action, Edit};
use iced::Task;

impl Hylite {
    pub(super) fn open_first_inline_spike(&mut self) -> Task<Message> {
        if !self.inline_spike_enabled {
            return Task::none();
        }
        let source = self.document.buffer_text();
        let snapshot = self.document.snapshot();
        let target = self
            .document
            .preview_blocks()
            .iter()
            .find_map(|block| target_for_block(block, &source, snapshot));
        let Some(target) = target else {
            self.status_message =
                "No supported single-line formatted paragraph for the Stage 9a spike".into();
            return Task::none();
        };
        self.open_inline_spike(target)
    }

    pub(super) fn open_inline_spike(&mut self, target: Target) -> Task<Message> {
        if !self.inline_spike_enabled
            || self.inline_spike.is_some()
            || self.mode == crate::editor::EditMode::Source
            || !self.navigation_allowed()
            || self.operations.active().is_some()
            || self.paragraph_draft.is_some()
            || self.multiline_draft.is_some()
            || self.cell_draft.is_some()
        {
            self.status_message =
                "Stage 9a spike is unavailable in the current editor state".into();
            return Task::none();
        }
        let source = self.document.buffer_text();
        let snapshot = self.document.snapshot();
        let projection = self
            .document
            .preview_blocks()
            .iter()
            .find(|block| block.source == target.block && matches!(block.kind, Kind::Paragraph))
            .and_then(|block| authenticated_projection(block, &source, &target, snapshot));
        let Some(projection) = projection else {
            self.status_message =
                "Stage 9a target is stale or outside the bounded inline subset; Source kept".into();
            return Task::none();
        };
        let Some(draft) = crate::ui::inline_spike::Draft::new(target, projection) else {
            self.status_message =
                "Stage 9a could not initialize the native caret; Source kept".into();
            return Task::none();
        };
        self.document.break_undo_group();
        self.inline_spike = Some(draft);
        self.status_message = "Stage 9a: ASCII edits update Source live; Esc finishes".into();
        crate::source_focus::task_in(crate::ui::inline_spike::input_id())
    }

    pub(super) fn inline_spike_action(&mut self, action: Action) -> Task<Message> {
        let snapshot = self.document.snapshot();
        if self
            .inline_spike
            .as_ref()
            .is_some_and(|draft| draft.target.snapshot != snapshot)
        {
            self.inline_spike = None;
            self.status_message = "Stage 9a target became stale; inline editing stopped".into();
            return Task::none();
        }
        let Some(draft) = self.inline_spike.as_mut() else {
            return Task::none();
        };
        if draft.content.selection().is_some() {
            self.status_message =
                "Stage 9a only verifies a caret; collapse the selection before editing".into();
            return Task::none();
        }

        if matches!(
            &action,
            Action::Move(_) | Action::Click(_) | Action::Scroll { .. }
        ) {
            let previous = draft.value();
            let Some(caret) = draft.caret() else {
                self.status_message = "Stage 9a caret has no verified byte position".into();
                return Task::none();
            };
            draft.content.perform(action);
            if draft.caret().is_none() || draft.content.selection().is_some() {
                if let Some(content) = crate::document::content_at(&previous, caret) {
                    draft.content = content;
                }
                self.status_message =
                    "Stage 9a rejected a selection or unmapped caret movement".into();
            }
            return Task::none();
        }

        match &action {
            Action::Edit(Edit::Insert(ch)) => {
                let Some(caret) = draft.caret() else {
                    self.status_message = "Stage 9a caret has no verified byte position".into();
                    return Task::none();
                };
                if !draft.projection.can_insert(caret, *ch) {
                    self.status_message = format!(
                        "Stage 9a rejected that character: {}; Source unchanged",
                        inline_edit::Error::UnsupportedCharacter.label()
                    );
                    return Task::none();
                }
            }
            Action::Edit(Edit::Backspace | Edit::Delete) => {}
            _ => {
                self.status_message =
                    "Stage 9a allows caret motion and one safe character insert/delete only".into();
                return Task::none();
            }
        }

        let before = draft.value();
        let Some(before_caret) = draft.caret() else {
            self.status_message = "Stage 9a caret has no verified byte position".into();
            return Task::none();
        };
        let target = draft.target.clone();
        let old_projection = draft.projection.clone();
        draft.content.perform(action);
        let after = draft.value();
        let after_caret = draft.caret();
        if before == after {
            return Task::none();
        }
        let Some(after_caret) = after_caret else {
            self.restore_inline_spike(&before, before_caret);
            self.status_message = "Stage 9a caret has no verified byte position".into();
            return Task::none();
        };

        let change = match old_projection.plan_edit(&after, before_caret, after_caret) {
            Ok(change) => change,
            Err(error) => {
                self.restore_inline_spike(&before, before_caret);
                self.status_message = format!(
                    "Stage 9a rejected edit: {}; Source unchanged",
                    error.label()
                );
                return Task::none();
            }
        };

        let source = self.document.buffer_text();
        let current_projection = self
            .document
            .preview_blocks()
            .iter()
            .find(|block| block.source == target.block && matches!(block.kind, Kind::Paragraph))
            .and_then(|block| authenticated_projection(block, &source, &target, snapshot));
        if current_projection.as_ref() != Some(&old_projection) {
            self.restore_inline_spike(&before, before_caret);
            self.status_message = "Stage 9a target changed; edit refused and Source kept".into();
            return Task::none();
        }

        let mut candidate = source.clone();
        candidate.replace_range(change.range.clone(), &change.replacement);
        let next = projection_at_root(&candidate, target.block.start);
        let Some((block_range, payload, projection)) = next else {
            self.restore_inline_spike(&before, before_caret);
            self.status_message =
                "Stage 9a edit would change the root paragraph structure; Source kept".into();
            return Task::none();
        };
        if projection.visible() != after {
            self.restore_inline_spike(&before, before_caret);
            self.status_message =
                "Stage 9a edit no longer matches Preview text; Source kept".into();
            return Task::none();
        }

        let changed_range = change.range.clone();
        if !self.document.change(change) {
            self.restore_inline_spike(&before, before_caret);
            self.status_message = "Stage 9a Source transaction failed; document kept".into();
            return Task::none();
        }

        let target = Target {
            snapshot: self.document.snapshot(),
            block: block_range,
            payload,
        };
        if let Some(draft) = self.inline_spike.as_mut() {
            draft.replace_projection(target, projection);
        }
        self.edited();
        if !self.document.history_limited() {
            self.status_message = format!(
                "Stage 9a committed Source bytes {}..{}",
                changed_range.start, changed_range.end
            );
        }
        Task::none()
    }

    pub(super) fn finish_inline_spike(&mut self) -> Task<Message> {
        if self.inline_spike.take().is_some() {
            self.document.break_undo_group();
            self.status_message =
                "Stage 9a spike finished; live Source edits remain in document history".into();
        }
        self.restore_input_focus()
    }

    fn restore_inline_spike(&mut self, value: &str, caret: usize) {
        if let (Some(draft), Some(content)) = (
            self.inline_spike.as_mut(),
            crate::document::content_at(value, caret),
        ) {
            draft.content = content;
        }
    }
}

fn target_for_block(block: &Block, source: &str, snapshot: Snapshot) -> Option<Target> {
    if !matches!(block.kind, Kind::Paragraph) {
        return None;
    }
    let payload = block.paragraph_payload.clone()?;
    if crate::preview_paragraph::payload_range(source, &block.source).as_ref() != Some(&payload) {
        return None;
    }
    let projection = inline_edit::project(source, payload.clone()).ok()?;
    if !projection.has_formatted_run() || !matches_preview_runs(block, &projection) {
        return None;
    }
    Some(Target {
        snapshot,
        block: block.source.clone(),
        payload,
    })
}

fn authenticated_projection(
    block: &Block,
    source: &str,
    target: &Target,
    snapshot: Snapshot,
) -> Option<Projection> {
    if target.snapshot != snapshot
        || block.source != target.block
        || !matches!(block.kind, Kind::Paragraph)
    {
        return None;
    }
    let payload = block.paragraph_payload.as_ref()?;
    if payload != &target.payload
        || crate::preview_paragraph::payload_range(source, &block.source).as_ref() != Some(payload)
    {
        return None;
    }
    let projection = inline_edit::project(source, payload.clone()).ok()?;
    (projection.has_formatted_run() && matches_preview_runs(block, &projection))
        .then_some(projection)
}

fn projection_at_root(
    source: &str,
    block_start: usize,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>, Projection)> {
    let blocks = preview_model::parse(source);
    let block = blocks
        .iter()
        .find(|block| block.source.start == block_start && matches!(block.kind, Kind::Paragraph))?;
    let target = target_for_block(block, source, Snapshot::default())?;
    let projection = authenticated_projection(block, source, &target, Snapshot::default())?;
    Some((block.source.clone(), target.payload, projection))
}

fn matches_preview_runs(block: &Block, projection: &Projection) -> bool {
    let mut visible = String::new();
    let mut styles = Vec::new();
    for run in &block.runs {
        if run.math.is_some()
            || run.image.is_some()
            || run.style.link.is_some()
            || run.style.strike
            || !run.text.is_ascii()
        {
            return false;
        }
        let style = match (run.style.strong, run.style.emphasis, run.style.code) {
            (false, false, false) => Style::Plain,
            (true, false, false) => Style::Strong,
            (false, true, false) => Style::Emphasis,
            (false, false, true) => Style::Code,
            _ => return false,
        };
        visible.push_str(&run.text);
        styles.extend(std::iter::repeat_n(style, run.text.len()));
    }
    visible == projection.visible()
        && styles.len() == projection.visible().len()
        && styles
            .iter()
            .enumerate()
            .all(|(byte, style)| projection.style_at(byte) == Some(*style))
}
