//! Shared input limits, so a policy change is one edit instead of seven.
//!
//! Two concepts happen to share the same number today, but they keep separate
//! names because they can move independently: one bounds a raw draft payload,
//! the other bounds a single source line scanned by the lexer and Enter rules.

/// Largest accepted raw Markdown payload for one Preview draft target.
pub const DRAFT_MAX_BYTES: usize = 16 * 1024;
/// Largest accepted single source line for lexical scans and Enter transactions.
pub const LINE_MAX_BYTES: usize = 16 * 1024;
