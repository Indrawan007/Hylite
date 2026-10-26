mod code_context;
pub(crate) mod history;
mod preview;
mod preview_incremental;
pub(crate) mod preview_model;
mod snapshot;
mod state;

pub use preview::{
    render_inline_alt_probe_preview, render_inline_probe_preview, render_inline_spike_preview,
    render_preview, render_preview_with_inline_text,
};
pub use snapshot::Snapshot;
pub use state::Document;

pub(crate) use state::{buffer_text, content_at, paste_exact, SingleMovePlan};

/// Source-order navigation between authenticated eligible multiline roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphDirection {
    Previous,
    Next,
}
