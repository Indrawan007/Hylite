//! Standalone harness for production modules that do not depend on Iced.
//! Run with scripts/test-source-core.sh; this is not the full Cargo suite.
#[path = "../src/block_edit.rs"]
mod block_edit;
#[path = "../src/document/snapshot.rs"]
mod document;
#[path = "../src/document/history.rs"]
mod history;
#[path = "../src/operations.rs"]
mod operations;
#[path = "../src/search.rs"]
mod search;
#[path = "../src/text_edit.rs"]
mod text_edit;

#[path = "../src/syntax.rs"]
mod syntax;

#[path = "../src/continuation.rs"]
mod continuation;

#[path = "../src/navigation.rs"]
mod navigation;

#[path = "../src/line_edit.rs"]
mod line_edit;

#[path = "../src/preview_navigation.rs"]
mod preview_navigation;
