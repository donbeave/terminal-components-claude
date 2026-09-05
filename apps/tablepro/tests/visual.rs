//! Pre-refactor cell-exact visual coverage for `TablePro`; baseline is read-only.

use tablepro_app::{Surface, TableProApp};
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
    let mut harness = fresh(w, h);
    let _ = harness.key(KeyCode::Enter);
    assert_eq!(harness.app().screen(), tablepro_app::Screen::Workbench);
    harness
}

fn table_tab(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    for _ in 0..5 {
        let _ = harness.key(KeyCode::Down);
    }
    let _ = harness.key(KeyCode::Enter);
    assert!(matches!(
        harness.app().workbench.active(),
        Some(tablepro_app::tabs::Tab::Table(table)) if table.table.name == "orders"
    ));
    harness
}

fn connections(w: u16, h: u16) -> H {
    mark(fresh(w, h), Surface::Connections)
}
fn connections_failed(w: u16, h: u16) -> H {
    let mut harness = fresh(w, h);
    for _ in 0..3 {
        let _ = harness.key(KeyCode::Down);
    }
    let _ = harness.key(KeyCode::Enter);
    assert_eq!(harness.app().screen(), tablepro_app::Screen::Connections);
    mark(harness, Surface::ConnectionsFailed)
}

fn workbench_default(w: u16, h: u16) -> H {
    mark(connected(w, h), Surface::WorkbenchDefault)
}
fn explorer_focused(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    for _ in 0..2 {
        let _ = harness.key(KeyCode::Down);
    }
    mark(harness, Surface::ExplorerFocused)
}

fn table_grid(w: u16, h: u16) -> H {
    mark(table_tab(w, h), Surface::TableGrid)
}

fn grid_cell_editing(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    for _ in 0..6 {
        let _ = harness.key(KeyCode::Right);
    }
    let _ = harness.key(KeyCode::Enter);
    let _ = harness.ctrl('l');
    let _ = harness.type_str("EUR");
    mark(harness, Surface::GridCellEditing)
}

fn pending_change_bar(w: u16, h: u16) -> H {
    let mut harness = grid_cell_editing(w, h);
    let _ = harness.key(KeyCode::Enter);
    assert!(harness.text().contains("pending"));
    mark(harness, Surface::PendingChangeBar)
}

fn structure_view(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    let _ = harness.ctrl('d');
    mark(harness, Surface::StructureView)
}

fn query_editing(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Tab);
    let _ = harness.key(KeyCode::Char('i'));
    let _ = harness.type_str("SELECT * FROM orders");
    mark(harness, Surface::QueryEditing)
}

fn completion_popup(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Tab);
    let _ = harness.key(KeyCode::Char('i'));
    let _ = harness.type_str("SELECT * FROM ord");
    mark(harness, Surface::CompletionPopup)
}

fn results_grid(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Tab);
    let _ = harness.key(KeyCode::Char('i'));
    let _ = harness.type_str("SELECT * FROM orders LIMIT 25");
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.ctrl('r');
    harness.ticks(10);
    mark(harness, Surface::ResultsGrid)
}

fn error_result(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Tab);
    let _ = harness.key(KeyCode::Char('i'));
    let _ = harness.type_str("SELECT nope FROM orders");
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.ctrl('r');
    harness.ticks(10);
    mark(harness, Surface::ErrorResult)
}

fn explain_plan(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Tab);
    let _ = harness.key(KeyCode::Char('i'));
    let _ = harness.type_str("SELECT * FROM orders LIMIT 10");
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.alt('x');
    harness.ticks(10);
    mark(harness, Surface::ExplainPlan)
}

fn history_tab(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.ctrl('y');
    mark(harness, Surface::HistoryTab)
}

fn quick_switcher(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.ctrl('o');
    let _ = harness.type_str("cust");
    mark(harness, Surface::QuickSwitcher)
}

fn tab_list_picker(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    let _ = harness.ctrl('t');
    let _ = harness.ctrl('g');
    mark(harness, Surface::TabListPicker)
}

fn safe_mode_picker(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.ctrl('l');
    mark(harness, Surface::SafeModePicker)
}

fn filter_editor(w: u16, h: u16) -> H {
    let mut harness = table_tab(w, h);
    for _ in 0..4 {
        let _ = harness.key(KeyCode::Right);
    }
    let _ = harness.key(KeyCode::Char('f'));
    mark(harness, Surface::FilterEditor)
}

fn safety_dialog_typed_ack(w: u16, h: u16) -> H {
    let mut harness = connected(w, h);
    let _ = harness.key(KeyCode::Tab);
    let _ = harness.key(KeyCode::Char('i'));
    let _ = harness.type_str("DELETE FROM orders");
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.key(KeyCode::Esc);
    let _ = harness.ctrl('r');
    let _ = harness.key(KeyCode::Enter);
    let _ = harness.type_str("orders");
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
            let expected = BASELINE.before_image_digest(surface.label(), width, height);
            assert!(
                expected.is_some(),
                "missing or malformed frozen before-image row for {width}x{height} {}",
                surface.label()
            );
            let expected = expected.unwrap_or_default();
            let (first, second) = scene_pair(builder, surface, width, height);
            assert_eq!(
                first.before_image_digest(),
                expected,
                "{width}x{height} {}: rendered digest differs from frozen before-image",
                surface.label()
            );
            assert_eq!(
                second.before_image_digest(),
                expected,
                "{width}x{height} {}: repeated render differs from frozen before-image",
                surface.label()
            );
            assert_eq!(
                first.digest(),
                second.digest(),
                "{width}x{height} {}: two builds of the same surface differ",
                surface.label()
            );
        }
    }
    assert_eq!(rows, 42);
}
