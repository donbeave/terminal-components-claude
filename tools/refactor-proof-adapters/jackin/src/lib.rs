//! Reference observation adapter for Jackin oracle capture (TASK-004).
//!
//! The adapter constructs production [`App`](jackin_app::App) values through
//! [`App::for_scenario`](jackin_app::App::for_scenario) and observes the
//! public runtime draw path. It does not call the app-local historical
//! paint-over helpers and does not bless candidate frames as expected
//! artifacts.

#![forbid(unsafe_code)]

mod frame;
mod ja001;
mod ja002;
mod ja003;
mod ja004;
mod observe;

pub use frame::{ObservedCell, ObservedCursor, ObservedFrame};
pub use ja001::{
    JA001_FIRST_LEAF_COLOR, JA001_FIRST_LEAF_HEIGHT, JA001_FIRST_LEAF_SIZE, JA001_FIRST_LEAF_WIDTH,
    JA001_ID, JA001_REMAINING_SIZES, JA001_SIZES, Ja001Capture, expected_route,
    ja001_paused_frame0_all_sizes_truecolor, ja001_paused_frame0_at,
    ja001_paused_frame0_remaining_colors, ja001_paused_frame0_remaining_sizes_truecolor,
    ja001_paused_frame0_truecolor, ja001_worlds,
};
pub use ja002::{
    JA002_ID, ja002_frames, ja002_paused_first_use, knock_caption, production_intro_message,
};
pub use ja003::{JA003_ID, JA003_SIZES, Ja003Capture, ja003_first_use_full};
pub use ja004::{JA004_ID, JA004_SIZES, Ja004Capture, ja004_first_use_reduced_and_quit};
pub use observe::{
    CaptureColor, DirectSession, EPOCH_SECS, HELPER_TICK_MS, HISTORICAL_PAINT_SIZE, MOTION_SEED,
    Viewport, color_label, motion_name, route_name,
};

pub use jackin_app::{App, Motion, Route, Scenario};
pub use junie_tui::{Axis, ColorLevel, KeyCode, KeyModifiers, MouseKind};
