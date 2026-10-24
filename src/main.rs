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
mod inline_edit;
mod line_edit;
mod math;
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
    // The two explicit experimental flags never alter the default Preview or
    // replace any existing draft workflow; Stage 9b is strictly read-only.
    let inline_spike =
        arguments.first().and_then(|argument| argument.to_str()) == Some("--inline-edit-spike");
    let inline_probe = arguments.first().and_then(|argument| argument.to_str())
        == Some("--inline-feasibility-probe");
    let inline_experiment = inline_spike || inline_probe;
    if inline_experiment && arguments.len() != 2 {
        if inline_probe {
            eprintln!("usage: hylite --inline-feasibility-probe <file>");
        } else {
            eprintln!("usage: hylite --inline-edit-spike <file>");
        }
        std::process::exit(2);
    }
    let launch = if inline_experiment {
        Some(std::path::PathBuf::from(&arguments[1]))
    } else {
        arguments.first().map(std::path::PathBuf::from)
    };
    if inline_experiment && !launch.as_ref().is_some_and(|path| path.is_file()) {
        if inline_probe {
            eprintln!("--inline-feasibility-probe requires one existing document file");
        } else {
            eprintln!("--inline-edit-spike requires one existing document file");
        }
        std::process::exit(2);
    }
    iced::application(app::Hylite::title, app::Hylite::update, app::Hylite::view)
        .theme(app::Hylite::theme)
        .subscription(app::Hylite::subscription)
        .window_size((1200.0, 800.0))
        .exit_on_close_request(false)
        .run_with(move || {
            if inline_spike {
                app::Hylite::with_inline_spike_path(launch.expect("validated Stage 9a path"))
            } else if inline_probe {
                app::Hylite::with_inline_probe_path(launch.expect("validated Stage 9b path"))
            } else {
                app::Hylite::with_launch_path(launch)
            }
        })
}
