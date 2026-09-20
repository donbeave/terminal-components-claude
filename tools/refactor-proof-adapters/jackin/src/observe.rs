//! Production-view session: `App::for_scenario` plus runtime draw.

use std::time::Duration;

use jackin_app::{App, Motion, Route, Scenario};
use junie_tui::{ColorLevel, Theme};
use junie_tui_testing::Harness;

use crate::frame::{ObservedCell, ObservedCursor, ObservedFrame};

/// Atmosphere seed owned by the production rain renderer.
pub const MOTION_SEED: u64 = jackin_app::rain::MOTION_SEED;
/// Fixture epoch used by the production clock (`2026-09-03 09:14:00` UTC+7).
pub const EPOCH_SECS: i64 = 1_788_401_640;
/// Helper tick used by `apps/jackin-preview/tests/support.rs` `H::ticks`.
pub const HELPER_TICK_MS: u64 = 200;
/// Viewport at which `App::draw` currently overpaints selected routes.
///
/// JA-001's first leaf is 80×24 so that path is not taken. The adapter still
/// always calls production `App::draw`; it never invokes the historical
/// painter itself and never treats that overpaint as an expected artifact.
pub const HISTORICAL_PAINT_SIZE: (u16, u16) = (120, 40);

/// Terminal size for a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
}

/// Direct production-view session over the public runtime harness.
#[derive(Debug)]
pub struct DirectSession {
    scenario: Scenario,
    motion: Motion,
    construct_frame: u64,
    color: ColorLevel,
    harness: Harness<App>,
}

impl DirectSession {
    /// Construct `App::for_scenario(scenario, Paused)` and draw at 80×24 truecolor.
    #[must_use]
    pub fn ja001_first_leaf(scenario: Scenario) -> Self {
        Self::paused_frame0(
            scenario,
            Viewport {
                width: crate::JA001_FIRST_LEAF_WIDTH,
                height: crate::JA001_FIRST_LEAF_HEIGHT,
            },
            ColorLevel::TrueColor,
        )
    }

    /// `fresh(world, Paused, 0); draw` through production `App::for_scenario`.
    #[must_use]
    pub fn paused_frame0(scenario: Scenario, viewport: Viewport, color: ColorLevel) -> Self {
        let app = App::for_scenario(scenario, Motion::Paused);
        Self::from_app(app, scenario, Motion::Paused, 0, viewport, color)
    }

    fn from_app(
        app: App,
        scenario: Scenario,
        motion: Motion,
        construct_frame: u64,
        viewport: Viewport,
        color: ColorLevel,
    ) -> Self {
        let mut harness =
            Harness::new(app, Theme::junie(), viewport.width, viewport.height).with_color(color);
        // Harness::new already presents once; an extra production draw is the
        // explicit `draw` in `fresh(...); draw` and is state-neutral when paused.
        harness.draw();
        Self {
            scenario,
            motion,
            construct_frame,
            color,
            harness,
        }
    }

    /// Advance `n` helper ticks with a production draw after each, matching `H::ticks`.
    pub fn ticks(&mut self, n: usize) {
        for _ in 0..n {
            let _ = self.harness.advance(Duration::from_millis(HELPER_TICK_MS));
        }
    }

    /// Observe the last production frame without dispatching input.
    #[must_use]
    pub fn observe(&self, checkpoint: &str) -> ObservedFrame {
        let app = self.harness.app();
        let buffer = self.harness.buffer();
        let area = *buffer.area();
        let mut cells =
            Vec::with_capacity(usize::from(area.width).saturating_mul(usize::from(area.height)));
        for y in 0..area.height {
            for x in 0..area.width {
                cells.push(match buffer.cell((x, y)) {
                    Some(cell) => ObservedCell {
                        symbol: cell.symbol().to_owned(),
                        fg: format!("{:?}", cell.fg),
                        bg: format!("{:?}", cell.bg),
                        modifier: cell.modifier.bits(),
                    },
                    None => ObservedCell::missing(),
                });
            }
        }
        let cursor = self
            .harness
            .cursor()
            .map(|pos| ObservedCursor { x: pos.x, y: pos.y });
        ObservedFrame {
            identity: format!(
                "{id}/{world}/{motion}/{frame}/{width}x{height}/{color}/{checkpoint}",
                id = crate::JA001_ID,
                world = self.scenario.name(),
                motion = motion_name(self.motion),
                frame = self.construct_frame,
                width = area.width,
                height = area.height,
                color = color_label(self.color),
                checkpoint = checkpoint,
            ),
            scenario: self.scenario.name().to_owned(),
            motion: motion_name(self.motion).to_owned(),
            construct_frame: self.construct_frame,
            app_frame: app.frame(),
            width: area.width,
            height: area.height,
            color: color_label(self.color).to_owned(),
            theme: "junie".to_owned(),
            route: route_name(app.route()).to_owned(),
            focus: self.harness.focus().map(|id| format!("{id:?}")),
            cursor,
            now_ms: app.world.clock.now_ms,
            now_secs: app.world.now_secs(),
            clock_running: app.world.clock.running,
            motion_seed: MOTION_SEED,
            epoch_secs: EPOCH_SECS,
            digest: self.harness.snapshot().digest(),
            text: self.harness.text(),
            cells,
        }
    }
}

/// Stable CLI motion name.
#[must_use]
pub const fn motion_name(motion: Motion) -> &'static str {
    match motion {
        Motion::Full => "full",
        Motion::Reduced => "reduced",
        Motion::Paused => "paused",
    }
}

/// Stable route name for semantic records.
#[must_use]
pub const fn route_name(route: Route) -> &'static str {
    match route {
        Route::Intro => "intro",
        Route::Manager => "manager",
        Route::Prelude => "prelude",
        Route::Editor => "editor",
        Route::Accounts => "accounts",
        Route::Usage => "usage",
        Route::Settings => "settings",
        Route::Launch => "launch",
        Route::Cockpit => "cockpit",
        Route::Handoff => "handoff",
        Route::Capsule => "capsule",
        Route::Outro => "outro",
    }
}

/// Color-level label matching `junie_tui_testing` digest keys.
#[must_use]
pub const fn color_label(color: ColorLevel) -> &'static str {
    match color {
        ColorLevel::TrueColor => "truecolor",
        ColorLevel::Ansi256 => "256",
        ColorLevel::Ansi16 => "16",
        ColorLevel::Mono => "mono",
        _ => "other",
    }
}
