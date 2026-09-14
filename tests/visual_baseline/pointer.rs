//! Pointer & geometry group: the captures the `tuisnap run` CLI could not
//! express — hover states (`showcase/hover/`), diff drag-select
//! (`showcase/flows/`, including canonical drag-select expansion),
//! the right-click context menu and the inspector-under-scroll rows
//! (§3.7 S9/S12), wheel scrolling with scroll-fade evidence (`<app>/fade/`,
//! including the §3.3 rows, several frame-seeked with `--motion paused
//! --frame N` instead of the multi-minute boot streams), and mid-session
//! resize sequences (`<app>/resize/`) — closing honest gaps 1–2 of
//! `docs/baseline/tuisnap-coverage.md` through the library's
//! click/drag/scroll/resize API. Every state is deterministic (paused motion
//! or content/settle waits on seeded fixtures); each frame goes through the
//! same cell+pixel gate as the ported matrix. Reference evidence for the
//! target states (legacy corpus): `shots/f_buttons_hover.*`,
//! `shots/f_lists_hover.*`, `shots/f_tables_hover.*`,
//! `shots/audit-flows/diff_drag_*`, `shots/fade/*`.

use std::time::Duration;

use tuisnap::pty::{MouseButton, Scroll, Session};

use crate::support::{self, Case, Color, HOLLA, SHOWCASE, TABLEPRO};

const SHOWCASE_BOOT: &str = "Junie Design system";
const HOLLA_BOOT: &str = "holla❯";

/// First occurrence of `needle` as `(row, col)` — `Screen::find` order —
/// waiting until it appears. The hand-off to the pointer calls swaps the
/// axes (they take `(col, row)`).
fn find(s: &mut Session, needle: &str) -> (u16, u16) {
    let mut hit = None;
    s.wait_until(|screen| {
        hit = screen.find(needle);
        hit.is_some()
    })
    .unwrap_or_else(|e| panic!("`{needle}` never appeared: {e:#}"));
    hit.expect("wait_until passed with the needle on screen")
}

/// Bare-pointer move. termlens models clicks, drags and the wheel but has no
/// button-less motion report, so the bytes a real terminal sends for one are
/// written verbatim: every app enables SGR any-motion tracking
/// (`?1003h` + `?1006h` via crossterm's `EnableMouseCapture`), where a move
/// to `(col, row)` is `CSI < 35 ; col+1 ; row+1 M`.
fn mouse_move(s: &mut Session, col: u16, row: u16) {
    s.type_text(&format!("\x1b[<35;{};{}M", col + 1, row + 1))
        .expect("mouse move report");
}

fn hover_over(s: &mut Session, needle: &str) {
    let (row, col) = find(s, needle);
    mouse_move(s, col + 2, row);
}

/// `notches` wheel-down steps over `needle`'s cell, paced like a send step.
fn wheel_down(s: &mut Session, needle: &str, notches: u32) {
    let (row, col) = find(s, needle);
    for _ in 0..notches {
        s.scroll(col, row, Scroll::Down).expect("wheel scroll");
        std::thread::sleep(Duration::from_millis(120));
    }
}

pub(crate) fn wheel_below(s: &mut Session, needle: &str, notches: u32) {
    let (row, col) = find(s, needle);
    for _ in 0..notches {
        s.scroll(col, row + 1, Scroll::Down).expect("wheel scroll");
        std::thread::sleep(Duration::from_millis(120));
    }
}

/// `notches` wheel-up steps over `needle`'s cell — the tail-following
/// surfaces (log, terminal viewport) move off the tail and reveal the
/// bottom fade.
fn wheel_up(s: &mut Session, needle: &str, notches: u32) {
    let (row, col) = find(s, needle);
    for _ in 0..notches {
        s.scroll(col, row, Scroll::Up).expect("wheel scroll");
        std::thread::sleep(Duration::from_millis(120));
    }
}

/// Resize `from` → `to`, waiting until the emulator reports the new geometry
/// before settling (the reflowed frame is the gated state).
fn resize_case(case: &Case, cols: u16, rows: u16) {
    let mut s = support::spawn_boot(case);
    resize_to(&mut s, cols, rows);
    find(&mut s, case.needle);
    support::settle_and_gate(&mut s, &case.name);
}

fn resize_to(s: &mut Session, cols: u16, rows: u16) {
    resize_geometry(s, cols, rows);
    // emulator blanks/scrolls the alt-screen on resize and reports the new
    // geometry before the app redraws (~300 ms): without this pause settle
    // can gate the blank post-resize frame
    std::thread::sleep(Duration::from_millis(700));
}

fn resize_geometry(s: &mut Session, cols: u16, rows: u16) {
    s.resize(cols, rows).expect("resize");
    s.wait_until(|screen| screen.size() == (cols, rows))
        .unwrap_or_else(|e| panic!("never reached {cols}x{rows}: {e:#}"));
}

// ------------------------------------------------------------------ hover --

crate::baseline_case_live!(
    showcase_hover_buttons_matrix => Case::new(
        "showcase/hover/buttons/120x40/truecolor",
        SHOWCASE,
        &["--page", "buttons"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        hover_over(s, "Preview");
    }
);

crate::baseline_case_live!(
    showcase_hover_lists_matrix => Case::new(
        "showcase/hover/lists/120x40/truecolor",
        SHOWCASE,
        &["--page", "lists"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        hover_over(s, "Python");
    }
);

crate::baseline_case_live!(
    showcase_hover_tables_matrix => Case::new(
        "showcase/hover/tables/120x40/truecolor",
        SHOWCASE,
        &["--page", "tables"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, variant| {
        if variant.cols == 72 {
            hover_over(s, "#1040");
        } else {
            hover_over(s, "#1042");
        }
    }
);

// ------------------------------------------------------------ drag-select --

crate::baseline_case_live!(
    showcase_flows_diff_drag_selected_matrix => Case::new(
        "showcase/flows/diff/drag-selected/120x40/truecolor",
        SHOWCASE,
        &["--page", "diff"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    )
    .sends(&["tab", "enter", "wait:● Review"]),
    |s, _| {
        let (row, col) = find(s, "attempts = 3");
        s.drag(col, row, col + 11, row).expect("drag select");
    }
);

// ------------------------------------------------------ S9 context menu --

// Right (secondary) click on a session row opens its context menu, titled
// with the row label (chrome.rs `PageEvent::Secondary`).
crate::baseline_case_live!(
    showcase_flows_chrome_context_matrix => Case::new(
        "showcase/flows/chrome/context/120x40/truecolor",
        SHOWCASE,
        &["--page", "chrome"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        let (row, col) = find(s, "Codex (Primary)");
        s.click_with(MouseButton::Right, col, row)
            .expect("right click");
    }
);

// --------------------------------------------------- S12 inspector scroll --

// The spec's "wheel over inspector" cannot work as written: the inspector
// registers no scroll region, so the wheel is `Outcome::Ignored` and the
// runtime (which repaints only on `Outcome::Changed`) never redraws — the
// frame would be byte-identical to `inspector_open`. The capturable form
// of the idea: inspector open on the lists page while a wheel scroll runs
// under it; the scroll returns Changed, and the repainted inspector shows
// the pointer's `mouse` row from the same interaction.
crate::baseline_case_live!(
    showcase_flows_inspector_scrolled_matrix => Case::new(
        "showcase/flows/inspector/scrolled/120x40/truecolor",
        SHOWCASE,
        &["--page", "lists"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    )
    .sends(&["i"]),
    |s, _| {
        wheel_down(s, "Python", 2);
    }
);

// ------------------------------------------------------ wheel scroll-fade --

crate::baseline_case_live!(
    showcase_fade_lists_wheel_fade_matrix => Case::new(
        "showcase/fade/lists/wheel-fade/120x40/truecolor",
        SHOWCASE,
        &["--page", "lists"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        wheel_down(s, "Python", 2);
    }
);

crate::baseline_case_live!(
    showcase_fade_trees_wheel_fade_matrix => Case::new(
        "showcase/fade/trees/wheel-fade/120x40/truecolor",
        SHOWCASE,
        &["--page", "trees"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, variant| {
        if variant.cols == 72 {
            wheel_down(s, "src", 1);
        } else {
            wheel_down(s, "config.rs", 1);
        }
    }
);

crate::baseline_case_live!(
    showcase_fade_datagrid_wheel_matrix => Case::new(
        "showcase/fade/datagrid/wheel/120x40/truecolor",
        SHOWCASE,
        &["--page", "datagrid"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        wheel_down(s, "Northwind Traders", 2);
    }
);

crate::baseline_case_live_with_compact_sends!(
    holla_fade_browser_wheel_matrix => Case::new(
        "holla/fade/browser_wheel/120x40/truecolor",
        HOLLA,
        &["--scenario", "parity-browser", "--motion", "reduced"],
        120,
        40,
        Color::Truecolor,
        HOLLA_BOOT,
    )
    .sends(&[
        "type:Browse ~/work/site",
        "enter",
        "wait:16 entries",
        "down",
        "down",
        "down",
        "wait:first 2000 lines",
    ]),
    100,
    &[
        "type:Browse ~/work/site",
        "enter",
        "wait:16 entries",
        "home",
        "down",
        "down",
        "down",
        "right",
        "wait:first 2000 lines",
    ],
    |s, _| {
        wheel_down(s, "line 5", 2);
    }
);

crate::baseline_case_live!(
    tablepro_fade_table_wheel_matrix => Case::new(
        "tablepro/fade/table_wheel/120x40/truecolor",
        TABLEPRO,
        &["--connect", "Production"],
        120,
        40,
        Color::Truecolor,
        "Query 1",
    )
    .sends(&[
        "down",
        "down",
        "down",
        "down",
        "down",
        "enter",
        "wait:public › orders",
    ]),
    |s, _| {
        wheel_down(s, "9157cff3", 3);
    }
);

crate::baseline_case_live!(
    showcase_fade_editor_wheel_matrix => Case::new(
        "showcase/fade/editor/wheel/120x40/truecolor",
        SHOWCASE,
        &["--page", "codeeditor"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, variant| {
        if variant.rows <= 24 {
            wheel_down(s, "pub async fn fetch", 2);
        } else {
            wheel_down(s, "sleep(delay).await", 2);
        }
    }
);

crate::baseline_case_live!(
    showcase_fade_textarea_wheel_matrix => Case::new(
        "showcase/fade/textarea/wheel/120x40/truecolor",
        SHOWCASE,
        &["--page", "textareas"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, variant| {
        if variant.rows <= 24 {
            wheel_down(s, "1. Read", 2);
        } else {
            wheel_down(s, "2. Keep the public API", 2);
        }
    }
);

crate::baseline_case_live!(
    showcase_fade_diff_wheel_matrix => Case::new(
        "showcase/fade/diff/wheel/120x40/truecolor",
        SHOWCASE,
        &["--page", "diff"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    )
    .sends(&["tab", "enter", "wait:● Review"]),
    |s, _| {
        wheel_down(s, "attempts = 3", 2);
    }
);

crate::baseline_case_live!(
    showcase_fade_scrolling_wheel_fade_matrix => Case::new(
        "showcase/fade/scrolling/wheel-fade/120x40/truecolor",
        SHOWCASE,
        &[
            "--page",
            "scrolling",
            "--motion",
            "paused",
            "--frame",
            "1600",
        ],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        wheel_up(s, "739.63s", 2);
    }
);

crate::baseline_case_live!(
    showcase_fade_scroll_page_matrix => Case::new(
        "showcase/fade/scroll/page/120x40/truecolor",
        SHOWCASE,
        &[
            "--page",
            "scrolling",
            "--motion",
            "paused",
            "--frame",
            "1600",
        ],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    )
    .sends(&["tab", "pagedown"]),
    |_, _| ()
);

crate::baseline_case_live!(
    showcase_fade_sidebars_wheel_matrix => Case::new(
        "showcase/fade/sidebars/wheel/72x20/truecolor",
        SHOWCASE,
        &["--page", "sidebars"],
        72,
        20,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        wheel_down(s, "Members", 2);
    }
);

crate::baseline_case_live!(
    showcase_fade_terminal_scrollback_matrix => Case::new(
        "showcase/fade/terminal/scrollback/80x24/truecolor",
        SHOWCASE,
        &["--page", "terminal", "--motion", "paused", "--frame", "60"],
        80,
        24,
        Color::Truecolor,
        SHOWCASE_BOOT,
    ),
    |s, _| {
        wheel_up(s, "#4 RUN cargo build", 2);
    }
);

// ----------------------------------------------------------------- resize --

crate::baseline_case_resize!(
    showcase_resize_overview_shrunk_80x24_truecolor => Case::new(
        "showcase/resize/overview_shrunk/80x24/truecolor",
        SHOWCASE,
        &["--page", "overview"],
        120,
        40,
        Color::Truecolor,
        SHOWCASE_BOOT,
    )
    .timeout(15_000),
    |case, cols, rows| {
        let mut s = support::spawn_boot(case);
        resize_geometry(&mut s, cols, rows);
        s.send_key("ctrl-l").expect("redraw tick");
        std::thread::sleep(Duration::from_millis(700));
        find(&mut s, "Foundations / Overview");
        support::settle_and_gate(&mut s, &case.name);
    }
);

crate::baseline_case_resize!(
    showcase_resize_overview_grown_120x40_truecolor => Case::new(
        "showcase/resize/overview_grown/120x40/truecolor",
        SHOWCASE,
        &["--page", "overview"],
        80,
        24,
        Color::Truecolor,
        SHOWCASE_BOOT,
    )
    .timeout(15_000),
    |case, cols, rows| {
        let mut s = support::spawn_boot(case);
        resize_geometry(&mut s, cols, rows);
        s.send_key("ctrl-l").expect("redraw tick");
        std::thread::sleep(Duration::from_millis(700));
        find(&mut s, "Foundations / Overview");
        support::settle_and_gate(&mut s, &case.name);
    }
);

crate::baseline_case_resize!(
    holla_resize_rust_dirty_shrunk_80x24_truecolor => Case::new(
        "holla/resize/rust-dirty_shrunk/80x24/truecolor",
        HOLLA,
        &[
            "--scenario",
            "rust-dirty",
            "--motion",
            "paused",
            "--frame",
            "40",
        ],
        120,
        40,
        Color::Truecolor,
        HOLLA_BOOT,
    )
    .timeout(30000),
    resize_case
);

crate::baseline_case_resize!(
    holla_resize_rust_dirty_grown_120x40_truecolor => Case::new(
        "holla/resize/rust-dirty_grown/120x40/truecolor",
        HOLLA,
        &[
            "--scenario",
            "rust-dirty",
            "--motion",
            "paused",
            "--frame",
            "40",
        ],
        80,
        24,
        Color::Truecolor,
        HOLLA_BOOT,
    )
    .timeout(30000),
    resize_case
);

crate::baseline_case_resize!(
    tablepro_resize_workbench_shrunk_80x24_truecolor => Case::new(
        "tablepro/resize/workbench_shrunk/80x24/truecolor",
        TABLEPRO,
        &["--connect", "Production"],
        120,
        40,
        Color::Truecolor,
        "Query 1",
    )
    .sends(&["wait:S audit"])
    .timeout(15000),
    |case, cols, rows| {
        let mut s = support::spawn_boot(case);
        resize_to(&mut s, cols, rows);
        s.wait_until(|screen| !screen.text().contains("Connected to"))
            .unwrap_or_else(|e| panic!("`Connected to` status never expired: {e:#}"));
        find(&mut s, "TablePro");
        support::settle_and_gate(&mut s, &case.name);
    }
);

crate::baseline_case_resize!(
    tablepro_resize_workbench_grown_120x40_truecolor => Case::new(
        "tablepro/resize/workbench_grown/120x40/truecolor",
        TABLEPRO,
        &["--connect", "Production"],
        80,
        24,
        Color::Truecolor,
        "Query 1",
    )
    .timeout(15000),
    |case, cols, rows| {
        let mut s = support::spawn_boot(case);
        resize_to(&mut s, cols, rows);
        if cols >= 120 {
            find(&mut s, "No results yet");
        }
        if cols >= 100 {
            find(&mut s, "S audit");
        } else {
            find(&mut s, "S public");
        }
        s.wait_until(|screen| !screen.text().contains("Connected to"))
            .unwrap_or_else(|e| panic!("`Connected to` status never expired: {e:#}"));
        support::settle_and_gate(&mut s, &case.name);
    }
);
