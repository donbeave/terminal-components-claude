//! Jackin, redesigned: an interactive deterministic terminal preview.
#![deny(unsafe_code)]

mod app;
#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod app_tests_chrome;
mod arbiter;
mod clock;
mod domain;
#[cfg(test)]
mod perf_tests;
mod rain;
mod scenario;
mod screens;
mod sim;
#[cfg(test)]
mod visual_tests;

pub use app::{App, Route};
pub use scenario::{Motion, Scenario};

impl junie_tui::runtime::Application for App {
    fn handle(&mut self, input: junie_tui::core::event::Input) -> junie_tui::core::event::Outcome {
        App::handle(self, input)
    }

    fn render(&mut self, frame: &mut ratatui::Frame<'_>) {
        App::render(self, frame);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_interval(&self) -> std::time::Duration {
        App::tick_interval(self)
    }
}

/// Run the interactive preview using the public junie-tui runtime.
pub fn run() -> std::io::Result<()> {
    use junie_tui::theme::{ColorLevel, Theme, ThemeKind};

    let theme = Theme::for_theme(ThemeKind::Junie, ColorLevel::TrueColor);
    let mut app = App::for_scenario(Scenario::FirstUse, Motion::Full, 0, theme);
    junie_tui::runtime::run(&mut app)
}
