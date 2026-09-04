//! Junie-inspired design system for Ratatui: tokens, interaction primitives
//! and components. Applications (the showcase, TablePro) live in `src/bin`.

extern crate self as junie_tui;

pub mod core;
pub mod runtime;
pub mod theme;
pub mod ui;
pub mod widgets;

// The legacy TablePro implementation remains in the root library while the
// application package is moved to the workspace.  Keeping these modules at
// the crate root preserves every `crate::...` path in the implementation and
// makes the package move a boundary-only change.  They are intentionally
// hidden from the design-system documentation; the new `crates/tui` facade
// remains domain-free.
#[doc(hidden)]
#[path = "legacy_tablepro/app.rs"]
pub mod app;
#[doc(hidden)]
#[path = "legacy_tablepro/connections.rs"]
pub mod connections;
#[doc(hidden)]
#[path = "legacy_tablepro/db.rs"]
pub mod db;
#[doc(hidden)]
#[path = "legacy_tablepro/model.rs"]
pub mod model;
#[doc(hidden)]
#[path = "legacy_tablepro/sql.rs"]
pub mod sql;
#[doc(hidden)]
#[path = "legacy_tablepro/main.rs"]
pub mod tablepro_entry;
#[doc(hidden)]
#[path = "legacy_tablepro/tabs.rs"]
pub mod tabs;
#[doc(hidden)]
#[path = "legacy_tablepro/workbench.rs"]
pub mod workbench;
