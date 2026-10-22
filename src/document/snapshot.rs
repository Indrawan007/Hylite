//! Identity of a document revision, independent of its editor/widget state.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Snapshot {
    pub document_id: u64,
    pub revision: u64,
    /// Logical content checkpoint; undo/redo can revisit it while revision grows.
    pub state_id: u64,
}
