//! Direct-lane launch through the same constructors the production CLI uses.

use junie_tui::{ColorLevel, Theme};
use junie_tui_testing::Harness;
use tablepro_app::TableProApp;

use crate::scenario::ColorProfile;

/// Default oracle theme resolved for `level`, matching `run_with`'s Junie choice.
#[must_use]
pub fn theme_for(level: ColorLevel) -> Theme {
    Theme::junie().for_level(level)
}

/// Preset C: no CLI arguments, so `run_with(theme, None)` → `TableProApp::default()`.
///
/// This is the production Connections landing path. It does not call
/// `set_surface`.
#[must_use]
pub fn launch_preset_c(width: u16, height: u16, profile: ColorProfile) -> Harness<TableProApp> {
    let level = profile.color_level();
    let app = TableProApp::default();
    Harness::new(app, theme_for(level), width, height).with_color(level)
}
