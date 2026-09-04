//! Jackin Preview: a deterministic terminal app built on `tui-next`.
#![forbid(unsafe_code)]
#![allow(
    elided_lifetimes_in_paths,
    missing_debug_implementations,
    unused_qualifications,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used
)]
#![expect(
    missing_docs,
    reason = "the preview's public fixture model is intentionally data-shaped"
)]
#![expect(
    unreachable_pub,
    reason = "fixture modules are public to integration tests"
)]
#![expect(
    clippy::pedantic,
    reason = "fixture and rendering code favors explicit deterministic data"
)]
#![expect(
    clippy::arithmetic_side_effects,
    reason = "all arithmetic is over bounded deterministic fixture values"
)]
// Keep the proven Slice-7 shell intact inside the app.  The app's external
// dependency remains the public `tui-next` boundary; this self alias lets the
// port retain its documented `junie_tui::…` module paths during migration.
extern crate self as junie_tui;

pub mod core;
pub mod runtime;
pub mod theme;
pub mod ui;
pub mod widgets;

mod app;
mod arbiter;
mod clock;
pub mod domain;
pub mod rain;
mod scenario;
pub mod screens;
pub mod sim;

pub use app::{App, Route};
pub use scenario::{Motion, Scenario};

/// Run the interactive preview through the public `tui-next` entry point.
pub fn run() -> std::io::Result<()> {
    run_scenario(Scenario::FirstUse, Motion::Full, 0)
}

/// Run a pinned scenario through the public `tui-next` entry point.
pub fn run_scenario(scenario: Scenario, motion: Motion, frame: u64) -> std::io::Result<()> {
    let theme = theme::Theme::for_theme(theme::ThemeKind::Junie, theme::ColorLevel::detect());
    let mut app = App::for_scenario_with_theme(scenario, motion, frame, theme);
    let _ = runtime::drain_pending_input();
    runtime::run(&mut app)
}

impl runtime::Application for App {
    fn handle(&mut self, input: core::event::Input) -> core::event::Outcome {
        App::handle(self, input)
    }

    fn render(&mut self, frame: &mut ratatui::Frame<'_>) {
        App::render(self, frame)
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_interval(&self) -> std::time::Duration {
        App::tick_interval(self)
    }
}
