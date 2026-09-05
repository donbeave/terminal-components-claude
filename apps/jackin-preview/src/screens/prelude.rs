//! Workspace creation prelude state.

use tui_next::Id;

/// Prelude composition root.
pub const ROOT: Id = Id::root("jackin.prelude");
/// Source browser control.
pub const SOURCE: Id = ROOT.sub("source");
/// Destination control.
pub const DESTINATION: Id = ROOT.sub("destination");
/// Continue action.
pub const CONTINUE: Id = ROOT.sub("continue");

/// Five-step workspace creation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreludeState {
    step: u8,
    source: String,
    name: String,
    duplicate: bool,
}

impl Default for PreludeState {
    fn default() -> Self {
        Self {
            step: 1,
            source: "~/src/payments-platform".into(),
            name: "payments-platform".into(),
            duplicate: false,
        }
    }
}

impl PreludeState {
    /// Current one-based step.
    pub const fn step(&self) -> u8 {
        self.step
    }

    /// Current source path.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Proposed workspace name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Advance one step, bounded by the five-step flow.
    pub fn advance(&mut self) {
        self.step = self.step.saturating_add(1).min(5);
    }

    /// Rewind one step.
    pub fn back(&mut self) {
        self.step = self.step.saturating_sub(1).max(1);
    }

    /// Whether the proposed name collides with an existing workspace.
    pub const fn duplicate(&self) -> bool {
        self.duplicate
    }

    /// Mark duplicate-name validation result.
    pub const fn set_duplicate(&mut self, duplicate: bool) {
        self.duplicate = duplicate;
    }
}
