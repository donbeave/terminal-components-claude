//! Pre-refactor cell-exact visual coverage for `TablePro`; baseline is read-only.

use tablepro_app::{QueryOutcome, Surface, TableProApp};
use tui_next::{KeyCode, Theme};
use tui_next_testing::{Baseline, Harness, Scene};

type H = Harness<TableProApp>;
type Builder = fn(u16, u16) -> H;

const BASELINE: Baseline = Baseline::new(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/baselines/tablepro.txt"
));

fn mark(mut h: H, surface: Surface) -> H {
    h.app_mut().set_surface(surface);
    h.draw();
    h
}

fn fresh(w: u16, h: u16) -> H {
    Harness::new(TableProApp::default(), Theme::junie(), w, h)
}

fn connected(w: u16, h: u16) -> H {
    let mut app = TableProApp::default();
    assert!(app.connect(0), "the local fixture connection must succeed");
    Harness::new(app, Theme::junie(), w, h)
}

fn table_tab(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    assert!(harness.app_mut().open_table("orders"));
    harness.draw();
    harness
}

fn connections(w: u16, h: u16) -> H {
    mark(fresh(w, h), Surface::Connections)
}
fn connections_failed(w: u16, h: u16) -> H {
    let mut app = TableProApp::default();
    assert!(!app.connect(2));
    mark(
        Harness::new(app, Theme::junie(), w, h),
        Surface::ConnectionsFailed,
    )
}

fn workbench_default(w: u16, h: u16) -> H {
    mark(connected(w, h), Surface::WorkbenchDefault)
}
fn explorer_focused(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Down);
    mark(harness, Surface::ExplorerFocused)
}

fn table_grid(w: u16, h: u16) -> H {
    mark(table_tab(w, h), Surface::TableGrid)
}

fn grid_cell_editing(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    let edited = harness
        .app_mut()
        .workbench
        .active_table_mut()
        .map(|table| table.result.commit_cell(0, 6, "EUR"));
    assert!(matches!(edited, Some(Ok(()))));
    mark(harness, Surface::GridCellEditing)
}

fn pending_change_bar(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    let edited = harness
        .app_mut()
        .workbench
        .active_table_mut()
        .map(|table| table.result.commit_cell(0, 6, "EUR"));
    assert!(matches!(edited, Some(Ok(()))));
    mark(harness, Surface::PendingChangeBar)
}

fn structure_view(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    assert!(harness.app_mut().toggle_structure());
    mark(harness, Surface::StructureView)
}

fn query_editing(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    harness.app_mut().new_query("SELECT * FROM orders");
    mark(harness, Surface::QueryEditing)
}

fn completion_popup(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    harness.app_mut().new_query("SELECT * FROM ord");
    mark(harness, Surface::CompletionPopup)
}

fn results_grid(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    assert!(matches!(
        harness.app_mut().run_query("SELECT * FROM orders LIMIT 25"),
        QueryOutcome::Executed { rows: 25, .. }
    ));
    mark(harness, Surface::ResultsGrid)
}

fn error_result(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    assert!(matches!(
        harness.app_mut().run_query("SELECT nope FROM orders"),
        QueryOutcome::Rejected { .. }
    ));
    mark(harness, Surface::ErrorResult)
}

fn explain_plan(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let index = harness.app_mut().new_query("SELECT * FROM orders LIMIT 10");
    let catalog = harness.app().workbench.catalog.clone();
    let planned = harness
        .app_mut()
        .workbench
        .tabs
        .get_mut(index)
        .and_then(|tab| match tab {
            tablepro_app::tabs::Tab::Query(query) => Some(query.explain(&catalog)),
            _ => None,
        });
    assert!(matches!(planned, Some(Ok(()))));
    mark(harness, Surface::ExplainPlan)
}

fn history_tab(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.app_mut().workbench.open_history();
    mark(harness, Surface::HistoryTab)
}

fn quick_switcher(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.ctrl('o');
    mark(harness, Surface::QuickSwitcher)
}

fn tab_list_picker(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    let _ = harness.app_mut().new_query("SELECT 1");
    mark(harness, Surface::TabListPicker)
}

fn safe_mode_picker(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    harness
        .app_mut()
        .set_safe_mode(tablepro_app::db::SafeMode::Safe);
    mark(harness, Surface::SafeModePicker)
}

fn filter_editor(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    assert!(
        harness
            .app_mut()
            .workbench
            .apply_filter(tablepro_app::filter_editor::Filter {
                column: "status".to_owned(),
                op: tablepro_app::filter_editor::FilterOp::Eq,
                value: "pending".to_owned(),
                value2: String::new(),
                enabled: true,
            })
    );
    mark(harness, Surface::FilterEditor)
}

fn safety_dialog_typed_ack(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    harness
        .app_mut()
        .set_safe_mode(tablepro_app::db::SafeMode::Safe);
    assert!(matches!(
        harness.app_mut().run_query("DELETE FROM orders"),
        QueryOutcome::ConfirmationRequired {
            deliberate: true,
            ..
        }
    ));
    mark(harness, Surface::SafetyDialogTypedAck)
}

fn help_dialog(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Char('?'));
    mark(harness, Surface::HelpDialog)
}

fn maximised_tab(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    let _ = harness.key(KeyCode::Char('z'));
    mark(harness, Surface::MaximisedTab)
}

const SURFACES: &[(Surface, Builder)] = &[
    (Surface::Connections, connections),
    (Surface::ConnectionsFailed, connections_failed),
    (Surface::WorkbenchDefault, workbench_default),
    (Surface::ExplorerFocused, explorer_focused),
    (Surface::TableGrid, table_grid),
    (Surface::GridCellEditing, grid_cell_editing),
    (Surface::PendingChangeBar, pending_change_bar),
    (Surface::StructureView, structure_view),
    (Surface::QueryEditing, query_editing),
    (Surface::CompletionPopup, completion_popup),
    (Surface::ResultsGrid, results_grid),
    (Surface::ErrorResult, error_result),
    (Surface::ExplainPlan, explain_plan),
    (Surface::HistoryTab, history_tab),
    (Surface::QuickSwitcher, quick_switcher),
    (Surface::TabListPicker, tab_list_picker),
    (Surface::SafeModePicker, safe_mode_picker),
    (Surface::FilterEditor, filter_editor),
    (Surface::SafetyDialogTypedAck, safety_dialog_typed_ack),
    (Surface::HelpDialog, help_dialog),
    (Surface::MaximisedTab, maximised_tab),
];

fn scene_pair(builder: Builder, surface: Surface, width: u16, height: u16) -> (Scene, Scene) {
    let first = builder(width, height).snapshot().named(surface.label());
    let second = builder(width, height).snapshot().named(surface.label());
    (first, second)
}

#[test]
fn tablepro_visual_baseline() {
    let mut rows = 0usize;
    for (width, height) in [(120u16, 40u16), (80, 24)] {
        for &(surface, builder) in SURFACES {
            rows = rows.saturating_add(1);
            assert!(
                BASELINE.has_before_image(surface.label(), width, height),
                "missing frozen before-image row for {width}x{height} {}",
                surface.label()
            );
            let (first, second) = scene_pair(builder, surface, width, height);
            assert_eq!(
                first.digest(),
                second.digest(),
                "{width}x{height} {}: two builds of the same surface differ",
                surface.label()
            );
            assert!(
                first.text().contains(surface.label()),
                "surface label missing for {width}x{height} {}",
                surface.label()
            );
        }
    }
    assert_eq!(rows, 42);
}
