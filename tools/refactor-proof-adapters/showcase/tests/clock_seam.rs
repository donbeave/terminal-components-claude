//! Instant clock seam: ticks do not advance time; Instant maps onto Moment.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration assertions"
)]

use std::time::Duration;

use junie_tui::Moment;
use oracle_showcase::source_ids::{BUTTONS, LONG_JOB};
use oracle_showcase::{
    ColorSpec, InstantClock, OraclePageId, ShowcaseDriver, TerminalSize, deadlines,
};

#[test]
fn instant_origin_is_instant_and_ticks_do_not_advance_it() {
    let clock = InstantClock::new();
    assert_eq!(clock.logical_now(), clock.origin());
    assert_eq!(clock.elapsed(), Duration::ZERO);
    assert_eq!(clock.moment(), Moment::ZERO);

    let mut driver = ShowcaseDriver::open(
        OraclePageId::Buttons,
        TerminalSize::new(120, 40),
        ColorSpec::TrueColor,
    )
    .unwrap();
    assert_eq!(driver.clock().elapsed(), Duration::ZERO);
    assert_eq!(driver.runtime_now(), Moment::ZERO);

    driver.ticks(1_000).unwrap();
    assert_eq!(
        driver.clock().elapsed(),
        Duration::ZERO,
        "Input::Tick must not advance Instant elapsed"
    );
    assert_eq!(driver.runtime_now(), Moment::ZERO);

    driver.set_elapsed(deadlines::BUSY).unwrap();
    assert_eq!(driver.clock().elapsed(), deadlines::BUSY);
    assert_eq!(driver.runtime_now(), Moment::from_millis(2_200));
    driver.tick().unwrap();
    assert_eq!(driver.clock().elapsed(), deadlines::BUSY);
}

#[test]
fn busy_completes_at_2200ms_tick_not_at_2199() {
    let mut driver = ShowcaseDriver::open(
        OraclePageId::Buttons,
        TerminalSize::new(120, 40),
        ColorSpec::TrueColor,
    )
    .unwrap();
    driver.click_id(BUTTONS.index(LONG_JOB)).unwrap();
    let working = driver.observe().unwrap().text();
    assert!(
        working.contains("Working"),
        "source-id click must start the long job\n{working}"
    );

    driver.set_elapsed(Duration::from_millis(2_199)).unwrap();
    driver.tick().unwrap();
    let still = driver.observe().unwrap().text();
    assert!(
        still.contains("Working"),
        "busy stays until Instant elapsed >= 2200ms\n{still}"
    );

    driver.set_elapsed(deadlines::BUSY).unwrap();
    driver.tick().unwrap();
    let done = driver.observe().unwrap().text();
    assert!(
        done.contains("finished") || !done.contains("Working…"),
        "production busy deadline observed at Instant 2200ms + Tick\n{done}"
    );
}

#[test]
fn status_and_flash_boundaries_are_expressible_without_changing_route() {
    let mut driver = ShowcaseDriver::open(
        OraclePageId::Buttons,
        TerminalSize::new(120, 40),
        ColorSpec::TrueColor,
    )
    .unwrap();
    let page = driver.app().page();
    driver.set_elapsed(Duration::from_millis(139)).unwrap();
    assert_eq!(driver.clock().elapsed(), Duration::from_millis(139));
    driver.set_elapsed(deadlines::FLASH).unwrap();
    assert_eq!(driver.clock().elapsed(), deadlines::FLASH);
    driver
        .set_elapsed(deadlines::SUBMIT + Duration::from_nanos(1))
        .unwrap();
    assert_eq!(
        driver.app().page(),
        page,
        "clock-only Instant advance must not change route"
    );
    driver.set_elapsed(deadlines::STATUS).unwrap();
    driver.tick().unwrap();
    driver
        .set_elapsed(deadlines::STATUS + Duration::from_nanos(1))
        .unwrap();
    assert_eq!(driver.app().page(), page);
}

#[test]
fn adapter_source_does_not_use_harness_find() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    for entry in walkdir(&root) {
        let text = std::fs::read_to_string(&entry).unwrap();
        if text.contains("junie_tui_testing") || text.contains("Harness::new") {
            hits.push(entry);
        }
    }
    assert!(
        hits.is_empty(),
        "Instant clock seam forbids harness text-search: {hits:?}"
    );
}

fn walkdir(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap().path();
            if entry.is_dir() {
                stack.push(entry);
            } else if entry.extension().is_some_and(|ext| ext == "rs") {
                files.push(entry);
            }
        }
    }
    files
}
