mod app;
mod block_edit;
mod commands;
mod config;
mod continuation;
mod document;
mod editor;
mod environment_report;
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
#[cfg(test)]
mod performance_benchmarks;
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
mod preview_visual;
mod search;
mod source_focus;
mod syntax;
mod text_edit;
mod ui;
mod workspace;

/// Opt-in experimental harnesses. None of them changes the default Preview.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Experiment {
    Stage9a,
    Stage9b,
    AltWidget,
}

impl Experiment {
    fn from_flag(flag: Option<&str>) -> Option<Self> {
        match flag {
            Some("--inline-edit-spike") => Some(Self::Stage9a),
            Some("--inline-feasibility-probe") => Some(Self::Stage9b),
            Some("--inline-alt-widget-probe") => Some(Self::AltWidget),
            _ => None,
        }
    }

    fn usage(self) -> &'static str {
        match self {
            Self::Stage9a => "usage: hylite --inline-edit-spike <file>",
            Self::Stage9b => "usage: hylite --inline-feasibility-probe <file>",
            Self::AltWidget => "usage: hylite --inline-alt-widget-probe <file>",
        }
    }

    fn missing_file(self) -> &'static str {
        match self {
            Self::Stage9a => "--inline-edit-spike requires one existing document file",
            Self::Stage9b => "--inline-feasibility-probe requires one existing document file",
            Self::AltWidget => "--inline-alt-widget-probe requires one existing document file",
        }
    }
}

/// `Ok(None)` means no experiment: the first argument keeps its normal
/// file-or-folder meaning. Any other outcome is a usage error message.
fn parse_experiment(args: &[std::ffi::OsString]) -> Result<Option<Experiment>, &'static str> {
    let flag = args.first().and_then(|argument| argument.to_str());
    let Some(experiment) = Experiment::from_flag(flag) else {
        return Ok(None);
    };
    if args.len() != 2 {
        return Err(experiment.usage());
    }
    if !std::path::Path::new(&args[1]).is_file() {
        return Err(experiment.missing_file());
    }
    Ok(Some(experiment))
}

fn main() -> iced::Result {
    // `hylite export-pdf <input.md> <output.pdf>` renders without a window, so
    // a script (and the X11 gate) can produce a PDF headlessly.
    // Collect once: `ArgsOs` is not cloneable and both paths read the same list.
    let arguments: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    if export_cli::is_export_pdf(arguments.first()) {
        let theme = config::Config::load().config.appearance.theme.iced();
        let palette = export::theme_palette(&theme);
        std::process::exit(export_cli::run(arguments, palette));
    }
    // `hylite <folder>` opens a workspace, `hylite <file.md>` opens that file.
    // An explicit experimental flag replaces that argument with one file.
    let experiment = match parse_experiment(&arguments) {
        Ok(experiment) => experiment,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    let launch = if experiment.is_some() {
        Some(std::path::PathBuf::from(&arguments[1]))
    } else {
        arguments.first().map(std::path::PathBuf::from)
    };
    let inline_spike = experiment == Some(Experiment::Stage9a);
    let inline_probe = experiment == Some(Experiment::Stage9b);
    let inline_alt_probe = experiment == Some(Experiment::AltWidget);
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
            } else if inline_alt_probe {
                app::Hylite::with_inline_alt_probe_path(
                    launch.expect("validated alternative probe path"),
                )
            } else {
                app::Hylite::with_launch_path(launch)
            }
        })
}
