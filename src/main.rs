mod app;
mod block_edit;
mod commands;
mod config;
mod continuation;
mod document;
mod editor;
mod export;
mod file_io;
mod files;
mod highlighting;
mod hyprland;
mod line_edit;
mod navigation;
mod operations;
mod preview_cell;
mod preview_heading;
mod preview_navigation;
mod preview_paragraph;
mod preview_paragraph_multiline;
mod preview_setext;
mod preview_table_column;
mod preview_table_edit;
mod preview_table_row;
mod preview_task;
mod search;
mod source_focus;
mod syntax;
mod text_edit;
mod ui;

fn main() -> iced::Result {
    iced::application(app::Hylite::title, app::Hylite::update, app::Hylite::view)
        .theme(app::Hylite::theme)
        .subscription(app::Hylite::subscription)
        .window_size((1200.0, 800.0))
        .exit_on_close_request(false)
        .run_with(app::Hylite::new)
}
