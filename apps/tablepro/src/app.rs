//! `TablePro` application shell built only on the public `tui-next` facade.

use tui_next::{
    Action, ActionKey, App, Chord, Cx, Field, Form, FormAction, FormState, Grid, GridAction,
    GridState, Id, KeyCode, KeyMap, KeyModifiers, KeyPhase, List, ListAction, ListState, Panel,
    PanelKind, Response, Size, StatusBar, StatusItem, Tabs, TabsAction, TabsState, TextInput,
    TextInputState, Ui, UpdateCause, Variant,
};

use crate::connections::{self, ConnectionDraft, ConnectionsScreen};
use crate::db::{self, Catalog, ColType, ConnectOutcome, Connection, SafeMode};
use crate::grid_model::ResultGrid;
use crate::workbench::Workbench;

/// Minimum terminal width.
pub const MIN_WIDTH: u16 = 72;
/// Minimum terminal height.
pub const MIN_HEIGHT: u16 = 20;
const QUERY: Id = Id::root("tablepro.query");
const RESULTS: Id = Id::root("tablepro.results");
const STATUS: Id = Id::root("tablepro.status");
const HEADER: Id = Id::root("tablepro.header");
const RUN: ActionKey = ActionKey::custom("tablepro.run");
const QUIT: ActionKey = ActionKey::custom("tablepro.quit");
const OPEN: ActionKey = ActionKey::custom("tablepro.open");
const NEW_QUERY: ActionKey = ActionKey::custom("tablepro.new-query");
const HISTORY: ActionKey = ActionKey::custom("tablepro.history");
const STRUCTURE: ActionKey = ActionKey::custom("tablepro.structure");
const FORM: ActionKey = ActionKey::custom("tablepro.form");
const HELP: ActionKey = ActionKey::custom("tablepro.help");
const SAFE_MODE: ActionKey = ActionKey::custom("tablepro.safe-mode");
const MAXIMIZE: ActionKey = ActionKey::custom("tablepro.maximize");
const EXPLAIN: ActionKey = ActionKey::custom("tablepro.explain");
const FILTER: ActionKey = ActionKey::custom("tablepro.filter");
const TAB_LIST: ActionKey = ActionKey::custom("tablepro.tab-list");
const CONNECTIONS: Id = Id::root("tablepro.connections.list");
const CONNECTIONS_PANEL: Id = Id::root("tablepro.connections.panel");
const EXPLORER: Id = Id::root("tablepro.workbench.explorer.list");
const EXPLORER_PANEL: Id = Id::root("tablepro.workbench.explorer.panel");
const TAB_STRIP: Id = Id::root("tablepro.workbench.tab-strip");
const MAX_TAB_LABELS: usize = 64;

/// Product-level screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Connection list and setup screen.
    Connections,
    /// Connected workbench screen.
    Workbench,
}

/// Named visual surfaces retained from the legacy showcase matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Connection list.
    Connections,
    /// Failed connection state.
    ConnectionsFailed,
    /// Default workbench.
    WorkbenchDefault,
    /// Explorer focus.
    ExplorerFocused,
    /// Table data grid.
    TableGrid,
    /// Inline cell editing.
    GridCellEditing,
    /// Pending-change bar.
    PendingChangeBar,
    /// Structure view.
    StructureView,
    /// Query editor.
    QueryEditing,
    /// Completion popup state.
    CompletionPopup,
    /// Successful results.
    ResultsGrid,
    /// Error results.
    ErrorResult,
    /// Explain plan.
    ExplainPlan,
    /// History tab.
    HistoryTab,
    /// Quick switcher.
    QuickSwitcher,
    /// Tab-list picker.
    TabListPicker,
    /// Safe-mode picker.
    SafeModePicker,
    /// Filter editor.
    FilterEditor,
    /// Safety acknowledgement dialog.
    SafetyDialogTypedAck,
    /// Help dialog.
    HelpDialog,
    /// Maximised tab.
    MaximisedTab,
}

impl Surface {
    /// All matrix surfaces in stable order.
    pub const ALL: [Self; 21] = [
        Self::Connections,
        Self::ConnectionsFailed,
        Self::WorkbenchDefault,
        Self::ExplorerFocused,
        Self::TableGrid,
        Self::GridCellEditing,
        Self::PendingChangeBar,
        Self::StructureView,
        Self::QueryEditing,
        Self::CompletionPopup,
        Self::ResultsGrid,
        Self::ErrorResult,
        Self::ExplainPlan,
        Self::HistoryTab,
        Self::QuickSwitcher,
        Self::TabListPicker,
        Self::SafeModePicker,
        Self::FilterEditor,
        Self::SafetyDialogTypedAck,
        Self::HelpDialog,
        Self::MaximisedTab,
    ];
    /// Stable matrix label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Connections => "connections",
            Self::ConnectionsFailed => "connections-failed",
            Self::WorkbenchDefault => "workbench-default",
            Self::ExplorerFocused => "explorer-focused",
            Self::TableGrid => "table-grid",
            Self::GridCellEditing => "grid-cell-editing",
            Self::PendingChangeBar => "pending-change-bar",
            Self::StructureView => "structure-view",
            Self::QueryEditing => "query-editing",
            Self::CompletionPopup => "completion-popup",
            Self::ResultsGrid => "results-grid",
            Self::ErrorResult => "error-result",
            Self::ExplainPlan => "explain-plan",
            Self::HistoryTab => "history-tab",
            Self::QuickSwitcher => "quick-switcher",
            Self::TabListPicker => "tab-list-picker",
            Self::SafeModePicker => "safe-mode-picker",
            Self::FilterEditor => "filter-editor",
            Self::SafetyDialogTypedAck => "safety-dialog-typed-ack",
            Self::HelpDialog => "help-dialog",
            Self::MaximisedTab => "maximised-tab",
        }
    }
}

fn keymap() -> KeyMap {
    KeyMap::new()
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('r'), KeyModifiers::CONTROL),
            RUN,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('q'), KeyModifiers::CONTROL),
            QUIT,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('o'), KeyModifiers::CONTROL),
            OPEN,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('t'), KeyModifiers::CONTROL),
            NEW_QUERY,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('y'), KeyModifiers::CONTROL),
            HISTORY,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('d'), KeyModifiers::CONTROL),
            STRUCTURE,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('n'), KeyModifiers::CONTROL),
            FORM,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('l'), KeyModifiers::CONTROL),
            SAFE_MODE,
        )
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('x'), KeyModifiers::ALT),
            EXPLAIN,
        )
        .bind(KeyPhase::Bubble, Chord::key(KeyCode::Char('f')), FILTER)
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('g'), KeyModifiers::CONTROL),
            TAB_LIST,
        )
        .bind(KeyPhase::Bubble, Chord::key(KeyCode::Char('z')), MAXIMIZE)
        .bind(
            KeyPhase::Bubble,
            Chord::with(KeyCode::Char('?'), KeyModifiers::NONE),
            HELP,
        )
}

fn fallback_connection(catalog: &Catalog) -> Connection {
    Connection {
        name: "Local PostgreSQL".to_owned(),
        engine: db::Engine::Postgres,
        host: "localhost".to_owned(),
        port: 5432,
        database: catalog.database.clone(),
        user: "postgres".to_owned(),
        environment: db::Environment::Local,
        safe_mode: SafeMode::Silent,
        ssl: false,
        ssh: None,
        group: "Personal".to_owned(),
        last_used: "never".to_owned(),
        outcome: ConnectOutcome::Ok,
    }
}

/// `TablePro` state and app-owned adapters.
pub struct TableProApp {
    catalog: Catalog,
    connections: Vec<Connection>,
    connection: Connection,
    keymap: KeyMap,
    safe_mode: SafeMode,
    query: String,
    query_state: TextInputState,
    columns: Vec<(String, ColType)>,
    result: ResultGrid,
    grid_state: GridState,
    connection_list_state: ListState,
    explorer_list_state: ListState,
    tab_state: TabsState,
    status: String,
    quit: bool,
    /// Current product screen.
    pub screen: Screen,
    /// Current visual matrix surface.
    pub surface: Surface,
    /// Connection list state.
    pub connections_screen: ConnectionsScreen,
    /// Connected workbench.
    pub workbench: Workbench,
    draft: Option<ConnectionDraft>,
    form_state: FormState,
    form_fields: Box<[tui_next::FieldSpec<'static>]>,
    form_actions: Box<[Action<'static>]>,
    form_open: bool,
}

impl core::fmt::Debug for TableProApp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TableProApp")
            .field("screen", &self.screen)
            .field("surface", &self.surface)
            .field("connections", &self.connections.len())
            .field("connection", &self.connection.name)
            .field("safe_mode", &self.safe_mode)
            .field("query", &"[redacted]")
            .field("columns", &self.columns.len())
            .field("result", &self.result)
            .field("status", &self.status)
            .field("form_open", &self.form_open)
            .finish_non_exhaustive()
    }
}

impl Default for TableProApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TableProApp {
    /// Construct the deterministic demo app.
    pub fn new() -> Self {
        let catalog = Catalog::acme_prod();
        let connections = db::connections();
        let connection = connections
            .first()
            .cloned()
            .unwrap_or_else(|| fallback_connection(&catalog));
        let mut app = Self {
            safe_mode: connection.safe_mode,
            catalog: catalog.clone(),
            connections: connections.clone(),
            connection: connection.clone(),
            keymap: keymap(),
            query:
                "SELECT * FROM orders WHERE status = 'pending' ORDER BY total_amount DESC LIMIT 20"
                    .to_owned(),
            query_state: TextInputState::default(),
            columns: Vec::new(),
            result: ResultGrid::empty(),
            grid_state: GridState::default(),
            connection_list_state: ListState::default(),
            explorer_list_state: ListState::default(),
            tab_state: TabsState::default(),
            status: "Ready · Ctrl+R runs · Ctrl+Q quits".to_owned(),
            quit: false,
            screen: Screen::Connections,
            surface: Surface::Connections,
            connections_screen: ConnectionsScreen::new(connections),
            workbench: Workbench::new(connection, catalog),
            draft: None,
            form_state: FormState::default(),
            form_fields: Box::from(connections::form_fields()),
            form_actions: Box::from(connections::form_actions()),
            form_open: false,
        };
        let _ = app.execute_query();
        app
    }
    /// Active safe-mode policy.
    pub const fn safe_mode(&self) -> SafeMode {
        self.safe_mode
    }
    /// Current SQL text.
    pub fn query(&self) -> &str {
        &self.query
    }
    /// Current result adapter.
    pub const fn result(&self) -> &ResultGrid {
        &self.result
    }
    /// Latest status text.
    pub fn status(&self) -> &str {
        &self.status
    }
    /// Current screen.
    pub const fn screen(&self) -> Screen {
        self.screen
    }
    /// Current visual surface.
    pub const fn surface(&self) -> Surface {
        self.surface
    }
    /// Set a named surface for capture/tests.
    pub const fn set_surface(&mut self, surface: Surface) {
        self.surface = surface;
    }
    /// Borrow the connected workbench.
    pub const fn workbench(&self) -> &Workbench {
        &self.workbench
    }
    /// Whether the public connection form is open.
    pub const fn connection_form_open(&self) -> bool {
        self.form_open
    }
    /// Borrow the draft without exposing a password string.
    pub const fn connection_draft(&self) -> Option<&ConnectionDraft> {
        self.draft.as_ref()
    }
    /// Close the form (kept small so deterministic tests can model Esc).
    pub fn form_open_for_test(&mut self, open: bool) {
        self.form_open = open;
        if !open {
            self.draft = None;
        }
    }
    /// Open the connection form with the active connection as its draft.
    pub fn begin_connection_form(&mut self) {
        self.draft = Some(ConnectionDraft::from_connection(&self.connection));
        self.form_state = FormState::default();
        self.form_open = true;
        self.surface = Surface::Connections;
    }
    /// Select a connection and open its workbench.
    pub fn connect(&mut self, index: usize) -> bool {
        let Some(connection) = self.connections.get(index).cloned() else {
            return false;
        };
        if connection.outcome != ConnectOutcome::Ok {
            self.status = format!("Connection failed: {}", connection.name);
            self.connections_screen.error = Some("Connection failed; press r to retry".to_owned());
            self.surface = Surface::ConnectionsFailed;
            return false;
        }
        self.safe_mode = connection.safe_mode;
        self.connection = connection.clone();
        let mut workbench = Workbench::new(connection.clone(), self.catalog.clone());
        // Every connected workbench starts on the legacy blank query tab.  A
        // separate app-level query buffer would otherwise make Tab land on a
        // pre-executed fixture statement instead of the user's editor.
        workbench.new_query("");
        self.workbench = workbench;
        self.query.clear();
        self.query_state = TextInputState::default();
        self.columns.clear();
        self.result = ResultGrid::empty();
        self.grid_state = GridState::default();
        if let Some(item) = self.workbench.explorer.get(1) {
            self.explorer_list_state
                .set_cursor(1, tui_next::ItemKey::text(&item.name));
        }
        self.connection_list_state
            .choose(Some(tui_next::ItemKey::index(index)));
        self.screen = Screen::Workbench;
        self.surface = Surface::WorkbenchDefault;
        self.status = format!("Connected to {}", connection.name);
        true
    }
    /// Open a catalog table in the active workbench and expose its grid.
    pub fn open_table(&mut self, name: &str) -> bool {
        let opened = self.workbench.open_table(name);
        if opened {
            self.sync_result_from_active_tab();
            self.surface = Surface::TableGrid;
        }
        opened
    }
    /// Create and activate a query tab.
    pub fn new_query(&mut self, query: impl Into<String>) -> usize {
        let query = query.into();
        self.query.clone_from(&query);
        self.query_state = TextInputState::default();
        let index = self.workbench.new_query(query);
        self.surface = Surface::QueryEditing;
        index
    }
    /// Toggle the active table's data/structure view.
    pub fn toggle_structure(&mut self) -> bool {
        let changed = self.workbench.toggle_structure();
        if changed {
            self.surface = if self
                .workbench
                .active_table()
                .is_some_and(crate::tabs::TableTab::is_structure)
            {
                Surface::StructureView
            } else {
                Surface::TableGrid
            };
        }
        changed
    }
    /// Change the active safe-mode policy.
    pub fn set_safe_mode(&mut self, mode: SafeMode) {
        self.safe_mode = mode;
        self.connection.safe_mode = mode;
        self.workbench.connection.safe_mode = mode;
        self.surface = Surface::SafeModePicker;
    }
    /// Run a query through the same parser, gate and executor as Ctrl+R.
    pub fn run_query(&mut self, query: impl Into<String>) -> QueryOutcome {
        self.query = query.into();
        self.query_state = TextInputState::default();
        self.execute_query()
    }
    /// Parse, gate and execute the current query.
    pub fn execute_query(&mut self) -> QueryOutcome {
        let statement = match crate::sql::parse(self.query.trim()) {
            Ok(statement) => statement,
            Err(error) => {
                let out = QueryOutcome::Rejected {
                    message: error.message,
                };
                self.status = outcome_message(&out);
                let message = self
                    .status
                    .strip_prefix("Query rejected: ")
                    .map(str::to_owned);
                self.record_query_error(message.as_deref());
                self.surface = Surface::ErrorResult;
                return out;
            }
        };
        let table = match &statement {
            crate::sql::Statement::Select(select) => {
                self.catalog.find(select.schema.as_deref(), &select.table)
            }
            _ => None,
        };
        match crate::sql::gate(self.safe_mode, &statement) {
            crate::sql::Decision::Deny => {
                let risk = crate::sql::assess(&statement, table);
                let out = QueryOutcome::Denied {
                    summary: format!("{} is denied in Read-Only mode", risk.action),
                };
                self.status = outcome_message(&out);
                let message = self.status.clone();
                self.record_query_error(Some(&message));
                self.surface = Surface::ErrorResult;
                out
            }
            crate::sql::Decision::Confirm { deliberate } => {
                let risk = crate::sql::assess(&statement, table);
                let out = QueryOutcome::ConfirmationRequired {
                    deliberate,
                    summary: format!("{} · {}", risk.action, risk.scope),
                };
                self.status = outcome_message(&out);
                self.record_query_error(None);
                self.surface = Surface::SafetyDialogTypedAck;
                out
            }
            crate::sql::Decision::Run => {
                let crate::sql::Statement::Select(select) = statement else {
                    let out = QueryOutcome::Rejected {
                        message: "The demo executor only runs SELECT statements".to_owned(),
                    };
                    self.status = outcome_message(&out);
                    let message = self
                        .status
                        .strip_prefix("Query rejected: ")
                        .map(str::to_owned);
                    self.record_query_error(message.as_deref());
                    self.surface = Surface::ErrorResult;
                    return out;
                };
                match crate::sql::run_select(&self.catalog, &select) {
                    Ok(result) => {
                        let out = QueryOutcome::Executed {
                            rows: result.rows.len(),
                            editable: result.editable,
                        };
                        self.columns.clone_from(&result.columns);
                        self.result = ResultGrid::from_result(&result);
                        self.grid_state = GridState::default();
                        self.status = outcome_message(&out);
                        self.record_query_result();
                        self.surface = Surface::ResultsGrid;
                        out
                    }
                    Err(error) => {
                        let out = QueryOutcome::Rejected {
                            message: error.message,
                        };
                        self.status = outcome_message(&out);
                        let message = self
                            .status
                            .strip_prefix("Query rejected: ")
                            .map(str::to_owned);
                        self.record_query_error(message.as_deref());
                        self.surface = Surface::ErrorResult;
                        out
                    }
                }
            }
        }
    }
    fn record_query_result(&mut self) {
        let query = self.query.clone();
        let result = self.result.clone();
        if let Some(crate::tabs::Tab::Query(tab)) = self.workbench.active_mut() {
            tab.query = query;
            tab.result = Some(result);
            tab.error = None;
        }
    }
    fn record_query_error(&mut self, message: Option<&str>) {
        let query = self.query.clone();
        let message = message.map(str::to_owned);
        if let Some(crate::tabs::Tab::Query(tab)) = self.workbench.active_mut() {
            tab.query = query;
            tab.error = message;
            tab.result = None;
        }
    }
    fn column_specs(
        columns: &[(String, ColType)],
        editable: bool,
    ) -> ([tui_next::Column<'_>; tui_next::GRID_MAX_COLUMNS], usize) {
        let empty = tui_next::Column::new(tui_next::ColumnKey::num(0), "");
        let mut specs = [empty; tui_next::GRID_MAX_COLUMNS];
        let count = columns.len().min(tui_next::GRID_MAX_COLUMNS);
        for (index, (name, _)) in columns.iter().take(count).enumerate() {
            let mut col = tui_next::Column::new(
                tui_next::ColumnKey::num((index as u16).saturating_add(1)),
                name.as_str(),
            );
            col.sortable = true;
            col.editable = editable;
            col.sticky = index == 0;
            if let Some(slot) = specs.get_mut(index) {
                *slot = col;
            }
        }
        (specs, count)
    }
    fn connection_form<'a>(
        fields: &'a [tui_next::FieldSpec<'a>],
        actions: &'a [Action<'a>],
    ) -> Form<'a> {
        Form::new(connections::FORM, fields)
            .actions(actions)
            .submit(connections::SAVE_CONNECT)
    }
    fn handle_grid(&mut self, action: &GridAction) {
        match action {
            GridAction::Sort(key, direction) => {
                self.result.sort(*key, *direction);
                self.status = format!("Sorted column {}", key.raw());
            }
            GridAction::Copy(text) => {
                self.status = format!("Copied {} cells", text.lines().count());
            }
            GridAction::Activated(key) => self.status = format!("Activated row {key:?}"),
            GridAction::EditRequested(key, column) => {
                self.status = format!("Edit requested for {key:?}, column {column:?}");
            }
            GridAction::CellAction(key, column, action) => {
                self.status = format!("Cell action {action:?} on {key:?}/{column:?}");
            }
            GridAction::FetchMore => {
                "All deterministic demo rows are loaded".clone_into(&mut self.status);
            }
            GridAction::Moved | GridAction::LeaveForward | GridAction::LeaveBackward => {}
        }
    }

    fn connection_list<'a>() -> List<
        'a,
        Connection,
        impl Fn(&Connection) -> tui_next::ItemKey,
        impl Fn(&Connection, &mut tui_next::RowUi<'_>),
    > {
        List::<Connection>::new(CONNECTIONS)
            .key(|connection: &Connection| tui_next::ItemKey::text(&connection.name))
            .row(|connection, row| {
                row.label(&connection.name);
                row.meta(connection.environment.label());
            })
    }

    fn explorer_list<'a>() -> List<
        'a,
        crate::tabs::ExplorerItem,
        impl Fn(&crate::tabs::ExplorerItem) -> tui_next::ItemKey,
        impl Fn(&crate::tabs::ExplorerItem, &mut tui_next::RowUi<'_>),
    > {
        List::<crate::tabs::ExplorerItem>::new(EXPLORER)
            .key(|item: &crate::tabs::ExplorerItem| tui_next::ItemKey::text(&item.name))
            .row(|item, row| {
                if item.openable {
                    row.label_fmt(format_args!("{} › {}", item.schema, item.name));
                    row.meta(&item.rows.to_string());
                } else if item.schema.is_empty() {
                    row.label(&item.name);
                } else {
                    row.label_fmt(format_args!("▾ {}", item.name));
                    row.meta(&item.rows.to_string());
                }
            })
    }

    fn tab_strip<'a>() -> Tabs<
        'a,
        &'a str,
        impl Fn(&&'a str) -> tui_next::ItemKey,
        impl Fn(&&'a str, &mut tui_next::RowUi<'_>),
    > {
        Tabs::<&'a str>::new(TAB_STRIP)
            .key(|label| tui_next::ItemKey::text(label))
            .row(|label, row| row.label(label))
            .allow_new(true)
            .closable(true)
    }

    fn tab_labels(tabs: &[crate::tabs::Tab]) -> ([&str; MAX_TAB_LABELS], usize) {
        let mut labels = [""; MAX_TAB_LABELS];
        let count = tabs.len().min(MAX_TAB_LABELS);
        for (index, tab) in tabs.iter().take(count).enumerate() {
            if let Some(slot) = labels.get_mut(index) {
                *slot = tab.label_ref();
            }
        }
        (labels, count)
    }

    fn sync_result_from_active_tab(&mut self) {
        let Some(tab) = self.workbench.active() else {
            return;
        };
        match tab {
            crate::tabs::Tab::Table(table) => {
                self.columns = table
                    .table
                    .columns
                    .iter()
                    .map(|column| (column.name.clone(), column.ty))
                    .collect();
                self.result = table.result.clone();
            }
            crate::tabs::Tab::Query(query) => {
                if let Some(result) = query.result.as_ref() {
                    self.result = result.clone();
                }
            }
            crate::tabs::Tab::History(_) => {}
        }
        self.grid_state = GridState::default();
    }
}

/// Result of executing a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOutcome {
    /// Query returned rows.
    Executed {
        /// Number of returned rows.
        rows: usize,
        /// Whether the result supports edits.
        editable: bool,
    },
    /// Safety gate requires confirmation.
    ConfirmationRequired {
        /// Whether the target must be typed.
        deliberate: bool,
        /// Human-readable risk summary.
        summary: String,
    },
    /// Read-only policy denied a write.
    Denied {
        /// Human-readable denial summary.
        summary: String,
    },
    /// Parse/execution rejection.
    Rejected {
        /// Human-readable error message.
        message: String,
    },
}

fn outcome_message(outcome: &QueryOutcome) -> String {
    match outcome {
        QueryOutcome::Executed { rows, editable } => format!(
            "Loaded {rows} rows{}",
            if *editable {
                " · editable"
            } else {
                " · read-only"
            }
        ),
        QueryOutcome::ConfirmationRequired {
            deliberate,
            summary,
        } => format!(
            "Confirmation required{}: {summary}",
            if *deliberate { " · type target" } else { "" }
        ),
        QueryOutcome::Denied { summary } => summary.clone(),
        QueryOutcome::Rejected { message } => format!("Query rejected: {message}"),
    }
}

fn query_input(value: Option<&str>) -> TextInput<'_> {
    let input = TextInput::new(QUERY).placeholder("SELECT …");
    match value {
        Some(text) => input.value(text),
        None => input,
    }
}
fn header_panel() -> Panel<'static> {
    Panel::new(HEADER).kind(PanelKind::Framed)
}
fn results_panel() -> Panel<'static> {
    Panel::new(RESULTS).kind(PanelKind::Framed).title("Results")
}
fn connections_panel() -> Panel<'static> {
    Panel::new(CONNECTIONS_PANEL)
        .kind(PanelKind::Framed)
        .title("Connections")
}
fn explorer_panel() -> Panel<'static> {
    Panel::new(EXPLORER_PANEL)
        .kind(PanelKind::Framed)
        .title("Explorer")
}
fn status_bar<'a>(left: &'a [StatusItem<'a>], right: &'a [StatusItem<'a>]) -> StatusBar<'a> {
    StatusBar::new(STATUS)
        .left(left)
        .right(right)
        .variant(Variant::DEFAULT)
}
fn result_grid<'a>(columns: &'a [tui_next::Column<'a>]) -> Grid<'a> {
    Grid::new(RESULTS, columns)
        .nav(tui_next::NavUnit::Cell)
        .select_mode(tui_next::SelectMode::Multi)
}

impl App for TableProApp {
    #[expect(
        clippy::too_many_lines,
        reason = "Event routing remains one audited state machine for legacy parity."
    )]
    fn update(&mut self, cx: &mut Cx<'_>) -> Response<()> {
        let mut response = Response::ignored();
        if matches!(
            cx.update_cause(),
            UpdateCause::Bootstrap | UpdateCause::Event
        ) && let Some(command) = cx.command()
        {
            match command {
                c if c == QUIT => {
                    self.quit = true;
                    cx.quit();
                    response |= Response::consumed();
                }
                c if c == RUN => {
                    let _ = self.execute_query();
                    response |= Response::changed();
                }
                c if c == OPEN => {
                    self.surface = Surface::QuickSwitcher;
                    response |= Response::changed();
                }
                c if c == NEW_QUERY => {
                    self.query.clear();
                    self.query_state = TextInputState::default();
                    self.result = ResultGrid::empty();
                    self.columns.clear();
                    self.workbench.new_query("");
                    self.surface = Surface::QueryEditing;
                    response |= Response::changed();
                }
                c if c == HISTORY => {
                    self.workbench.open_history();
                    self.surface = Surface::HistoryTab;
                    response |= Response::changed();
                }
                c if c == EXPLAIN => {
                    let catalog = self.workbench.catalog.clone();
                    let planned = self.workbench.active_mut().and_then(|tab| match tab {
                        crate::tabs::Tab::Query(query) => Some(query.explain(&catalog)),
                        _ => None,
                    });
                    if matches!(planned, Some(Ok(()))) {
                        self.surface = Surface::ExplainPlan;
                        "Explain plan ready".clone_into(&mut self.status);
                    }
                    response |= Response::changed();
                }
                c if c == STRUCTURE => {
                    let _ = self.workbench.toggle_structure();
                    self.surface = Surface::StructureView;
                    response |= Response::changed();
                }
                c if c == FILTER => {
                    if self.workbench.active_table().is_some() {
                        self.surface = Surface::FilterEditor;
                        response |= Response::changed();
                    }
                }
                c if c == TAB_LIST => {
                    self.surface = Surface::TabListPicker;
                    response |= Response::changed();
                }
                c if c == FORM => {
                    self.begin_connection_form();
                    response |= Response::changed();
                }
                c if c == HELP => {
                    self.surface = Surface::HelpDialog;
                    response |= Response::changed();
                }
                c if c == SAFE_MODE => {
                    let modes = &SafeMode::ALL;
                    let current = modes
                        .iter()
                        .position(|mode| *mode == self.safe_mode)
                        .unwrap_or(0);
                    self.set_safe_mode(
                        modes
                            .get(if current.saturating_add(1) >= modes.len() {
                                0
                            } else {
                                current.saturating_add(1)
                            })
                            .copied()
                            .unwrap_or(SafeMode::Silent),
                    );
                    response |= Response::changed();
                }
                c if c == MAXIMIZE => {
                    self.workbench.toggle_maximized();
                    self.surface = Surface::MaximisedTab;
                    response |= Response::changed();
                }
                _ => {}
            }
        }
        // Props are built once through these constructors and reused by both
        // update and draw.  This keeps the app phase-pure and avoids a
        // render-time configuration drift between the two paths.
        let _ = (
            header_panel(),
            results_panel(),
            connections_panel(),
            explorer_panel(),
            status_bar(&[], &[]),
        );
        if self.form_open {
            let fields = &self.form_fields;
            let actions = &self.form_actions;
            if let Some(draft) = self.draft.as_mut() {
                let form = Self::connection_form(fields, actions);
                let form_response = form.update(cx, &mut self.form_state, draft);
                if let Some(action) = form_response.action_ref() {
                    match action {
                        FormAction::Action(ActionKey::CANCEL) => {
                            self.form_open = false;
                            self.draft = None;
                        }
                        FormAction::Action(ActionKey::SAVE | connections::SAVE_CONNECT) => {
                            if draft.validate_all().is_ok()
                                && let Some(connection) =
                                    draft.to_connection(Some(&self.connection))
                            {
                                self.connections.push(connection.clone());
                                if action == &FormAction::Action(connections::SAVE_CONNECT) {
                                    let _ = self.connect(self.connections.len().saturating_sub(1));
                                    self.form_open = false;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                response |= form_response.erase();
            }
            return response;
        }

        if self.screen == Screen::Connections {
            let list = Self::connection_list();
            let list_response = list.update(cx, &mut self.connection_list_state, &self.connections);
            if let Some(ListAction::Activated(key)) = list_response.action_ref().copied()
                && let Some(index) = self
                    .connections
                    .iter()
                    .position(|connection| tui_next::ItemKey::text(&connection.name) == key)
            {
                let _ = self.connect(index);
                cx.focus(EXPLORER);
            }
            response |= list_response.erase();
            return response;
        }

        let explorer = Self::explorer_list();
        let explorer_response =
            explorer.update(cx, &mut self.explorer_list_state, &self.workbench.explorer);
        if let Some(ListAction::Activated(key)) = explorer_response.action_ref().copied()
            && let Some(index) = self
                .workbench
                .explorer
                .iter()
                .position(|item| tui_next::ItemKey::text(&item.name) == key)
        {
            self.workbench.explorer_selected = index;
            if self.workbench.open_selected() {
                self.sync_result_from_active_tab();
                self.surface = Surface::TableGrid;
                cx.focus(RESULTS);
            }
        }
        response |= explorer_response.erase();

        let tabs_action = {
            let (labels, label_count) = Self::tab_labels(&self.workbench.tabs);
            let labels = labels.get(..label_count).unwrap_or(&[]);
            let tabs = Self::tab_strip();
            let tabs_response = tabs.update(cx, &mut self.tab_state, labels);
            let action = tabs_response.action_ref().copied();
            response |= tabs_response.erase();
            action
        };
        match tabs_action {
            Some(TabsAction::Activated(key)) => {
                if let Some(index) = self
                    .workbench
                    .tabs
                    .iter()
                    .position(|candidate| tui_next::ItemKey::text(candidate.label_ref()) == key)
                {
                    self.workbench.active = index;
                    self.sync_result_from_active_tab();
                }
            }
            Some(TabsAction::Close(key)) => {
                if let Some(index) = self
                    .workbench
                    .tabs
                    .iter()
                    .position(|candidate| tui_next::ItemKey::text(candidate.label_ref()) == key)
                {
                    let _ = self.workbench.close_tab(index);
                    self.sync_result_from_active_tab();
                }
            }
            Some(TabsAction::New) => {
                self.workbench.new_query("");
                self.surface = Surface::QueryEditing;
            }
            None => {}
        }
        let query_response = query_input(None)
            .update(cx, &mut self.query_state, &mut self.query)
            .erase();
        if let Some(crate::tabs::Tab::Query(query)) = self.workbench.active_mut() {
            query.query.clone_from(&self.query);
        }
        response |= query_response;
        let editable = self.result.is_editable();
        let (columns, column_count) = Self::column_specs(&self.columns, editable);
        let grid = result_grid(columns.get(..column_count).unwrap_or(&[]));
        let grid_response = if self.result.is_editable() {
            grid.update_editable(cx, &mut self.grid_state, &mut self.result)
        } else {
            grid.update(cx, &mut self.grid_state, &self.result)
        };
        let sync_table = grid_response.action_ref().is_some_and(|action| {
            !matches!(
                action,
                GridAction::Moved | GridAction::LeaveForward | GridAction::LeaveBackward
            )
        });
        if let Some(action) = grid_response.action_ref() {
            self.handle_grid(action);
        }
        if sync_table {
            let result = self.result.clone();
            if let Some(crate::tabs::Tab::Table(table)) = self.workbench.active_mut() {
                table.result = result;
            }
        }
        response |= grid_response.erase();
        response
    }
    fn draw(&self, ui: &mut Ui<'_>) {
        let rows = tui_next::layout::rows(
            ui.full(),
            &[
                tui_next::Track::Fixed(3),
                tui_next::Track::Flex(1),
                tui_next::Track::Fixed(1),
            ],
        );
        let header = rows.first().copied().unwrap_or_else(|| ui.full());
        let body = rows.get(1).copied().unwrap_or_else(|| ui.full());
        let footer = rows.get(2).copied().unwrap_or_else(|| ui.full());
        if self.form_open {
            header_panel()
                .title("Connect to database")
                .draw(ui, ui.full(), |ui, area| {
                    if let Some(draft) = self.draft.as_ref() {
                        Self::connection_form(&self.form_fields, &self.form_actions).draw(
                            ui,
                            area,
                            &self.form_state,
                            draft,
                        );
                    }
                });
        } else if self.screen == Screen::Connections {
            ui.paint_str(header, "TablePro · Connections", ui.surface_style());
            connections_panel().draw(ui, body, |ui, area| {
                Self::connection_list().draw(
                    ui,
                    area,
                    &self.connection_list_state,
                    &self.connections,
                );
            });
        } else {
            let title = format!("TablePro · {}", self.connection.environment.label());
            ui.paint_str(header, &title, ui.surface_style());
            let workbench_rows = tui_next::layout::rows(
                body,
                &[
                    tui_next::Track::Fixed(3),
                    tui_next::Track::Fixed(2),
                    tui_next::Track::Flex(1),
                ],
            );
            let query_area = workbench_rows.first().copied().unwrap_or(body);
            let tabs_area = workbench_rows.get(1).copied().unwrap_or(body);
            let grid_area = workbench_rows.get(2).copied().unwrap_or(body);
            let panes = tui_next::layout::columns(
                grid_area,
                &[tui_next::Track::Fixed(28), tui_next::Track::Flex(1)],
                1,
            );
            let explorer_area = panes.first().copied().unwrap_or(grid_area);
            let result_area = panes.get(1).copied().unwrap_or(grid_area);
            explorer_panel().draw(ui, explorer_area, |ui, area| {
                Self::explorer_list().draw(
                    ui,
                    area,
                    &self.explorer_list_state,
                    &self.workbench.explorer,
                );
            });
            // Keep the explorer as the first workbench stop.  The legacy
            // route enters the explorer after connect and tabs into the query
            // editor; draw order is the public runtime's focus-ring order.
            Field::new("SQL query", query_input(Some(&self.query)))
                .plain(true)
                .draw(ui, query_area, &self.query_state);
            let (labels, label_count) = Self::tab_labels(&self.workbench.tabs);
            let labels = labels.get(..label_count).unwrap_or(&[]);
            Self::tab_strip().draw(ui, tabs_area, &self.tab_state, labels);
            let (columns, column_count) =
                Self::column_specs(&self.columns, self.result.is_editable());
            let grid = result_grid(columns.get(..column_count).unwrap_or(&[]));
            let meta = format!(
                "{} rows · {}",
                self.result.total(),
                self.result.source().unwrap_or("no relation")
            );
            results_panel()
                .meta(&meta)
                .draw(ui, result_area, |ui, area| {
                    grid.draw(ui, area, &self.grid_state, &self.result);
                });
        }
        let safe_token = self.safe_mode.token();
        let pending = self.result.pending_total();
        let pending_label = format!("• {pending} pending");
        let left = [
            StatusItem::new(&self.connection.name).strong(),
            StatusItem::new(safe_token),
            StatusItem::new(&pending_label),
        ];
        let right = [StatusItem::new(&self.status)];
        status_bar(&left, &right).draw(ui, footer);
    }
    fn should_quit(&self) -> bool {
        self.quit
    }
    fn keymap(&self) -> &KeyMap {
        &self.keymap
    }
    fn min_size(&self) -> Size {
        Size {
            min: (MIN_WIDTH, MIN_HEIGHT),
            preferred: (120, 36),
        }
    }
    fn on_esc(&mut self, _cx: &mut Cx<'_>) -> Response<()> {
        if self.form_open {
            self.form_open = false;
            self.draft = None;
            self.surface = Surface::Connections;
            Response::changed()
        } else {
            Response::ignored()
        }
    }
}

/// Start the interactive `TablePro` binary.
///
/// # Errors
///
/// Returns terminal setup or runtime I/O errors.
pub fn run() -> std::io::Result<()> {
    run_with(tui_next::Theme::junie(), None)
}

/// Start `TablePro` with an explicit theme and optional initial connection.
///
/// # Errors
///
/// Returns an invalid-input error when `connect` names no configured
/// connection, or the terminal setup/runtime I/O error from `tui_next`.
pub fn run_with(theme: tui_next::Theme, connect: Option<&str>) -> std::io::Result<()> {
    let mut app = TableProApp::default();
    if let Some(name) = connect {
        let Some(index) = app
            .connections
            .iter()
            .position(|candidate| candidate.name.eq_ignore_ascii_case(name))
        else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("no connection named {name:?}"),
            ));
        };
        let _ = app.connect(index);
    }
    tui_next::run(app, theme)
}
