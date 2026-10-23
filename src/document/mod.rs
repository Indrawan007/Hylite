mod code_context;
pub(crate) mod history;
mod preview;
mod preview_model;
mod snapshot;
mod state;

pub use preview::render_preview;
pub use snapshot::Snapshot;
pub use state::Document;

pub(crate) use state::{buffer_text, content_at, paste_exact, SingleMovePlan};

/// Source-order navigation between authenticated eligible multiline roots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParagraphDirection {
    Previous,
    Next,
}
