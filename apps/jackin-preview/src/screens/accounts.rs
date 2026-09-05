//! Accounts route state and controls.

use tui_next::{Id, ListState};

/// Accounts list control.
pub const LIST: Id = Id::root("jackin.accounts.list");
/// Account form root.
pub const FORM: Id = Id::root("jackin.accounts.form");
/// Account form save action.
pub const SAVE: Id = FORM.sub("save");
/// Provider selection control.
pub const PROVIDER: Id = FORM.sub("provider");
/// 1Password source control.
pub const OP: Id = FORM.sub("op");

/// Durable accounts route state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AccountsState {
    /// Public-list selection state owned by this route.
    pub list: ListState,
    pub form_open: bool,
    pub editing: Option<String>,
    pub draft_name: String,
    pub masked_input: String,
    pub pending_refresh: Option<String>,
    pub remove_confirmation: Option<String>,
}

impl AccountsState {
    /// Open a new account form.
    pub fn open_new(&mut self) {
        self.form_open = true;
        self.editing = None;
        self.draft_name.clear();
        self.masked_input.clear();
    }

    /// Close the form without writing a credential.
    pub const fn close(&mut self) {
        self.form_open = false;
    }
}
