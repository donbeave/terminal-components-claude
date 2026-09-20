//! JA-001: eight CLI worlds, Paused, frame 0, first leaf 80×24 truecolor.

use jackin_app::{Route, Scenario};

use crate::ObservedFrame;
use crate::observe::DirectSession;

/// Scenario id.
pub const JA001_ID: &str = "JA-001";
/// First-leaf width.
pub const JA001_FIRST_LEAF_WIDTH: u16 = 80;
/// First-leaf height.
pub const JA001_FIRST_LEAF_HEIGHT: u16 = 24;
/// First-leaf viewport.
pub const JA001_FIRST_LEAF_SIZE: (u16, u16) = (JA001_FIRST_LEAF_WIDTH, JA001_FIRST_LEAF_HEIGHT);
/// First-leaf color label.
pub const JA001_FIRST_LEAF_COLOR: &str = "truecolor";

/// One world's initial draw and the paused T(5) follow-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ja001Capture {
    /// `fresh(world, Paused, 0); draw`
    pub initial: ObservedFrame,
    /// `T(5)` after the initial draw; paused frames must be unchanged.
    pub after_ticks: ObservedFrame,
}

/// Source-enum world order for JA-001.
#[must_use]
pub fn ja001_worlds() -> [Scenario; 8] {
    Scenario::ALL
}

/// Capture every JA-001 world at Paused frame 0, 80×24, truecolor.
#[must_use]
pub fn ja001_paused_frame0_truecolor() -> Vec<Ja001Capture> {
    ja001_worlds().into_iter().map(capture_world).collect()
}

fn capture_world(scenario: Scenario) -> Ja001Capture {
    let mut session = DirectSession::ja001_first_leaf(scenario);
    let initial = session.observe("initial");
    session.ticks(5);
    let after_ticks = session.observe("after-t5");
    Ja001Capture {
        initial,
        after_ticks,
    }
}

/// Initial route for Paused frame 0, matching `App::for_scenario`.
#[must_use]
pub const fn expected_route(scenario: Scenario) -> Route {
    match scenario {
        Scenario::FirstUse => Route::Intro,
        Scenario::Returning | Scenario::HardCases => Route::Manager,
        Scenario::AccountsMixed => Route::Accounts,
        Scenario::LaunchRunning | Scenario::LaunchFailure => Route::Cockpit,
        Scenario::CapsuleMulti | Scenario::OutroLast => Route::Capsule,
    }
}
