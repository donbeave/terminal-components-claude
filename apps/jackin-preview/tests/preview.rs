//! Supplemental public integration checks for the concrete Jackin shell.

#![allow(
    dead_code,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    clippy::explicit_iter_loop,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used
)]

mod support;

use ratatui::crossterm::event::KeyCode;

use jackin_app::domain::fixtures::world_for;
use jackin_app::{Motion, Route, Scenario};
use support::H;

#[test]
fn first_use_flow_enters_the_manager() {
    let mut h = H::new(Scenario::FirstUse, Motion::Reduced, 0, 100, 30);
    assert_eq!(h.app.route, Route::Intro);
    assert!(h.text().contains("Enter Continue"));
    h.ticks(3);
    h.key(KeyCode::Enter);
    assert_eq!(h.app.route, Route::Manager);
    assert!(h.text().contains("Current directory"));
}

#[test]
fn product_routes_render_through_the_concrete_shell() {
    let mut h = H::new(Scenario::Returning, Motion::Reduced, 0, 100, 30);
    for (key, route, label) in [
        (KeyCode::Char('c'), Route::Accounts, "Overview"),
        (KeyCode::Char('u'), Route::Usage, "Usage"),
        (KeyCode::Char('c'), Route::Accounts, "Overview"),
        (KeyCode::Char('s'), Route::Settings, "Settings"),
    ] {
        h.key(key);
        assert_eq!(h.app.route, route);
        assert!(h.text().contains(label), "{}", h.text());
    }
    let capsule = H::new(Scenario::CapsuleMulti, Motion::Paused, 0, 100, 30);
    assert_eq!(capsule.app.route, Route::Capsule);
    assert!(capsule.text().contains("Shell"));
}

#[test]
fn launch_dialog_contains_role_and_account_choices() {
    let mut h = H::new(Scenario::Returning, Motion::Reduced, 0, 120, 40);
    h.key(KeyCode::Down);
    h.key(KeyCode::Enter);
    assert!(h.text().contains("Launch · choose Agent"), "{}", h.text());
    h.key(KeyCode::Esc);
    assert_eq!(h.app.route, Route::Manager);
}

#[test]
fn launch_simulation_is_deterministic_and_reaches_capsule() {
    let mut h = H::new(Scenario::LaunchRunning, Motion::Reduced, 0, 120, 40);
    assert_eq!(h.app.route, Route::Cockpit);
    for _ in 0..100 {
        h.ticks(10);
        if h.app.route == Route::Capsule {
            break;
        }
    }
    assert_eq!(h.app.route, Route::Capsule);
    assert!(h.text().contains("jackin❯"));
}

#[test]
fn resize_preserves_the_shell_and_reports_small_terminals() {
    let mut h = H::new(Scenario::Returning, Motion::Reduced, 0, 100, 30);
    h.resize(60, 18);
    assert!(h.text().contains("Terminal too small"));
    h.resize(84, 24);
    assert!(h.text().contains("Workspaces"));
}

#[test]
fn scenarios_and_references_are_stable_without_secret_material() {
    for scenario in Scenario::ALL {
        let first = world_for(scenario);
        let second = world_for(scenario);
        assert_eq!(first.scenario, second.scenario);
        assert_eq!(first.cwd, second.cwd);
        assert_eq!(first.workspaces, second.workspaces);
        assert_eq!(first.instances, second.instances);
        assert_eq!(first.accounts, second.accounts);
    }
    let debug = format!("{:?}", world_for(Scenario::AccountsMixed).accounts);
    assert!(debug.contains("v_eng01"));
    assert!(debug.contains("it_cdx01"));
    assert!(!debug.contains("valid-ant01"));
    assert!(!debug.contains("valid-cdx01"));
}

#[test]
fn pinned_frames_keep_each_ritual_route_reachable() {
    assert_eq!(
        H::new(Scenario::FirstUse, Motion::Paused, 0, 120, 40)
            .app
            .route,
        Route::Intro
    );
    assert_eq!(
        H::new(
            Scenario::FirstUse,
            Motion::Full,
            jackin_app::rain::INTRO_END + 1,
            120,
            40,
        )
        .app
        .route,
        Route::Manager
    );
    assert_eq!(
        H::new(Scenario::LaunchRunning, Motion::Paused, 0, 120, 40)
            .app
            .route,
        Route::Cockpit
    );
    assert_eq!(
        H::new(Scenario::CapsuleMulti, Motion::Paused, 0, 120, 40)
            .app
            .route,
        Route::Capsule
    );
    assert_eq!(
        H::new(Scenario::OutroLast, Motion::Paused, 1, 120, 40)
            .app
            .route,
        Route::Outro
    );
}

#[test]
fn paused_frames_freeze_the_virtual_clock() {
    let mut h = H::new(Scenario::Returning, Motion::Paused, 12, 100, 30);
    let before = h.app.world.now_ms();
    h.ticks(20);
    assert_eq!(h.app.world.now_ms(), before);
}

#[test]
fn container_uid_is_total_for_every_fixture_scenario() {
    for scenario in Scenario::ALL {
        let world = world_for(scenario);
        for instance in world.instances {
            assert!(instance.container_id().len() > instance.container.len());
            assert!(instance.container_id().starts_with("jackin-"));
        }
    }
}

#[test]
fn every_named_scenario_renders_a_deterministic_frame() {
    for scenario in Scenario::ALL {
        let first = H::new(scenario, Motion::Paused, 0, 120, 40);
        let second = H::new(scenario, Motion::Paused, 0, 120, 40);
        assert_eq!(first.text(), second.text(), "{}", scenario.name());
    }
}
