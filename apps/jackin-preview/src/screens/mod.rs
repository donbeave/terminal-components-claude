//! Route screens and the contract they share with the shell: input,
//! requests back to the app, modal ownership, rendering and hints.

pub mod accounts;
pub mod capsule;
pub mod cockpit;
pub mod config;
pub mod editor;
pub mod file_browser;
pub mod inspect;
pub mod manager;
pub mod modals;
pub mod op_flow;
pub mod prelude;
pub mod settings;
pub mod usage;

use crate::ratatui::buffer::Buffer;
use crate::ratatui::layout::{Position, Rect};
use junie_tui::core::event::{Key, Outcome};
use junie_tui::core::focus::{Focus, FocusRing};
use junie_tui::core::id::WidgetId;
use junie_tui::ui::ctx::RenderCtx;
use junie_tui::widgets::dialog::Dialog;
use junie_tui::widgets::keyhint::Hint;
use junie_tui::widgets::picker::{Picker, PickerItem};
use junie_tui::widgets::segments::Segment;

use crate::domain::account::AccountId;
use crate::domain::agent::Agent;
use crate::domain::instance::InstanceId;
use crate::domain::onepassword::OpReference;
use crate::domain::workspace::{Workspace, WorkspaceId};
use crate::sim::launch::LaunchPlan;
use crate::sim::world::{Msg, World};
use modals::{
    BrowserResult, ChoiceDialog, FileBrowser, FormDialog, FormValues, HelpOverlay, InfoDialog,
    InfoResult, OpFlow,
};

/// Identifies a modal's purpose and target for its owning screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalTag {
    pub kind: &'static str,
    pub key: String,
    pub n: usize,
}

impl ModalTag {
    pub fn new(kind: &'static str) -> Self {
        Self {
            kind,
            key: String::new(),
            n: 0,
        }
    }
    pub fn key(mut self, k: impl Into<String>) -> Self {
        self.key = k.into();
        self
    }
    pub fn n(mut self, n: usize) -> Self {
        self.n = n;
        self
    }
}

/// A screen-specific modal that still lives on the shared modal stack.
pub trait LegacyCustomModal {
    fn on_key(&mut self, key: &Key, focus: &mut Focus, ring: &FocusRing, w: &World) -> Outcome;
    fn on_click(
        &mut self,
        _id: WidgetId,
        _pos: Position,
        _focus: &mut Focus,
        _w: &World,
    ) -> Outcome {
        Outcome::Consumed
    }
    /// Wheel over the modal; `pos` lets a modal with several regions route it.
    fn on_wheel(&mut self, _delta: i32, _pos: Position) -> Outcome {
        Outcome::Consumed
    }
    /// Pointer drag that started on `pressed` inside the modal.
    fn on_drag(&mut self, _pressed: WidgetId, _pos: Position) -> Outcome {
        Outcome::Consumed
    }
    fn on_tick(&mut self, _w: &World) -> Outcome {
        Outcome::Ignored
    }
    fn render(&mut self, screen: Rect, buf: &mut Buffer, ctx: &mut RenderCtx, w: &World);
    fn done(&mut self) -> Option<ModalResult>;
    fn initial_focus(&self) -> WidgetId;
    fn hints(&self) -> Vec<Hint>;
    fn cancel_on_outside_click(&self) -> bool {
        true
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Modal {
    Dialog(Dialog),
    Picker(Picker),
    Browser(FileBrowser),
    Choice(ChoiceDialog),
    Form(FormDialog),
    Op(OpFlow),
    Info(InfoDialog),
    Help(HelpOverlay),
    Custom(Box<dyn LegacyCustomModal>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalResult {
    /// `action` is the pressed button index (None = cancelled); `text` the
    /// prompt's value.
    Dialog {
        action: Option<usize>,
        text: Option<String>,
    },
    Picked(usize),
    PickedAlt(usize),
    /// Picker Tab: cycle scope (owner re-supplies rows).
    Scope,
    Cancelled,
    Browser(BrowserResult),
    Choice(Option<usize>),
    Form(Option<FormValues>),
    /// Named button on a form other than Save/Cancel (e.g. Validate).
    FormAction(String, FormValues),
    Op(Option<OpReference>),
    Info(InfoResult),
    Custom(String),
}

/// Navigation the app performs on a screen's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Go {
    Manager,
    Settings,
    Accounts {
        select: Option<AccountId>,
    },
    Usage {
        select: Option<AccountId>,
    },
    /// Edit an existing Workspace, or create from a pending one.
    Editor {
        workspace: Option<WorkspaceId>,
        pending: Option<Box<Workspace>>,
    },
    Prelude,
    Launch {
        workspace: Option<WorkspaceId>,
        role: String,
        agent: Agent,
        account: Option<AccountId>,
        plan: LaunchPlan,
    },
    Attach {
        instance: InstanceId,
        pane: Option<u64>,
    },
    NewSession {
        instance: InstanceId,
        agent: Option<Agent>,
        account: Option<AccountId>,
    },
    /// The client leaves; the instance keeps running.
    Detach,
    /// The foreground instance ended (last session closed / exit chosen).
    InstanceEnded {
        instance: InstanceId,
        purge: bool,
    },
    /// A failed fresh launch was acknowledged.
    LaunchFailedAck {
        instance: Option<InstanceId>,
    },
    Quit,
}

pub enum Request {
    Status(String),
    Error(String),
    Open(Box<Modal>, ModalTag),
    Close,
    Go(Go),
    Copy(String),
    /// Open the key reference for the current screen.
    Help,
    /// Mutate the form that is the top modal (after a nested picker).
    WithForm(Box<dyn FnOnce(&mut FormDialog)>),
}

pub struct LegacyCx<'a> {
    pub focus: &'a mut Focus,
    pub ring: &'a FocusRing,
    pub requests: Vec<Request>,
}

/// Compatibility name for the pre-public shell context.
///
/// New screens use [`crate::public_tui::Cx`] through [`Screen`]. Keeping the
/// alias here lets the legacy event adapter be migrated one route at a time
/// without changing its request semantics.
pub type Cx<'a> = LegacyCx<'a>;

impl LegacyCx<'_> {
    pub fn focus_next(&mut self) {
        self.focus.next(self.ring);
    }
    pub fn status(&mut self, s: impl Into<String>) {
        self.requests.push(Request::Status(s.into()));
    }
    pub fn error(&mut self, s: impl Into<String>) {
        self.requests.push(Request::Error(s.into()));
    }
    pub fn open(&mut self, modal: Modal, tag: ModalTag) {
        self.requests.push(Request::Open(Box::new(modal), tag));
    }
    pub fn close(&mut self) {
        self.requests.push(Request::Close);
    }
    pub fn go(&mut self, g: Go) {
        self.requests.push(Request::Go(g));
    }
    pub fn help(&mut self) {
        self.requests.push(Request::Help);
    }
    pub fn copy(&mut self, s: impl Into<String>) {
        self.requests.push(Request::Copy(s.into()));
    }
    pub fn with_form(&mut self, f: impl FnOnce(&mut FormDialog) + 'static) {
        self.requests.push(Request::WithForm(Box::new(f)));
    }
}

/// Everything a route screen can do. Default bodies let small screens stay
/// small; the shell calls only what it needs.
pub trait LegacyScreen {
    fn on_key(&mut self, key: &Key, w: &mut World, cx: &mut LegacyCx) -> Outcome;
    fn on_click(
        &mut self,
        _id: WidgetId,
        _pos: Position,
        _w: &mut World,
        _cx: &mut LegacyCx,
    ) -> Outcome {
        Outcome::Ignored
    }
    fn on_double_click(
        &mut self,
        id: WidgetId,
        pos: Position,
        w: &mut World,
        cx: &mut LegacyCx,
    ) -> Outcome {
        let _ = (id, pos, w, cx);
        Outcome::Ignored
    }
    fn on_drag(&mut self, _pressed: WidgetId, _pos: Position, _w: &mut World) -> Outcome {
        Outcome::Ignored
    }
    /// Secondary (right) mouse button on `id`: context menus.
    fn on_secondary(
        &mut self,
        _id: WidgetId,
        _pos: Position,
        _w: &mut World,
        _cx: &mut LegacyCx,
    ) -> Outcome {
        Outcome::Ignored
    }
    /// Mouse button went down on `id`; a drag may follow before the click
    /// completes on release. Screens that select text anchor here.
    fn on_press(
        &mut self,
        _id: WidgetId,
        _pos: Position,
        _w: &mut World,
        _cx: &mut LegacyCx,
    ) -> Outcome {
        Outcome::Ignored
    }
    fn on_release(
        &mut self,
        _pressed: WidgetId,
        _pos: Position,
        _w: &mut World,
        _cx: &mut LegacyCx,
    ) -> Outcome {
        Outcome::Ignored
    }
    fn on_wheel(&mut self, _id: WidgetId, _delta: i32, _pos: Position, _w: &mut World) -> Outcome {
        Outcome::Ignored
    }
    fn on_paste(&mut self, _text: &str, _w: &mut World) -> Outcome {
        Outcome::Ignored
    }
    fn on_tick(&mut self, _w: &mut World, _cx: &mut Cx) -> Outcome {
        Outcome::Ignored
    }
    fn on_msg(&mut self, _msg: &Msg, _w: &mut World, _cx: &mut Cx) -> Outcome {
        Outcome::Ignored
    }
    fn on_modal(
        &mut self,
        _tag: &ModalTag,
        _result: ModalResult,
        _w: &mut World,
        _cx: &mut Cx,
    ) -> Outcome {
        Outcome::Ignored
    }
    /// Rows for a picker this screen opened; `None` keeps the current rows.
    fn picker_items(
        &mut self,
        _tag: &ModalTag,
        _query: &str,
        _w: &World,
    ) -> Option<Vec<PickerItem>> {
        None
    }
    /// A form field changed: reveal/hide dependent fields.
    fn form_changed(&mut self, _tag: &ModalTag, _form: &mut FormDialog, _w: &World) {}
    fn render(&mut self, area: Rect, buf: &mut Buffer, ctx: &mut RenderCtx, w: &mut World);
    fn hints(&self, focus: Option<WidgetId>, w: &World) -> Vec<Hint>;
    /// Scope crumb after the Construct state: `Workspaces › api-server`.
    fn crumb(&self, w: &World) -> String;
    /// Extra right-hand strip segments (dirty count, refreshing…).
    fn strip_right(&self, _w: &World) -> Vec<Segment> {
        vec![]
    }
    fn is_editing(&self) -> bool {
        false
    }
    fn animating(&self, _w: &World) -> bool {
        false
    }
    /// The route became active.
    fn enter(&mut self, _w: &mut World, _cx: &mut Cx) {}
    /// Where keyboard focus lands when nothing better is known.
    fn primary_focus(&self) -> Option<WidgetId>;
    /// Esc at the top of this screen's ladder: leave it?
    fn on_esc_top(&mut self, _w: &mut World, cx: &mut Cx) -> Outcome {
        cx.go(Go::Manager);
        Outcome::Changed
    }
}

/// Public two-phase screen contract used by the migrated shell.
///
/// This deliberately has no access to the legacy event/render adapter. A
/// route receives input in [`update`](Screen::update), then paints from an
/// immutable snapshot in [`draw`](Screen::draw). Product routes can adopt it
/// independently while the legacy adapter remains available for compatibility
/// tests during the migration.
pub trait Screen {
    fn update(
        &mut self,
        cx: &mut crate::public_tui::Cx<'_>,
        jx: &mut Jx<'_>,
        world: &mut World,
    ) -> crate::public_tui::Response<()>;

    fn draw(
        &self,
        ui: &mut crate::public_tui::Ui<'_>,
        area: crate::public_tui::Rect,
        world: &World,
    );

    fn hints(&self, _world: &World) -> crate::public_tui::HintLayer {
        crate::public_tui::HintLayer::empty()
    }

    fn crumb(&self, _world: &World) -> String {
        String::new()
    }

    fn primary_focus(&self) -> Option<crate::public_tui::Id> {
        None
    }

    fn on_esc_top(
        &mut self,
        _cx: &mut crate::public_tui::Cx<'_>,
        jx: &mut Jx<'_>,
        _world: &mut World,
    ) -> crate::public_tui::Response<()> {
        jx.go(Go::Manager);
        crate::public_tui::Response::consumed().repaint()
    }
}

/// Requests emitted by a public screen and consumed by the public app shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicRequest {
    Status(String),
    Go(Go),
    Quit,
}

/// Commands owned by the public Jackin shell. They are intentionally stable
/// identities, so the shell keymap and a route's update phase share one
/// vocabulary without exposing the legacy event enum.
pub const PUBLIC_MANAGER_UP: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.manager.up");
pub const PUBLIC_MANAGER_DOWN: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.manager.down");
pub const PUBLIC_MANAGER_ACTIVATE: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.manager.activate");
pub const PUBLIC_QUIT: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.quit");

/// Public navigation commands shared by the migrated route screens.
///
/// They are shell-owned identities rather than legacy key events.  A route
/// decides what moving or activating means for its own state in `update`.
pub const PUBLIC_NAV_UP: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.navigation.up");
pub const PUBLIC_NAV_DOWN: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.navigation.down");
pub const PUBLIC_ACTIVATE: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.navigation.activate");
pub const PUBLIC_BACK: crate::public_tui::ActionKey =
    crate::public_tui::ActionKey::custom("jackin.navigation.back");

/// Product-owned command sink for the public screen contract.
pub struct Jx<'a> {
    requests: &'a mut Vec<PublicRequest>,
}

impl Jx<'_> {
    pub fn new(requests: &mut Vec<PublicRequest>) -> Jx<'_> {
        Jx { requests }
    }

    pub fn status(&mut self, status: impl Into<String>) {
        self.requests.push(PublicRequest::Status(status.into()));
    }

    pub fn go(&mut self, route: Go) {
        self.requests.push(PublicRequest::Go(route));
    }

    pub fn quit(&mut self) {
        self.requests.push(PublicRequest::Quit);
    }
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {many}")
    }
}
