//! Workspace editor state and public control ids.

use tui_next::Id;

/// Editor root.
pub const ROOT: Id = Id::root("jackin.editor");
/// Editor form root retained as a stable namespace for nested controls.
pub const FORM: Id = ROOT.sub("form");
/// Save action.
pub const SAVE: Id = FORM.sub("save");
/// Editor tabs.
pub const TABS: Id = ROOT.sub("tabs");

/// Editor tab projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// General workspace properties.
    #[default]
    General,
    /// Mounts.
    Mounts,
    /// Roles.
    Roles,
    /// Environment variables.
    Environments,
    /// Account policy.
    Accounts,
}

/// Durable editor state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorState {
    pub tab: Tab,
    pub dirty: bool,
    pub preview_open: bool,
    pub env_visible: bool,
}

impl EditorState {
    /// Select a tab by one-based fixture index.
    pub const fn select_index(&mut self, index: u8) {
        self.tab = match index {
            1 => Tab::Mounts,
            2 => Tab::Roles,
            3 => Tab::Environments,
            4 | 5 => Tab::Accounts,
            _ => Tab::General,
        };
    }
}
