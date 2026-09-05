//! Database-domain compatibility exports.
//!
//! Grid lifecycle and `GridModel`/`GridEditor` behavior live in
//! [`crate::grid_model`]. This module keeps the app's domain-facing import
//! path available while making adapter ownership explicit.

pub use crate::grid_model::{PendingEdits, ResultGrid, preview_sql, sql_literal};
