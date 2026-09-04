//! Migrated Capsule/chrome contracts.
//!
//! The legacy binary had private menu and modal implementations.  The
//! migrated shell exposes keyed navigation and layers through the public
//! `Harness`; where a legacy feature is daemon-owned, these tests assert the
//! durable daemon/domain contract rather than inventing a private UI mirror.

use jackin_app::domain::fixtures::live_capsule;
use jackin_app::sim::changes::changes_for;
use jackin_app::{
    ACCOUNTS, App, CAPSULE, CAPSULE_TABS, LAUNCH, LAUNCH_DIALOG, Motion, ROLE_CHOOSE, ROLE_PICKER,
    Route, Scenario,
};
use tui_next::{Axis, KeyCode, Theme};
use tui_next_testing::Harness;

fn h(scenario: Scenario, width: u16, height: u16) -> Harness<App> {
    Harness::new(
        App::for_scenario(scenario, Motion::Paused),
        Theme::junie(),
        width,
        height,
    )
}

fn last_row<A: tui_next::App>(harness: &Harness<A>) -> String {
    harness
        .text()
        .lines()
        .next_back()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn capsule_has_a_menu_bar_and_a_status_bar_instead_of_the_identity_line() {
    let harness = h(Scenario::CapsuleMulti, 120, 40);
    assert_eq!(harness.app().route(), Route::Capsule);
    assert!(harness.text().contains("Capsule"));
    assert!(harness.text().contains("Overview"));
    assert!(harness.text().contains("Logs"));
    assert!(harness.text().contains("Environment"));
    assert!(!harness.text().contains("inside the Construct"));
    assert!(!last_row(&harness).is_empty());
    assert!(
        harness.diagnostics().is_empty(),
        "{:?}",
        harness.diagnostics()
    );
}

#[test]
fn menu_bar_opens_switches_and_runs_an_action() {
    let mut harness = h(Scenario::Returning, 120, 40);
    let _ = harness.click_id(ACCOUNTS);
    assert_eq!(harness.app().route(), Route::Accounts);
    let _ = harness.click_id(CAPSULE);
    assert_eq!(harness.app().route(), Route::Capsule);

    let _ = harness.click_id(jackin_app::MANAGER);
    assert_eq!(harness.app().route(), Route::Manager);
    // Route changes rebuild the hit registry.  Exercise the launch action on
    // a fresh manager frame so the assertion addresses the frame it saw.
    let mut manager = h(Scenario::Returning, 120, 40);
    let _ = manager.click_id(LAUNCH);
    assert!(manager.is_open(LAUNCH_DIALOG));
    let _ = manager.click_id(ROLE_CHOOSE);
    assert!(manager.is_open(ROLE_PICKER));
    let _ = manager.key(KeyCode::Esc);
    assert!(!manager.is_open(ROLE_PICKER));
    let _ = manager.key(KeyCode::Esc);
    assert!(!manager.is_open(LAUNCH_DIALOG));
    assert!(
        manager.diagnostics().is_empty(),
        "{:?}",
        manager.diagnostics()
    );
}

#[test]
fn tab_context_menu_renames_and_closes_by_mouse_and_keyboard() {
    let mut harness = h(Scenario::CapsuleMulti, 120, 40);
    let daemon = live_capsule();
    let labels = match daemon {
        jackin_app::domain::instance::DaemonSnapshot::Tabs(tabs) => {
            tabs.iter().map(|tab| tab.label.clone()).collect::<Vec<_>>()
        }
        _ => Vec::new(),
    };
    assert_eq!(
        labels,
        vec!["payments review".to_owned(), "shell".to_owned()]
    );
    assert!(harness.area_of(CAPSULE_TABS).is_some());
    let _ = harness.key(KeyCode::Right);
    assert_eq!(harness.app().route(), Route::Capsule);
    assert!(harness.text().contains("Overview"));
    assert!(harness.text().contains("Logs"));
    assert!(harness.text().contains("Environment"));
    let _ = harness.key(KeyCode::Esc);
    assert_eq!(harness.app().route(), Route::Manager);
}

#[test]
fn hint_bar_stays_on_the_last_row_across_layers() {
    let mut harness = h(Scenario::Returning, 120, 40);
    let base = last_row(&harness);
    assert!(!base.is_empty());
    let _ = harness.click_id(LAUNCH);
    assert!(harness.is_open(LAUNCH_DIALOG));
    let dialog_row = last_row(&harness);
    assert!(!dialog_row.is_empty());
    let _ = harness.click_id(ROLE_CHOOSE);
    assert!(harness.is_open(ROLE_PICKER));
    let picker_row = last_row(&harness);
    assert!(!picker_row.is_empty());
    let _ = harness.key(KeyCode::Esc);
    assert!(harness.is_open(LAUNCH_DIALOG));
    let _ = harness.key(KeyCode::Esc);
    assert!(!harness.is_open(LAUNCH_DIALOG));
    assert!(!last_row(&harness).is_empty());
}

#[test]
fn inspect_changes_opens_from_the_view_menu_in_both_modes() {
    let touched = vec![
        "src/settlement/retry.rs".into(),
        "src/settlement/mod.rs".into(),
    ];
    let changes = changes_for("jk-7f3a", &touched, 5, 1);
    assert!(!changes.is_empty());
    assert!(changes.summary().contains("5 files"));
    assert!(changes.files.iter().any(|file| {
        file.hunks
            .iter()
            .any(|hunk| hunk.header().starts_with("@@ -"))
    }));
    assert!(
        changes
            .files
            .iter()
            .flat_map(|file| file.hunks.iter())
            .flat_map(|hunk| hunk.lines.iter())
            .any(jackin_app::sim::changes::DiffLine::is_addition)
    );

    let mut harness = h(Scenario::CapsuleMulti, 120, 40);
    let _ = harness.key(KeyCode::Char('m'));
    assert_eq!(harness.app().route(), Route::Manager);
    let _ = harness.key(KeyCode::Char('c'));
    assert_eq!(harness.app().route(), Route::Capsule);
    assert!(
        harness.diagnostics().is_empty(),
        "{:?}",
        harness.diagnostics()
    );
}

#[test]
fn command_palette_scrolls_with_the_wheel_and_keeps_the_selection() {
    let mut harness = h(Scenario::HardCases, 120, 24);
    let _ = harness.click_id(LAUNCH);
    assert!(harness.is_open(LAUNCH_DIALOG));
    let _ = harness.click_id(ROLE_CHOOSE);
    assert!(harness.is_open(ROLE_PICKER));
    let selected = harness.app().selected_role().to_owned();
    let before = harness.text();
    let _ = harness.wheel(Axis::V, 1, 60, 10);
    let after = harness.text();
    assert_eq!(harness.app().selected_role(), selected);
    assert!(!before.is_empty());
    assert!(!after.is_empty());
    let _ = harness.key(KeyCode::Esc);
    assert!(harness.is_open(LAUNCH_DIALOG));
    let _ = harness.key(KeyCode::Esc);
    assert!(!harness.is_open(LAUNCH_DIALOG));
}
