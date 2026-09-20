//! Expand remaining SC rows: default frames, Diff absence, executable keys.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "integration assertions"
)]

use oracle_showcase::{
    Action, ColorSpec, OraclePageId, ShowcaseDriver, TerminalSize, capture_case,
    default_page_frames, expand, rows,
};

#[test]
fn every_sc_base_default_frame_is_observed() {
    for case in default_page_frames().unwrap() {
        let frame = capture_case(case.page, case.size, case.color, &case.program).unwrap();
        assert_eq!(frame.oracle_page, case.page);
        assert_eq!(frame.size, case.size);
        assert_eq!(frame.color, case.color);
        if case.page == OraclePageId::Diff {
            assert!(
                frame.production_route_absent,
                "Diff is oracle-only; do not fabricate a production frame"
            );
            assert!(frame.cells.is_empty());
            continue;
        }
        assert!(!frame.production_route_absent);
        assert_eq!(frame.production_page, Some(case.page));
        assert!(frame.is_complete());
    }
}

#[test]
fn sc_shell_nav_keys_replay_without_find() {
    let row = rows()
        .unwrap()
        .into_iter()
        .find(|row| row.scenario_id == "SC-SHELL-NAV")
        .unwrap();
    let mut driver = ShowcaseDriver::open(
        OraclePageId::Overview,
        TerminalSize::new(80, 24),
        ColorSpec::TrueColor,
    )
    .unwrap();
    for step in &row.program.steps {
        match step {
            Action::Key(key) => driver.key(*key).unwrap(),
            Action::FreshDraw => driver.present().unwrap(),
            Action::Tick => driver.tick().unwrap(),
            // Labeled hits stay fail-closed; keys already prove routing
            // without buffer text search.
            Action::Unfrozen { .. }
            | Action::Ticks(_)
            | Action::Time(_)
            | Action::TimeThenTick(_)
            | Action::Resize(_)
            | Action::Pointer { .. } => {}
        }
    }
    let frame = driver.observe().unwrap();
    assert!(!frame.quit, "Esc in SC-SHELL-NAV must never quit");
    assert!(frame.is_complete());
}

#[test]
fn expansion_retains_every_tsv_row() {
    let rows = rows().unwrap();
    let cases = expand().unwrap();
    for row in &rows {
        assert!(
            cases.iter().any(|case| case.scenario_id == row.scenario_id),
            "missing expansion for {}",
            row.scenario_id
        );
    }
}

#[test]
fn production_cli_page_parser_is_observed_not_repaired() {
    assert!(showcase_app::PageId::from_name("overview").is_some());
    assert!(showcase_app::PageId::from_name("diff").is_none());
    assert!(showcase_app::PageId::from_name("codeeditor").is_some());
    assert!(showcase_app::PageId::from_name("datagrid").is_some());
    assert!(showcase_app::PageId::from_name("chipsselects").is_some());
    assert!(showcase_app::PageId::from_name("editabletables").is_some());
    for page in OraclePageId::ALL {
        match page {
            OraclePageId::Diff => assert!(page.production().is_none()),
            _ => assert!(
                page.production().is_some(),
                "production missing {}",
                page.slug()
            ),
        }
    }
}
