//! `TablePro` application package.
//!
//! The legacy application is exposed through the root package's public
//! terminal facade while the binary and its tests live in this workspace
//! package.  The move keeps the application implementation and its rendered
//! behavior unchanged; only ownership and the entry-point boundary changed.
#![forbid(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    reason = "the compatibility facade intentionally mirrors the legacy app surface"
)]

pub extern crate tui_next as legacy_facade;

pub use legacy_facade::app;
pub use legacy_facade::app::{App, Modal, Screen};
pub use legacy_facade::connections;
pub use legacy_facade::db;
pub use legacy_facade::model;
pub use legacy_facade::sql;
pub use legacy_facade::tablepro_entry::run;
pub use legacy_facade::tabs;
pub use legacy_facade::workbench;
