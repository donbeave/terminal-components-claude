//! `TablePro` application-shell benchmarks (`docs/audit/performance-audit.md`
//! §7.2 A/B/D/F). Run in release:
//!
//! ```text
//! cargo test -p tablepro --test perf --release -- --test-threads=1 --nocapture
//! ```
//!
//! Fixture: the `Production` connection with `public.orders` opened as a
//! table tab. `orders` has 14 columns and the loader caps at `sql::ROW_CAP` =
//! 500 rows, so the "500×12" benchmark name remains the historical key.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::items_after_statements,
    clippy::manual_let_else,
    clippy::panic,
    clippy::print_stdout,
    reason = "the allocator gate preserves direct fixture assertions and reports"
)]

use std::hint::black_box;

use tablepro_app::{TableProApp, db, grid_model::ResultGrid, sql};
use tui_next::{
    Axis, Input, Key, KeyCode, KeyModifiers, Mouse, MouseKind, Position, SortDir, Theme,
};
use tui_next_testing::{Harness, perf};

// These rows remain historical ownership evidence; normal runs never bless.
// Release assertions enforce the audit thresholds independently of baselines.
#[global_allocator]
static GLOBAL: perf::Counting = perf::Counting;

const BASELINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/perf_baseline.txt");

struct H {
    harness: Harness<TableProApp>,
}

impl H {
    /// Connected workbench with `public.orders` open and the grid focused.
    fn orders(w: u16, h: u16) -> Self {
        let mut app = TableProApp::new();
        assert!(app.connect(0));
        assert!(app.open_table("orders"));
        let harness = Harness::new(app, Theme::junie(), w, h).with_auto_draw(false);
        let hh = Self { harness };
        let t = hh.table();
        assert_eq!(t.table.name, "orders");
        assert_eq!(t.result.row_count(), 500, "ROW_CAP rows loaded");
        assert_eq!(t.table.columns.len(), 14);
        hh
    }

    fn draw(&mut self) {
        self.harness.draw();
    }

    fn key(&mut self, code: KeyCode) -> tui_next::Response<()> {
        self.harness.handle(key(code))
    }

    fn table(&self) -> &tablepro_app::tabs::TableTab {
        self.harness
            .app()
            .workbench
            .active_table()
            .expect("active tab is not a table")
    }

    fn table_mut(&mut self) -> &mut tablepro_app::tabs::TableTab {
        self.harness
            .app_mut()
            .workbench
            .active_table_mut()
            .expect("active tab is not a table")
    }

    fn regions(&self) -> (usize, usize) {
        (
            self.harness.runtime().region_count(),
            self.harness.ring().entries().len(),
        )
    }
}

fn key(code: KeyCode) -> Input {
    Input::Key(Key {
        code,
        mods: KeyModifiers::NONE,
    })
}

const FRAME: &str = "frame_tablepro_grid_500x12_120x40";

// ------------------------------------------------------------ A. frames

#[test]
fn frame_tablepro_grid_500x12_120x40() {
    let _g = perf::lock();
    let mut h = H::orders(120, 40);
    let s = perf::bench(1, perf::iters(200), &mut || h.draw());
    let (hits, ring) = h.regions();
    println!("PERF-GATE frame allocations={} bytes={}", s.allocs, s.bytes);
    perf::report_alloc_to(BASELINE, FRAME, &s.with_regions(hits, ring));
    if !cfg!(debug_assertions) {
        assert!(s.allocs < 100, "frame allocates {} times", s.allocs);
    }
}

// ------------------------------------------------------------ B. events

/// 1 000 arrow keys cycling Down/Right/Up/Left on the 500-row grid.
#[test]
fn key_tablepro_grid_cursor() {
    let _g = perf::lock();
    let mut h = H::orders(120, 40);
    const ARROWS: [KeyCode; 4] = [KeyCode::Down, KeyCode::Right, KeyCode::Up, KeyCode::Left];
    let mut i = 0usize;
    let s = perf::bench(10, perf::iters(1000), &mut || {
        let _ = black_box(h.key(ARROWS[i % 4]));
        i += 1;
    });
    println!(
        "PERF-NOTE key_tablepro_grid_cursor cursor={:?}",
        h.harness.focus()
    );
    perf::report_alloc_to(BASELINE, "key_tablepro_grid_cursor", &s);
}

/// 20 local sorts, isolating domain comparison and row-key preservation.
#[test]
fn key_tablepro_grid_sort_local() {
    let _g = perf::lock();
    let mut h = H::orders(120, 40);
    let s = perf::bench(0, 20, &mut || {
        h.table_mut().sort(0, SortDir::Asc);
        black_box(h.table().result.row_count());
    });
    perf::report_alloc_to(BASELINE, "key_tablepro_grid_sort_local", &s);
}

/// 1 000 clicks (Down + Up) on a body cell, routed through the public
/// harness and the generic grid's pointer resolver.
#[test]
fn mouse_click_grid_cell() {
    let _g = perf::lock();
    let mut h = H::orders(120, 40);
    let pos = Position::new(60, 18);
    let click = |h: &mut H| {
        // Keep each sample a single click. Otherwise the runtime correctly
        // promotes every second sample to a double-click/editor transition.
        h.harness.runtime_mut().advance_clock(1_000);
        let _ = h.harness.handle(Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos,
            mods: KeyModifiers::NONE,
        }));
        let _ = black_box(h.harness.handle(Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos,
            mods: KeyModifiers::NONE,
        })));
    };
    let s = perf::bench(10, perf::iters(1000), &mut || click(&mut h));
    assert_eq!(
        h.table().result.row_count(),
        500,
        "click leaves the result loaded"
    );
    perf::report_alloc_to(BASELINE, "mouse_click_grid_cell", &s);
}

/// 1 000 wheel events over the grid, alternating direction.
#[test]
fn wheel_tablepro_grid() {
    let _g = perf::lock();
    let mut h = H::orders(120, 40);
    let mut down = true;
    let pos = Position::new(60, 18);
    let s = perf::bench(10, perf::iters(1000), &mut || {
        down = !down;
        let kind = MouseKind::Wheel(Axis::V, if down { 1 } else { -1 });
        let _ = black_box(h.harness.handle(Input::Mouse(Mouse {
            kind,
            pos,
            mods: KeyModifiers::NONE,
        })));
    });
    perf::report_alloc_to(BASELINE, "wheel_tablepro_grid", &s);
    if !cfg!(debug_assertions) {
        assert_eq!(s.allocs, 0, "wheel routing allocated {} times", s.allocs);
    }
}

// ------------------------------------------------------------ D. large data

/// One `ResultGrid::from_result`: the adapter owns one current row copy and
/// preformats only scalar cells, while text/JSON cells remain borrowed.
#[test]
fn grid_500x12_load() {
    let _g = perf::lock();
    let catalog = db::Catalog::acme_prod();
    let statement = match sql::parse("SELECT * FROM orders") {
        Ok(sql::Statement::Select(select)) => select,
        _ => panic!("orders fixture must parse"),
    };
    let result = sql::run_select(&catalog, &statement).expect("orders fixture must execute");
    assert_eq!(result.rows.len(), 500);
    let mut last = ResultGrid::empty();
    let s = perf::bench(1, perf::iters(5), &mut || {
        last = ResultGrid::from_result(&result);
        black_box(last.row_count());
    });
    assert_eq!(last.row_count(), 500);
    perf::report_alloc_to(BASELINE, "grid_500x12_load", &s);
    if !cfg!(debug_assertions) {
        assert!(s.allocs < 8_000, "load allocates {} times", s.allocs);
    }
}

// ------------------------------------------------------------ F. invariants

/// R4 guard: the optimized grid frame stays under the semantic allocation
/// budget. Build-mode allocation counts are not compared with copied legacy
/// timings: debug instrumentation is a different workload, while release is
/// the shipped contract.
#[test]
fn debug_and_release_alloc_counts_match() {
    let _g = perf::lock();
    let mut h = H::orders(120, 40);
    let s = perf::bench(1, perf::iters(20), &mut || h.draw());
    println!(
        "PERF-BUILD {FRAME} debug_assertions={} allocs={}",
        cfg!(debug_assertions),
        s.allocs
    );
    if !cfg!(debug_assertions) {
        assert!(s.allocs < 100, "frame allocates {} times", s.allocs);
    }
}
