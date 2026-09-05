//! Showcase application-shell benchmarks.
//!
//! These are the seven legacy `showcase` gates, moved with the binary.  They
//! deliberately exercise the public `Harness` and `tui_next::Registry`
//! surfaces so the measurements include the real application/runtime path.
//! Run in release with:
//!
//! ```text
//! CARGO_TARGET_DIR=/private/tmp/tc-target-showcase-luna \
//!   cargo test -p showcase --release --test perf -- --test-threads=1 --nocapture
//! ```

use std::hint::black_box;

use showcase_app::{App, PageId};
use tui_next::{Axis, KeyCode, Position, Theme};
use tui_next_testing::Harness;
use tui_next_testing::perf::{Counting, Stats, bench, iters, lock, measure_once, report_to};

#[global_allocator]
static GLOBAL: Counting = Counting;

const BASELINE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/perf_baseline.txt");

fn app(page: PageId, width: u16, height: u16) -> Harness<App> {
    Harness::new(App::with_page(page), Theme::junie(), width, height)
}

fn report(name: &str, stats: &Stats) {
    report_to(BASELINE, name, stats);
}

fn frame_bench(name: &str, h: &mut Harness<App>, frames: usize) {
    let stats = bench(1, iters(frames), &mut || h.draw());
    let hits = h.runtime().registry().len();
    let ring = h.ring().reachable().count();
    report(name, &stats.with_regions(hits, ring));
}

// ------------------------------------------------------------ A. frames

#[test]
fn frame_showcase_lists_120x40() {
    let _guard = lock();
    let mut h = app(PageId::Lists, 120, 40);
    frame_bench("frame_showcase_lists_120x40", &mut h, 200);
}

#[test]
fn frame_showcase_lists_80x24() {
    let _guard = lock();
    let mut h = app(PageId::Lists, 80, 24);
    frame_bench("frame_showcase_lists_80x24", &mut h, 200);
}

/// Lists page with the global help dialog open.  The page remains rendered
/// below the modal layer, preserving the legacy shadowed-page workload.
#[test]
fn frame_showcase_dialog_open() {
    let _guard = lock();
    let mut h = app(PageId::Lists, 120, 40);
    let _ = h.key(KeyCode::Char('?'));
    assert!(
        h.text().contains("Showcase help"),
        "help dialog opens with ?"
    );
    frame_bench("frame_showcase_dialog_open", &mut h, 200);
}

/// Identical frames must allocate identically: no lazily growing render cache.
#[test]
fn render_twice_allocates_the_same() {
    let _guard = lock();
    let mut h = app(PageId::Lists, 120, 40);
    h.draw();
    let first = measure_once(&mut || h.draw());
    let second = measure_once(&mut || h.draw());
    report("render_twice_allocates_the_same", &second);
    assert_eq!(
        first.allocs, second.allocs,
        "identical frames must have identical allocation counts"
    );
    assert_eq!(
        first.bytes, second.bytes,
        "identical frames must request identical bytes"
    );
}

// ------------------------------------------------------------ B. events

/// 1,000 Down keys into the focused language list after the legacy Tab setup.
#[test]
fn key_showcase_down_lists() {
    let _guard = lock();
    let mut h = app(PageId::Lists, 120, 40);
    let _ = h.key(KeyCode::Tab);
    assert!(h.focus().is_some(), "Tab focuses the first list control");
    h = h.with_auto_draw(false);
    let stats = bench(10, iters(1_000), &mut || {
        let _ = black_box(h.key(KeyCode::Down));
    });
    report("key_showcase_down_lists", &stats);
    h.draw();
    assert!(h.text().contains("Lists"), "list page remains rendered");
}

/// A raster of pointer positions over the real Lists-page registry.
#[test]
fn mouse_move_showcase_frame() {
    let _guard = lock();
    let h = app(PageId::Lists, 120, 40);
    let stats = bench(1, iters(10), &mut || {
        for y in 0..40_u16 {
            for x in (0..120_u16).step_by(2) {
                black_box(h.runtime().registry().hit(Position::new(x, y)));
            }
        }
    });
    let hits = h.runtime().registry().len();
    let ring = h.ring().reachable().count();
    report("mouse_move_showcase_frame", &stats.with_regions(hits, ring));
    assert_eq!(stats.allocs, 0, "pointer hit testing must not allocate");
}

/// 1,000 wheel events over the language list, alternating direction.
#[test]
fn wheel_showcase_lists() {
    let _guard = lock();
    let mut h = app(PageId::Lists, 120, 40);
    let position = h.find("Rust");
    assert!(position.is_some(), "language list starts in the frame");
    let (x, y) = position.unwrap_or((0, 0));
    h = h.with_auto_draw(false);
    let mut down = true;
    let stats = bench(10, iters(1_000), &mut || {
        down = !down;
        let delta = if down { 2 } else { -2 };
        let _ = black_box(h.wheel(Axis::V, delta, x, y));
    });
    report("wheel_showcase_lists", &stats);
    assert_eq!(stats.allocs, 0, "wheel routing must not allocate");
    h.draw();
}
