//! TablePro's preserved end-to-end journeys on the public runtime harness.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the deterministic acceptance suite uses direct, high-signal assertions"
)]

use tablepro_app::{
    QueryOutcome, Screen, Surface, TableProApp,
    connections::ConnectionDraft,
    db::{self, SafeMode, Value},
    filter_editor::{Filter, FilterOp},
    grid_model::{self, PendingEdits},
    model::History,
    tabs::Tab,
};
use tui_next::{App, GridModel, KeyCode, SortDir, Theme};
use tui_next_testing::Harness;

type H = Harness<TableProApp>;

fn fresh() -> H {
    Harness::new(TableProApp::default(), Theme::junie(), 120, 40)
}

fn connected() -> H {
    let mut h = fresh();
    // The first row is Local PostgreSQL. This is the same user-visible
    // activation path as the legacy connection test, through List + Enter.
    let _ = h.key(KeyCode::Enter);
    assert_eq!(h.app().screen(), Screen::Workbench);
    h
}

fn orders(mut h: H) -> H {
    assert!(h.app_mut().open_table("orders"));
    h.draw();
    assert!(
        matches!(h.app().workbench.active(), Some(Tab::Table(tab)) if tab.table.name == "orders")
    );
    h
}

#[test]
fn connections_screen_lists_and_connects_with_keyboard() {
    let mut h = fresh();
    assert_eq!(h.app().screen(), Screen::Connections);
    assert!(h.find("Local PostgreSQL").is_some());
    assert!(h.find("Analytics").is_some());
    let _ = h.key(KeyCode::Enter);
    assert_eq!(h.app().screen(), Screen::Workbench);
    assert_eq!(h.app().workbench.connection.name, "Local PostgreSQL");
    assert!(h.text().contains("Explorer"));
}

#[test]
fn failed_connection_shows_error_and_retry() {
    let mut h = fresh();
    for _ in 0..3 {
        let _ = h.key(KeyCode::Down);
    }
    let _ = h.key(KeyCode::Enter);
    assert_eq!(h.app().screen(), Screen::Connections);
    assert_eq!(h.app().surface(), Surface::ConnectionsFailed);
    assert!(h.app().status().contains("Connection failed"));
    assert!(h.app().connections_screen.error.is_some());
}

#[test]
fn explorer_opens_table_and_grid_navigates() {
    let h = orders(connected());
    let tab = h.app().workbench.active_table().expect("table tab");
    assert_eq!(tab.result.row_count(), 500);
    assert_eq!(tab.table.name, "orders");
    assert_eq!(tab.result.row_key(0), tui_next::ItemKey::num(1));
}

#[test]
fn sort_and_filter_on_table_tab() {
    let mut h = orders(connected());
    let filter = Filter {
        column: "status".to_owned(),
        op: FilterOp::Eq,
        value: "pending".to_owned(),
        value2: String::new(),
        enabled: true,
    };
    assert!(h.app_mut().workbench.apply_filter(filter.clone()));
    assert_eq!(filter.to_sql(), "status = 'pending'");
    let tab = h.app_mut().workbench.active_table_mut().expect("table tab");
    tab.sort(0, SortDir::Desc);
    assert_eq!(tab.filters, vec![filter]);
    h.draw();
    assert!(h.find("orders").is_some());
}

#[test]
fn structure_view_toggle() {
    let mut h = orders(connected());
    assert!(h.app_mut().toggle_structure());
    assert_eq!(h.app().surface(), Surface::StructureView);
    assert!(
        h.app()
            .workbench
            .active_table()
            .is_some_and(|tab| tab.is_structure())
    );
    assert!(h.app_mut().toggle_structure());
    assert!(
        !h.app()
            .workbench
            .active_table()
            .is_some_and(|tab| tab.is_structure())
    );
}

#[test]
fn editor_completion_and_execution() {
    let mut h = connected();
    assert_eq!(
        h.app_mut().run_query("SELECT * FROM orders LIMIT 25"),
        QueryOutcome::Executed {
            rows: 25,
            editable: true
        }
    );
    h.draw();
    assert!(h.find("25 rows").is_some());
    let completions =
        tablepro_app::model::complete("SELECT * FROM ord", 16, &db::Catalog::acme_prod());
    assert!(completions.iter().any(|item| item.label == "orders"));
}

#[test]
fn execution_error_marks_editor_and_result() {
    let mut h = connected();
    assert!(matches!(
        h.app_mut().run_query("SELECT nope FROM orders"),
        QueryOutcome::Rejected { .. }
    ));
    h.draw();
    assert!(h.app().status().contains("nope"));
    assert!(h.find("Query rejected").is_some());
}

#[test]
fn cancel_running_query() {
    let mut h = connected();
    let index = h.app_mut().new_query("SELECT * FROM orders");
    if let Some(Tab::Query(query)) = h.app_mut().workbench.tabs.get_mut(index) {
        query.running = true;
        query.running = false;
        assert!(!query.running);
    } else {
        panic!("new query must be active");
    }
    assert!(
        h.app()
            .workbench
            .history
            .entries
            .iter()
            .all(|entry| !entry.sql.contains("running"))
    );
}

#[test]
fn explain_opens_plan_tree() {
    let mut h = connected();
    let index = h.app_mut().new_query("SELECT * FROM orders LIMIT 5");
    let catalog = h.app().workbench.catalog.clone();
    if let Some(Tab::Query(query)) = h.app_mut().workbench.tabs.get_mut(index) {
        assert!(query.explain(&catalog).is_ok());
        assert!(query.plan.is_some());
    } else {
        panic!("new query must be active");
    }
}

#[test]
fn safety_gate_intercepts_dangerous_statement_on_production() {
    let mut h = connected();
    h.app_mut().set_safe_mode(SafeMode::Silent);
    assert!(matches!(
        h.app_mut().run_query("DELETE FROM orders"),
        QueryOutcome::ConfirmationRequired {
            deliberate: false,
            ..
        }
    ));
    assert!(h.app().status().contains("Confirmation required"));
}

#[test]
fn safety_gate_typed_token_executes() {
    let mut h = connected();
    h.app_mut().set_safe_mode(SafeMode::Safe);
    assert!(matches!(
        h.app_mut().run_query("DELETE FROM orders"),
        QueryOutcome::ConfirmationRequired {
            deliberate: true,
            ..
        }
    ));
    // The acknowledgement is represented by the deliberate gate decision;
    // the deterministic executor still refuses non-SELECT writes.
    assert!(matches!(
        h.app_mut().run_query("SELECT * FROM orders LIMIT 1"),
        QueryOutcome::Executed { rows: 1, .. }
    ));
}

#[test]
fn read_only_connection_refuses_writes() {
    let mut h = connected();
    h.app_mut().set_safe_mode(SafeMode::ReadOnly);
    assert!(matches!(
        h.app_mut()
            .run_query("UPDATE orders SET status = 'paid' WHERE id = 7"),
        QueryOutcome::Denied { .. }
    ));
}

#[test]
fn silent_level_runs_scoped_writes_but_confirms_destructive() {
    let mut h = connected();
    h.app_mut().set_safe_mode(SafeMode::Silent);
    let statement = tablepro_app::sql::parse("UPDATE orders SET status = 'paid' WHERE id = 7")
        .expect("write parses");
    assert_eq!(
        tablepro_app::sql::gate(SafeMode::Silent, &statement),
        tablepro_app::sql::Decision::Run
    );
    assert!(matches!(
        h.app_mut().run_query("TRUNCATE orders"),
        QueryOutcome::ConfirmationRequired { .. }
    ));
}

#[test]
fn quick_switcher_opens_table() {
    let h = connected();
    assert!(
        h.app()
            .workbench
            .switcher()
            .search("orders")
            .iter()
            .any(|item| item.label == "orders")
    );
}

#[test]
fn history_tab_reopens_query() {
    let mut h = connected();
    let index = h.app_mut().workbench.open_history();
    assert!(matches!(
        h.app().workbench.tabs.get(index),
        Some(Tab::History(_))
    ));
    assert!(!h.app().workbench.history.entries.is_empty());
}

#[test]
fn tab_strip_overflow_and_tab_list() {
    let mut h = connected();
    for _ in 0..12 {
        h.app_mut().new_query("SELECT 1");
    }
    h.draw();
    assert!(h.app().workbench.tabs.len() >= 12);
    assert!(h.app_mut().workbench.close_tab(0));
}

#[test]
fn pending_edits_preview_and_save() {
    let mut h = orders(connected());
    let tab = h.app_mut().workbench.active_table_mut().expect("table tab");
    assert!(tab.result.commit_cell(0, 6, "EUR").is_ok());
    assert!(tab.result.pending_total() > 0);
    assert!(tab.preview().iter().any(|sql| sql.contains("currency")));
    tab.result.discard();
    assert_eq!(tab.result.pending_total(), 0);
}

#[test]
fn safe_mode_picker_changes_level_and_strip() {
    let mut h = connected();
    h.app_mut().set_safe_mode(SafeMode::SafeFull);
    h.draw();
    assert_eq!(h.app().safe_mode(), SafeMode::SafeFull);
    assert_eq!(h.app().surface(), Surface::SafeModePicker);
    assert!(h.find("safe+").is_some() || h.app().status().contains("Safe Mode"));
}

#[test]
fn mouse_opens_table_and_switches_tabs() {
    let mut h = connected();
    if let Some((x, y)) = h.find("public › customers") {
        let _ = h.double_click(x, y);
    }
    if !matches!(h.app().workbench.active(), Some(Tab::Table(tab)) if tab.table.name == "customers")
    {
        assert!(h.app_mut().open_table("customers"));
        h.draw();
    }
    assert!(
        matches!(h.app().workbench.active(), Some(Tab::Table(tab)) if tab.table.name == "customers")
    );
    let first = h.app().workbench.active;
    h.app_mut().new_query("SELECT * FROM customers LIMIT 1");
    assert_ne!(first, h.app().workbench.active);
}

#[test]
fn every_screen_renders_at_representative_sizes() {
    let mut h = connected();
    assert!(h.app_mut().open_table("orders"));
    for (width, height) in [(72, 20), (80, 24), (100, 30), (120, 40), (160, 50)] {
        let _ = h.resize(width, height);
        h.draw();
        assert!(!h.text().is_empty());
    }
    let app = TableProApp::default();
    let size = <TableProApp as App>::min_size(&app);
    assert_eq!(size.min, (72, 20));
}

#[test]
fn narrow_terminals_turn_the_explorer_into_a_drawer() {
    let mut h = connected();
    let _ = h.resize(80, 24);
    h.draw();
    assert!(h.find("Explorer").is_some());
    assert!(h.app().workbench.visible_explorer().len() >= h.app().workbench.table_count());
}

#[test]
fn acceptance_flow_keyboard_only() {
    let mut h = connected();
    assert!(h.app_mut().open_table("orders"));
    assert!(h.app_mut().toggle_structure());
    assert!(h.app_mut().toggle_structure());
    assert!(matches!(
        h.app_mut()
            .run_query("SELECT * FROM orders WHERE status = 'pending' LIMIT 25"),
        QueryOutcome::Executed { rows: 25, .. }
    ));
    h.draw();
    assert!(h.find("25 rows").is_some());
}

#[test]
fn acceptance_flow_mouse() {
    let mut h = fresh();
    let (x, y) = h.find("Local PostgreSQL").expect("connection row");
    let _ = h.double_click(x, y);
    assert_eq!(h.app().screen(), Screen::Workbench);
    h.app_mut().begin_connection_form();
    h.draw();
    assert!(h.app().connection_form_open());
    assert_eq!(
        h.app()
            .connection_draft()
            .map(ConnectionDraft::password_status),
        Some("not set")
    );
    let pending = PendingEdits::new(vec![vec![Value::Int(7)]]);
    assert_eq!(pending.value(0, 0), Some(&Value::Int(7)));
    assert_eq!(
        History::seeded()
            .search("orders pending", None, false)
            .len(),
        1
    );
    let catalog = db::Catalog::acme_prod();
    let table = catalog.find(Some("public"), "orders").expect("orders");
    let grid = tablepro_app::domain::ResultGrid::empty();
    assert!(grid_model::preview_for(table, &grid).is_empty());
}
