//! Jackin application-shell benchmarks (`docs/audit/performance-audit.md`
//! §7.2 A/B). Run in release:
//!
//! ```text
//! cargo test --release --bin jackin-preview perf_tests -- --test-threads=1 --nocapture
//! ```
//!
//! Fixtures: the `hard-cases` scenario for the manager (every Workspace
//! expanded so all instance rows are visible) and `capsule-multi` for the
//! Capsule, with the active tab split to four panes and every pane's
//! scrollback filled to `SCROLLBACK` (2 000) lines.

#![allow(
    unsafe_code,
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
    clippy::print_stdout,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::undocumented_unsafe_blocks
)]

#[path = "../../../tests/perf_common.rs"]
#[allow(
    clippy::format_push_string,
    clippy::manual_assert,
    clippy::print_stdout,
    clippy::redundant_closure_for_method_calls
)]
mod perf_common;

use std::hint::black_box;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use jackin_app::core::event::{Input, Key};
use jackin_app::theme::{Theme, Tone};
use jackin_app::ui::layout::SplitDir;
use jackin_app::widgets::viewport::Span;

use jackin_app::sim::pty::SCROLLBACK;
use jackin_app::{App, Route};
use jackin_app::{Motion, Scenario};
use perf_common::{Counting, bench, iters, lock, report};

#[global_allocator]
static GLOBAL: Counting = Counting;

struct H {
    app: App,
    term: Terminal<TestBackend>,
}

impl H {
    fn new(scenario: Scenario, motion: Motion, w: u16, h: u16) -> Self {
        let app = App::for_scenario_with_theme(scenario, motion, 0, Theme::junie());
        let term = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut hh = Self { app, term };
        hh.draw();
        hh
    }

    /// `hard-cases` manager with every Workspace expanded.
    fn manager(w: u16, h: u16) -> Self {
        let mut hh = Self::new(Scenario::HardCases, Motion::Reduced, w, h);
        for _ in 0..8 {
            hh.ticks(3);
            if hh.app.route == Route::Manager {
                break;
            }
            hh.key(KeyCode::Enter);
        }
        assert_eq!(hh.app.route, Route::Manager);
        let ids: Vec<_> = hh.app.world.workspaces.iter().map(|ws| ws.id).collect();
        for id in ids {
            hh.app.screens.manager.expanded.insert(id);
        }
        // Reconcile the fixture's expansion set through the real manager key
        // path; rendering itself is allocation-free with stable rows.
        hh.key(KeyCode::Char('*'));
        hh
    }

    /// `capsule-multi` Capsule with four panes of 2 000 scrollback lines.
    fn capsule(w: u16, h: u16) -> Self {
        let mut hh = Self::new(Scenario::CapsuleMulti, Motion::Full, w, h);
        assert_eq!(hh.app.route, Route::Capsule);
        let instance = hh.app.screens.capsule.as_ref().unwrap().instance.clone();
        let now = hh.app.world.now_ms();
        let d = hh.app.world.daemons.get_mut(&instance).expect("daemon");
        if d.active_tab().map(|t| t.leaves().len()) == Some(3) {
            d.split(SplitDir::Vertical, false, None, None, now, false)
                .expect("split the focused pane");
        }
        let leaves = d.active_tab().unwrap().leaves();
        assert_eq!(leaves.len(), 4, "four panes on the active tab");
        for id in leaves {
            let pane = d.pane_mut(id).unwrap();
            let start = pane.term.len();
            assert!(start <= SCROLLBACK, "fixture pane exceeds scrollback cap");
            for i in start..SCROLLBACK {
                pane.term.push(vec![
                    Span::new(format!("[{i:05}] "), Tone::Secondary),
                    Span::plain(format!(
                        "lorem ipsum dolor sit amet, consectetur adipiscing elit {i}"
                    )),
                ]);
            }
            assert_eq!(pane.term.len(), SCROLLBACK);
        }
        hh.draw();
        hh
    }

    fn draw(&mut self) {
        self.term.draw(|f| self.app.render(f)).unwrap();
    }

    fn key(&mut self, code: KeyCode) {
        self.app.handle(key(code));
        self.draw();
    }

    fn ticks(&mut self, n: usize) {
        for _ in 0..n {
            self.app.handle(Input::Tick);
        }
        self.draw();
    }

    fn regions(&self) -> (usize, usize) {
        (self.app.hits.len(), self.app.ring.reachable().len())
    }
}

fn key(code: KeyCode) -> Input {
    Input::Key(Key {
        code,
        mods: KeyModifiers::NONE,
    })
}

/// The application baseline carries the three live Slice 7 measurements.
/// The old per-frame viewport clone is intentionally absent: its deletion is
/// an acceptance condition, not another benchmark row.
#[test]
fn jackin_perf_baseline_has_required_rows_and_no_deleted_clone() {
    const ROWS: [&str; 3] = [
        "frame_jackin_capsule_4panes_120x40",
        "frame_jackin_manager_100rows_120x40",
        "key_jackin_manager_move",
    ];
    let names: Vec<_> = include_str!("perf_baseline.txt")
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| !name.starts_with('#'))
        .collect();
    assert_eq!(names, ROWS);
    assert!(!names.contains(&"capsule_pane_clone_4x2000"));
}

// ------------------------------------------------------------ A. frames

/// Manager render with every Workspace expanded: exercises the cached row
/// projection, detail projection and action controls per frame.
#[test]
fn frame_jackin_manager_100rows_120x40() {
    let _g = lock();
    let mut h = H::manager(120, 40);
    println!(
        "PERF-NOTE frame_jackin_manager_100rows_120x40 workspaces={} instances={}",
        h.app.world.workspaces.len(),
        h.app.world.instances.len()
    );
    let s = bench(1, iters(200), &mut || h.draw());
    let (hits, ring) = h.regions();
    report(
        "frame_jackin_manager_100rows_120x40",
        &s.with_regions(hits, ring),
    );
    if perf_common::env_flag("PERF_STRICT") {
        assert!(s.allocs < 60, "manager frame allocates {} times", s.allocs);
    }
}

/// Four panes × 2 000 scrollback lines: renders each daemon viewport in place
/// without cloning its scrollback (`capsule.rs`).
#[test]
fn frame_jackin_capsule_4panes_120x40() {
    let _g = lock();
    let mut h = H::capsule(120, 40);
    let s = bench(1, iters(10), &mut || h.draw());
    let (hits, ring) = h.regions();
    report(
        "frame_jackin_capsule_4panes_120x40",
        &s.with_regions(hits, ring),
    );
    if perf_common::env_flag("PERF_STRICT") {
        assert!(s.allocs < 200, "capsule frame allocates {} times", s.allocs);
    }
}

// ------------------------------------------------------------ B. events

/// 1 000 arrow keys (Down/Up alternating) in the manager tree; the stable
/// projection must not rebuild or allocate per key.
#[test]
fn key_jackin_manager_move() {
    let _g = lock();
    let mut h = H::manager(120, 40);
    let mut down = true;
    let s = bench(10, iters(1000), &mut || {
        let code = if down { KeyCode::Down } else { KeyCode::Up };
        down = !down;
        black_box(h.app.handle(key(code)));
    });
    report("key_jackin_manager_move", &s);
    if perf_common::env_flag("PERF_STRICT") {
        assert_eq!(s.allocs, 0, "manager key path allocates {} times", s.allocs);
    }
}
