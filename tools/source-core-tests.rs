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

#[path = "../src/preview_task.rs"]
mod preview_task;

#[path = "../src/preview_heading.rs"]
mod preview_heading;

#[path = "../src/preview_setext.rs"]
mod preview_setext;

#[path = "../src/preview_cell.rs"]
mod preview_cell;
#[path = "../src/preview_table_edit.rs"]
mod preview_table_edit;
#[path = "../src/preview_table_row.rs"]
mod preview_table_row;

#[path = "../src/preview_table_column.rs"]
mod preview_table_column;

#[path = "../src/preview_paragraph.rs"]
mod preview_paragraph;

#[path = "../src/preview_paragraph_multiline.rs"]
mod preview_paragraph_multiline;

#[path = "../src/multiline_draft_history.rs"]
mod multiline_draft_history;

#[path = "../src/workspace.rs"]
mod workspace;
