//! Junie-inspired design system for Ratatui: tokens, interaction primitives
//! and components. Applications (the showcase, TablePro) live in `src/bin`.

pub mod core;
pub mod runtime;
pub mod theme;
pub mod ui;
pub mod widgets;

// Compatibility facade consumers use the public Ratatui types without adding
// a second direct terminal dependency to an application package.
#[doc(hidden)]
pub use ratatui;
