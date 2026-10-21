mod app;
mod config;
mod document;
mod export;
mod hyprland;
mod ui;

fn main() -> iced::Result {
    iced::application(app::Hylite::title, app::Hylite::update, app::Hylite::view)
        .theme(app::Hylite::theme)
        .subscription(app::Hylite::subscription)
        .window_size((1200.0, 800.0))
        .run_with(app::Hylite::new)
}
