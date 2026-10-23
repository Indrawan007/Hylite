mod app;
mod block_edit;
mod commands;
mod config;
mod continuation;
mod document;
mod editor;
mod export;
mod export_cli;
mod file_io;
mod files;
mod highlighting;
mod hyprland;
mod images;
mod line_edit;
mod multiline_draft_history;
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
mod workspace;

fn main() -> iced::Result {
    // `hylite export-pdf <input.md> <output.pdf>` renders without a window, so
    // a script (and the X11 gate) can produce a PDF headlessly.
    // Collect once: `ArgsOs` is not cloneable and both paths read the same list.
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if export_cli::is_export_pdf(arguments.first()) {
        let theme = config::Config::load().config.appearance.theme.iced();
        let palette = export_cli::theme_palette(&theme);
        std::process::exit(export_cli::run(arguments, palette));
    }
    // `hylite <folder>` opens a workspace, `hylite <file.md>` opens that file.
    let launch = arguments.first().map(std::path::PathBuf::from);
    iced::application(app::Hylite::title, app::Hylite::update, app::Hylite::view)
        .theme(app::Hylite::theme)
        .subscription(app::Hylite::subscription)
        .window_size((1200.0, 800.0))
        .exit_on_close_request(false)
        .run_with(move || app::Hylite::with_launch_path(launch))
}
