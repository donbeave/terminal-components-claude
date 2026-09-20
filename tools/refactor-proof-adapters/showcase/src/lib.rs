//! Showcase reference observation and Instant clock adapter (`oracle-showcase`).
//!
//! Drives production [`showcase_app::App`] through [`junie_tui::App::update`] /
//! [`junie_tui::App::draw`]. Oracle membership is 23 pages including Diff;
//! production currently exposes 22 routes. This crate observes that gap and
//! does not patch product UX. Pointer targets are frozen coordinates or
//! source `Id`s — never `Harness::find`.

#![allow(missing_docs, reason = "skeleton: public surface expands with SC rows")]

/// Accepted producer product identity for TASK-002.
pub const PRODUCT: &str = "oracle-showcase";

/// Oracle page count, including Diff.
pub const ORACLE_PAGE_COUNT: usize = 23;

/// Default page frames: 23 pages × 4 sizes × 4 colors.
pub const DEFAULT_PAGE_FRAME_COUNT: usize = 368;
