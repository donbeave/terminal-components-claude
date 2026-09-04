//! Jackin Preview application package.
//!
//! Product screens and deterministic fixtures stay in this package; generic
//! terminal primitives come from the single public compatibility facade.
#![forbid(unsafe_code)]
#![allow(
    elided_lifetimes_in_paths,
    missing_debug_implementations,
    unused_qualifications,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    reason = "the compatibility facade mirrors the legacy app surface"
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
pub extern crate tui_next as legacy_facade;
pub extern crate tui_next_public as public_tui;

extern crate self as junie_tui;

pub use legacy_facade::core;
pub use legacy_facade::ratatui;
pub use legacy_facade::runtime;
pub use legacy_facade::theme;
pub use legacy_facade::ui;
pub use legacy_facade::widgets;

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

/// Run the interactive preview through the legacy facade.
pub fn run() -> std::io::Result<()> {
    run_scenario(Scenario::FirstUse, Motion::Full, 0)
}

/// Run a pinned scenario through the legacy facade.
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

    fn render(&mut self, frame: &mut crate::ratatui::Frame<'_>) {
        App::render(self, frame);
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn tick_interval(&self) -> std::time::Duration {
        App::tick_interval(self)
    }
}
