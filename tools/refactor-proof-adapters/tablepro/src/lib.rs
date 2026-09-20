//! TASK-005 `TablePro` oracle observation adapter.
//!
//! Direct and subprocess lanes drive the production CLI (`tablepro_app::run`)
//! and the same `TableProApp::default` / `connect` constructors `run_with`
//! uses. Named visual surfaces are never seeded through `set_surface`.

#![forbid(unsafe_code)]

mod cli;
mod launch;
mod observe;
mod scenario;

pub use cli::{HELP, invoke_cli, invoke_cli_with_env, parse_color_flag};
pub use launch::{launch_preset_c, theme_for};
pub use observe::{Observation, observe};
pub use scenario::{
    ALL_SIZES, CHECKPOINT_INITIAL, COLOR_PROFILES, ColorProfile, PRESET_C, SCENARIO_TP001,
};
