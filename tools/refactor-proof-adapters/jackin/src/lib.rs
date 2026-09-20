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
mod ja005;
mod ja006;
mod ja007;
mod ja008;
mod ja009;
mod ja010;
mod ja011;
mod ja012;
mod ja013;
mod ja014;
mod ja015;
mod ja016;
mod ja017;
mod ja018;
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
pub use ja005::{
    JA005_ID, JA005_PREFIX, JA005_VARIANTS, Ja005Capture, ja005_returning_manager_keys,
};
pub use ja006::{JA006_ID, Ja006Capture, ja006_returning_manager_pointer};
pub use ja007::{
    JA007_ID, JA007_LABELS, JA007_SIZES, Ja007Capture, ja007_returning_row_activation,
};
pub use ja008::{JA008_ID, JA008_SIZES, JA008_WORLDS, Ja008Capture, ja008_workspace_actions};
pub use ja009::{JA009_ID, JA009_SIZES, Ja009Capture, ja009_instance_actions};
pub use ja010::{JA010_ID, Ja010Capture, ja010_hard_cases_refresh_scroll};
pub use ja011::{JA011_ID, JA011_SIZES, Ja011Capture, ja011_launch_picker};
pub use ja012::{JA012_ID, Ja012Capture, ja012_prelude_pending_editor};
pub use ja013::{JA013_ID, JA013_SIZES, Ja013Capture, ja013_prelude_git_and_rewind};
pub use ja014::{JA014_ID, JA014_SIZES, Ja014Capture, ja014_prelude_name_validation};
pub use ja015::{JA015_ID, Ja015Capture, ja015_prelude_browser};
pub use ja016::{JA016_ID, Ja016Capture, ja016_editor_tabs};
pub use ja017::{JA017_ID, JA017_SIZES, Ja017Capture, ja017_editor_name_edit};
pub use ja018::{JA018_ID, JA018_SIZES, Ja018Capture, ja018_editor_save_replay};
pub use observe::{
    CaptureColor, DirectSession, EPOCH_SECS, HELPER_TICK_MS, HISTORICAL_PAINT_SIZE, MOTION_SEED,
    Viewport, color_label, motion_name, route_name,
};

pub use jackin_app::{App, Motion, Route, Scenario};
pub use junie_tui::{Axis, ColorLevel, Id, KeyCode, KeyModifiers, MouseKind, Rect};
