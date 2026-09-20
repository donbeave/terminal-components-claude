//! Holla oracle reference observation adapter (`oracle-holla`).
//!
//! Observes production `holla_app` through `App::for_scenario`. Oracle
//! `--frame N` is an 80 ms tick ordinal. The adapter never treats a tick
//! ordinal as milliseconds and never substitutes a missing world.

#![allow(missing_docs)]

pub mod branches;
pub mod clock;
pub mod construct;
pub mod expansion;
pub mod observe;
pub mod resize_map;
pub mod stages;
pub mod tsv;
pub mod worlds;

pub use clock::{ORACLE_EPOCH_SECS, TICK_MS, tick_to_ms};
pub use construct::{Construction, construct, construct_ho_base};
pub use expansion::{ScenarioProgram, expand_scenarios};
pub use observe::{ORACLE_PALETTES, ORACLE_SIZES, Session};
pub use worlds::{ORACLE_WORLD_COUNT, ORACLE_WORLDS};
