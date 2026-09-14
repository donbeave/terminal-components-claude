//! Shared driver for the visual-baseline suite.
//!
//! **Fast mode** — `TUISNAP_FAST=1`: settle 100 ms, `PtyOptions::input_pace`
//! 0 ms, tiered grouped gate (`GroupedCheckOptions { full_render: false }`
//! skips PNG/HTML when `.ansi` and `.txt` both match). **Smoke matrix** —
//! `TUISNAP_MATRIX=smoke`: only 120×40
//! truecolor; PR CI sets this via `cargo nextest run --profile ci …`.
//!
//! One PTY per capture: spawn → boot needle (or idle if none) → send
//! steps (120 ms pacing, 0 in fast mode) → `wait_stable`(settle) → frame → gate.
//! [`run_once`] runs the ported matrix; [`boot`]/[`drive`] mirror it for
//! the pointer group, which needs the live session afterwards. A leading
//! `wait:` needle is readiness — live clocks skip the 200 ms quiet window.
//!
//! Store: the grouped multi-artifact store (`tuisnap::grouped`). Approved
//! frames live at `snapshots/<group>/<sub_group>/<name>.{ansi,txt,png,html}`
//! (committed, exactly four artifacts per scenario); actuals, diffs and the
//! HTML report are scratch under `target/tuisnap/` (gitignored). The capture
//! name is the grouped path, e.g. `holla/parity/discovery/120x40/truecolor`.
//!
//! Colour hygiene per capture: ambient `NO_COLOR` is stripped (crossterm
//! honours it by *presence* and would silently flatten every colour frame —
//! the lesson that forced the 2026-09-12 baseline re-capture), the motion
//! kill-switches `HOLLA_NO_MOTION`/`JACKIN_NO_MOTION` and the colour-forcing
//! `CLICOLOR_FORCE`/`FORCE_COLOR` are stripped for the same reason, and
//! `HOLLA_NO_HISTORY=1` suppresses history side effects; `nocolor` captures
//! re-add `NO_COLOR=1` instead of a `--color` flag (backend rule, not app
//! rule).

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tuisnap::grouped::{GroupedCheckOptions, GroupedOutcome, GroupedStore};
use tuisnap::pty::{run_once, PtyOptions, Session};
use tuisnap::snapshot::Status;
use tuisnap::{Frame, Profile, Renderer, VENDORED_FACES};

pub const SHOWCASE: &str = env!("CARGO_BIN_EXE_showcase");
pub const TABLEPRO: &str = env!("CARGO_BIN_EXE_tablepro");
pub const JACKIN: &str = env!("CARGO_BIN_EXE_jackin-preview");
pub const HOLLA: &str = env!("CARGO_BIN_EXE_holla");

/// Default settle from the bash runner (400 ms). Prefer [`settle`] at runtime.
pub const SETTLE: Duration = Duration::from_millis(400);

const SMOKE_SIZES: [(u16, u16); 1] = [(120, 40)];

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// `TUISNAP_FAST=1`: shorter settle, zero step pacing, tiered gate.
pub fn fast_mode() -> bool {
    env_flag("TUISNAP_FAST")
}

/// `TUISNAP_MATRIX=smoke`, or `NEXTEST_PROFILE=ci` (nextest has no per-test
/// `env` override; the ci profile sets the matrix via this hook).
pub fn smoke_matrix() -> bool {
    std::env::var("TUISNAP_MATRIX")
        .map(|v| v.eq_ignore_ascii_case("smoke"))
        .unwrap_or(false)
        || std::env::var("NEXTEST_PROFILE")
            .map(|v| v == "ci")
            .unwrap_or(false)
}

/// Active settle window (`TUISNAP_FAST=1` → 100 ms, else 400 ms).
pub fn settle() -> Duration {
    if fast_mode() {
        Duration::from_millis(100)
    } else {
        SETTLE
    }
}

/// Inter-step pacing after keys/types (`TUISNAP_FAST=1` → 0 ms, else 120 ms).
pub fn step_pace() -> Duration {
    if fast_mode() {
        Duration::ZERO
    } else {
        Duration::from_millis(120)
    }
}

/// Matrix sizes for canonical expansion (`TUISNAP_MATRIX=smoke` → 120×40 only).
pub fn matrix_sizes() -> &'static [(u16, u16)] {
    if smoke_matrix() {
        &SMOKE_SIZES
    } else {
        &CANONICAL_SIZES
    }
}

/// Matrix colours for canonical expansion (`TUISNAP_MATRIX=smoke` → truecolor only).
pub fn matrix_colors() -> &'static [Color] {
    if smoke_matrix() {
        &SMOKE_COLORS
    } else {
        &CANONICAL_COLORS
    }
}
/// TIMEOUT_MS default; boot-streaming screens override it per capture
/// ([`Case::timeout`], the bash `CAP_TIMEOUT=` prefix).
pub const TIMEOUT_MS: u64 = 8_000;

/// Canonical matrix axes: every canonical capture root and audit fixture is
/// captured at all 5 sizes × 5 colours.
pub const CANONICAL_SIZES: [(u16, u16); 5] = [(72, 20), (80, 24), (100, 30), (120, 40), (160, 50)];
pub const CANONICAL_COLORS: [Color; 5] = [
    Color::Truecolor,
    Color::Ansi256,
    Color::Ansi16,
    Color::None,
    Color::NoColorEnv,
];
const SMOKE_COLORS: [Color; 1] = [Color::Truecolor];
/// Prefixes for the 9 `audit_matrix` fixtures (`{prefix}/{cols}x{rows}/{color}`).
/// Accounts is a custom loop with the same name shape ([`AUDIT_PREFIX_JACKIN_ACCOUNTS`]).
pub const AUDIT_PREFIX_HOLLA_RUST: &str = "holla/audit/rust";
pub const AUDIT_PREFIX_HOLLA_UPGRADE: &str = "holla/audit/upgrade";
pub const AUDIT_PREFIX_JACKIN_CAPSULE: &str = "jackin/audit/capsule";
pub const AUDIT_PREFIX_SHOWCASE_BUTTONS: &str = "showcase/audit/buttons";
pub const AUDIT_PREFIX_SHOWCASE_DIFF: &str = "showcase/audit/diff";
pub const AUDIT_PREFIX_SHOWCASE_FORMS: &str = "showcase/audit/forms";
pub const AUDIT_PREFIX_SHOWCASE_INPUTS: &str = "showcase/audit/inputs";
pub const AUDIT_PREFIX_SHOWCASE_TEXTAREAS: &str = "showcase/audit/textareas";
pub const AUDIT_PREFIX_TABLEPRO_PRODUCTION: &str = "tablepro/audit/production";
pub const AUDIT_PREFIX_JACKIN_ACCOUNTS: &str = "jackin/audit/accounts";

pub const AUDIT_MATRIX_PREFIXES: [&str; 9] = [
    AUDIT_PREFIX_HOLLA_RUST,
    AUDIT_PREFIX_HOLLA_UPGRADE,
    AUDIT_PREFIX_JACKIN_CAPSULE,
    AUDIT_PREFIX_SHOWCASE_BUTTONS,
    AUDIT_PREFIX_SHOWCASE_DIFF,
    AUDIT_PREFIX_SHOWCASE_FORMS,
    AUDIT_PREFIX_SHOWCASE_INPUTS,
    AUDIT_PREFIX_SHOWCASE_TEXTAREAS,
    AUDIT_PREFIX_TABLEPRO_PRODUCTION,
];

pub fn audit_default_name(prefix: &str, cols: u16, rows: u16, color: Color) -> String {
    format!("{prefix}/{cols}x{rows}/{}", color.suffix())
}

/// Every capture name the suite produces: the canonical 5×5 expansion of each
/// Case::new root and the data-driven audit matrices. `Case::dynamic` loops are
/// not parsed; their names come from the audit constants below.
pub fn suite_capture_names() -> BTreeSet<String> {
    let mut names = parse_case_new_names();
    names.extend(generated_matrix_names());
    names
}

fn generated_matrix_names() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for prefix in AUDIT_MATRIX_PREFIXES {
        for &(cols, rows) in &CANONICAL_SIZES {
            for color in CANONICAL_COLORS {
                names.insert(audit_default_name(prefix, cols, rows, color));
            }
        }
    }
    for &(cols, rows) in &CANONICAL_SIZES {
        for color in CANONICAL_COLORS {
            names.insert(audit_default_name(
                AUDIT_PREFIX_JACKIN_ACCOUNTS,
                cols,
                rows,
                color,
            ));
        }
    }
    names
}

/// `name` is `<root>/<cols>x<rows>/<color>`; resize roots use the same
/// canonical matrix as every other capture root.
fn canonical_root(name: &str) -> Option<&str> {
    name.rsplit_once('/')
        .and_then(|(without_color, _)| without_color.rsplit_once('/'))
        .map(|(root, _)| root)
}

fn canonical_name(root: &str, cols: u16, rows: u16, color: Color) -> String {
    format!("{root}/{cols}x{rows}/{}", color.suffix())
}

fn parse_case_new_names() -> BTreeSet<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/visual_baseline");
    let mut names = BTreeSet::new();
    let mut declared_roots = BTreeSet::new();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .path();
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if path.extension().and_then(|s| s.to_str()) != Some("rs")
            || file_name == "support.rs"
            || file_name == "main.rs"
        {
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        extract_case_new_roots(&src, &mut names, &mut declared_roots);
    }
    assert!(
        !names.is_empty(),
        "no Case::new names parsed from {}",
        dir.display()
    );
    names
}

/// Parse only the representative in each `baseline_case*!` invocation. Variant
/// arrays can contain additional exact per-combo declarations; they select at
/// runtime and must not create duplicate canonical roots in the inventory.
fn extract_case_new_roots(
    src: &str,
    names: &mut BTreeSet<String>,
    declared_roots: &mut BTreeSet<String>,
) {
    const MACRO_MARK: &str = "crate::baseline_case";
    const MARK: &str = "Case::new(";
    let mut rest = src;
    while let Some(macro_i) = rest.find(MACRO_MARK) {
        // Flow tests declare representatives outside macros. Parse those raw
        // declarations before skipping the entire macro invocation.
        extract_raw_case_new_roots(&rest[..macro_i], names, declared_roots);
        let invocation_end = baseline_invocation_end(&rest[macro_i..]);
        let invocation = &rest[macro_i..macro_i + invocation_end];
        let Some(case_i) = invocation.find(MARK) else {
            rest = &rest[macro_i + MACRO_MARK.len()..];
            continue;
        };
        let body = invocation[case_i + MARK.len()..].trim_start();
        let Some(body) = body.strip_prefix('"') else {
            panic!("baseline_case representative is missing its name literal");
        };
        let Some(end) = body.find('"') else {
            panic!("unterminated Case::new string literal");
        };
        let name = &body[..end];
        if let Some(root) = canonical_root(name) {
            assert!(
                declared_roots.insert(root.to_string()),
                "duplicate representative declaration for canonical root `{root}`"
            );
            for &(cols, rows) in &CANONICAL_SIZES {
                for color in CANONICAL_COLORS {
                    names.insert(canonical_name(root, cols, rows, color));
                }
            }
        } else {
            names.insert(name.to_string());
        }
        rest = &rest[macro_i + invocation_end..];
    }
    extract_raw_case_new_roots(rest, names, declared_roots);
}

/// Return the offset just past a balanced `baseline_case*!(...)` invocation.
/// Sends can contain arbitrary text, so `;` is not a safe invocation boundary.
fn baseline_invocation_end(src: &str) -> usize {
    let open = src.find('(').expect("baseline_case invocation missing `(`");
    let mut depth = 1;
    let mut in_string = false;
    let mut escaped = false;
    for (relative, byte) in src[open + 1..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == '\\' {
                escaped = true;
            } else if byte == '"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + 1 + relative + byte.len_utf8();
                }
            }
            _ => {}
        }
    }
    panic!("unterminated baseline_case invocation");
}

fn extract_raw_case_new_roots(
    src: &str,
    names: &mut BTreeSet<String>,
    declared_roots: &mut BTreeSet<String>,
) {
    const MARK: &str = "Case::new(";
    let mut rest = src;
    while let Some(i) = rest.find(MARK) {
        rest = rest[i + MARK.len()..].trim_start();
        let Some(body) = rest.strip_prefix('"') else {
            continue;
        };
        let Some(end) = body.find('"') else {
            panic!("unterminated Case::new string literal");
        };
        let name = &body[..end];
        if let Some(root) = canonical_root(name) {
            assert!(
                declared_roots.insert(root.to_string()),
                "duplicate representative declaration for canonical root `{root}`"
            );
            for &(cols, rows) in &CANONICAL_SIZES {
                for color in CANONICAL_COLORS {
                    names.insert(canonical_name(root, cols, rows, color));
                }
            }
        } else {
            names.insert(name.to_string());
        }
        rest = &body[end + 1..];
    }
}

/// Exact-name filter for macOS Finder metadata. Not snapshot content; every
/// other unknown file remains a store-integrity failure.
pub fn is_macos_platform_metadata(path: &Path) -> bool {
    path.file_name() == Some(std::ffi::OsStr::new(".DS_Store"))
}

/// `--color` palette, or real `NO_COLOR=1` (the baseline's `nocolor`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Truecolor,
    Ansi256,
    Ansi16,
    None,
    NoColorEnv,
}

impl Color {
    /// Leaf suffix in capture names (`no_color` is spelled `nocolor`).
    pub fn suffix(self) -> &'static str {
        match self {
            Color::Truecolor => "truecolor",
            Color::Ansi256 => "256",
            Color::Ansi16 => "16",
            Color::None => "none",
            Color::NoColorEnv => "nocolor",
        }
    }
}

/// One capture definition — the typed form of one bash `cap` call.
#[derive(Clone)]
pub struct Case {
    /// Grouped store name (`<app>/<sub_group>/<surface>_<state>_<cols>x<rows>_<color>`);
    /// owned when built by the data-driven matrices.
    pub name: Cow<'static, str>,
    pub bin: &'static str,
    /// argv after the binary, before `--color` (e.g. `&["--page", "diff"]`).
    pub args: &'static [&'static str],
    pub cols: u16,
    pub rows: u16,
    pub color: Color,
    /// Boot needle on the fully-rendered first screen (`""` skips the wait);
    /// sent as the first step, before the sends, exactly as the bash runner
    /// did (the CLI's `--wait-for` would have run after them).
    pub needle: &'static str,
    /// DSL steps after the boot wait: key names, `type:<text>`,
    /// `sleep:<ms>`, `wait:<needle>`.
    pub sends: &'static [&'static str],
    pub timeout_ms: u64,
}

impl Case {
    pub fn new(
        name: &'static str,
        bin: &'static str,
        args: &'static [&'static str],
        cols: u16,
        rows: u16,
        color: Color,
        needle: &'static str,
    ) -> Self {
        Self {
            name: Cow::Borrowed(name),
            bin,
            args,
            cols,
            rows,
            color,
            needle,
            sends: &[],
            timeout_ms: TIMEOUT_MS,
        }
    }

    /// Owned-name form for the data-driven matrices (audit 5×5, audit-flows).
    pub fn dynamic(
        name: String,
        bin: &'static str,
        args: &'static [&'static str],
        cols: u16,
        rows: u16,
        color: Color,
        needle: &'static str,
    ) -> Self {
        Self {
            name: Cow::Owned(name),
            bin,
            args,
            cols,
            rows,
            color,
            needle,
            sends: &[],
            timeout_ms: TIMEOUT_MS,
        }
    }

    pub fn sends(self, sends: &'static [&'static str]) -> Self {
        Self { sends, ..self }
    }

    /// CAP_TIMEOUT override for screens whose boot stream outlasts
    /// TIMEOUT_MS (scrolling, terminal): the boot wait_idle shares it.
    pub fn timeout(self, ms: u64) -> Self {
        Self {
            timeout_ms: ms,
            ..self
        }
    }

    /// Re-root a representative declaration at another canonical combo while
    /// preserving its argv, boot needle, sends, and timeout drift.
    pub(crate) fn variant(&self, cols: u16, rows: u16, color: Color) -> Self {
        let root = canonical_root(&self.name).unwrap_or_else(|| {
            panic!(
                "`{}` is not a canonical `<root>/<size>/<color>` capture",
                self.name
            )
        });
        Self {
            name: Cow::Owned(canonical_name(root, cols, rows, color)),
            bin: self.bin,
            args: self.args,
            cols,
            rows,
            color,
            needle: self.needle,
            sends: self.sends,
            timeout_ms: self.timeout_ms,
        }
    }

    /// Re-root a resize capture at its target geometry while retaining the
    /// representative's initial PTY geometry and capture behavior.
    pub fn resize_variant(&self, cols: u16, rows: u16, color: Color) -> Self {
        let root = canonical_root(&self.name).unwrap_or_else(|| {
            panic!(
                "`{}` is not a canonical `<root>/<size>/<color>` capture",
                self.name
            )
        });
        Self {
            name: Cow::Owned(canonical_name(root, cols, rows, color)),
            bin: self.bin,
            args: self.args,
            cols: self.cols,
            rows: self.rows,
            color,
            needle: self.needle,
            sends: self.sends,
            timeout_ms: self.timeout_ms,
        }
    }
}

pub fn argv_for(case: &Case) -> Vec<String> {
    let mut argv = Vec::with_capacity(case.args.len() + 3);
    argv.push(case.bin.to_string());
    argv.extend(case.args.iter().map(|s| s.to_string()));
    let flag = match case.color {
        Color::Truecolor => Some("truecolor"),
        Color::Ansi256 => Some("256"),
        Color::Ansi16 => Some("16"),
        Color::None => Some("none"),
        Color::NoColorEnv => Option::None,
    };
    if let Some(flag) = flag {
        argv.push("--color".to_string());
        argv.push(flag.to_string());
    }
    argv
}

pub fn opts_for(case: &Case) -> PtyOptions {
    let opts = PtyOptions {
        cols: case.cols,
        rows: case.rows,
        timeout: Duration::from_millis(case.timeout_ms),
        input_pace: step_pace(),
        ..PtyOptions::default()
    }
    .without_env("NO_COLOR")
    .without_env("HOLLA_NO_MOTION")
    .without_env("JACKIN_NO_MOTION")
    .without_env("CLICOLOR_FORCE")
    .without_env("FORCE_COLOR")
    .with_env("HOLLA_NO_HISTORY", "1");
    match case.color {
        Color::NoColorEnv => opts.with_env("NO_COLOR", "1"),
        _ => opts,
    }
}

/// Boot needle first (a `wait:` step), then the sends — the bash runner's
/// step order.
fn steps_for(case: &Case) -> Vec<String> {
    let mut steps = Vec::with_capacity(case.sends.len() + 1);
    if !case.needle.is_empty() {
        steps.push(format!("wait:{}", case.needle));
    }
    steps.extend(case.sends.iter().map(|s| s.to_string()));
    steps
}

thread_local! {
    /// One renderer per test thread: font faces parsed once, glyph rasters
    /// cached across every check on the thread (no locks).
    static RENDERER: RefCell<Renderer> = RefCell::new(
        Profile::default_profile()
            .renderer(&VENDORED_FACES)
            .expect("vendored faces parse"),
    );
}

/// The grouped store: approved tree at `snapshots/` (committed), scratch
/// (actuals, diffs, report) under `target/tuisnap/` (gitignored).
pub fn store() -> GroupedStore {
    GroupedStore::new(Path::new("snapshots"))
        .with_actual_root(Path::new("target/tuisnap/actual"))
        .with_diff_root(Path::new("target/tuisnap/diff"))
        .with_report_path(Path::new("target/tuisnap/report.html"))
}

/// Cell-exact (ansi) + content (txt) + render-level (html) byte gates +
/// pixel-exact gate at threshold 1.0 through the thread's cached renderer.
/// Writes `target/tuisnap/actual/` artifacts even when unmatched.
///
/// With `TUISNAP_FAST=1`, uses tuisnap's tiered grouped gate (skips PNG/HTML
/// render when ansi+txt gates pass).
pub fn gate(name: &str, frame: &Frame) -> GroupedOutcome {
    RENDERER.with(|r| {
        let options = GroupedCheckOptions {
            full_render: !fast_mode(),
        };
        store()
            .check_with_options(&mut r.borrow_mut(), name, frame, 1.0, &options)
            .unwrap_or_else(|e| panic!("gate `{name}` failed: {e}"))
    })
}

/// Fail-closed assertion: only `matched` passes. Missing approval remains
/// pending after capture, but the test fails until the full suite is
/// generated and explicitly blessed (`tuisnap accept --grouped` is the only
/// bless, never the test); drift, dimension mismatch and corrupt approval
/// also fail.
pub fn assert_gated(outcome: &GroupedOutcome) {
    match outcome.status() {
        Status::Matched => eprintln!("baseline matched   {}", outcome.outcome.name),
        Status::MissingApproval => panic!(
            "baseline missing approval: {} (generate, review, then bless explicitly)",
            outcome.outcome.name
        ),
        _ => panic!("{}", outcome.ensure_matched().unwrap_err()),
    }
}

/// One-shot scripted capture via tuisnap `run_once` (`input_pace` from opts).
fn capture_once(argv: &[String], opts: &PtyOptions, sends: &[String]) -> Frame {
    run_once(argv, opts, sends, settle())
        .unwrap_or_else(|e| panic!("capture failed: {e:#}"))
}

/// A ported-matrix capture: the same runner the `tuisnap run` CLI used.
pub fn run_and_assert(case: &Case) {
    let frame = capture_once(&argv_for(case), &opts_for(case), &steps_for(case));
    assert_gated(&gate(&case.name, &frame));
}

/// Whether `(cols, rows, color)` is in the active matrix (`TUISNAP_MATRIX=smoke`
/// trims to 120×40 truecolor).
pub fn combo_in_active_matrix(cols: u16, rows: u16, color: Color) -> bool {
    matrix_sizes()
        .iter()
        .any(|&(c, r)| c == cols && r == rows)
        && matrix_colors().iter().any(|&c| c == color)
}

/// When set, matrix runners keep going after a combo failure so regeneration
/// workflows write every actual before panicking ([`collect_matrix`] /
/// [`finish_matrix`]).
#[allow(dead_code)] // TUISNAP_BLESS=1 regeneration; per-combo tests use run_and_assert
pub fn matrix_bless_mode() -> bool {
    env_flag("TUISNAP_BLESS")
}

#[allow(dead_code)]
fn run_matrix_combo(combo: &str, body: impl FnOnce()) -> bool {
    if matrix_bless_mode() {
        collect_matrix(combo, body)
    } else {
        body();
        true
    }
}

/// Validate legacy per-combo declarations for a canonical root.
pub fn validate_canonical_variants(representative: &Case, variants: &[Case]) {
    let root = canonical_root(&representative.name).unwrap_or_else(|| {
        panic!(
            "`{}` is not a canonical `<root>/<size>/<color>` capture",
            representative.name
        )
    });
    let mut selected = std::collections::BTreeMap::new();
    for variant in variants {
        let variant_root = canonical_root(&variant.name).unwrap_or_else(|| {
            panic!(
                "variant `{}` is not a canonical `<root>/<size>/<color>` capture",
                variant.name
            )
        });
        assert!(
            variant_root == root,
            "variant `{variant_root}` does not belong to representative root `{root}`"
        );
        let expected_name = canonical_name(variant_root, variant.cols, variant.rows, variant.color);
        assert!(
            variant.name.as_ref() == expected_name,
            "variant `{}` conflicts with its declared {}/{}/{} combo",
            variant.name,
            variant.cols,
            variant.rows,
            variant.color.suffix()
        );
        let key = (variant.cols, variant.rows, variant.color.suffix());
        assert!(
            selected.insert(key, ()).is_none(),
            "conflicting legacy declarations for `{root}` at {}/{}/{}",
            variant.cols,
            variant.rows,
            variant.color.suffix()
        );
    }
}

/// Pick the explicit legacy declaration for one combo, or inherit the
/// representative.
pub fn case_for_combo(
    representative: &Case,
    variants: &[Case],
    cols: u16,
    rows: u16,
    color: Color,
) -> Case {
    if let Some(variant) = variants.iter().find(|variant| {
        variant.cols == cols && variant.rows == rows && variant.color == color
    }) {
        variant.variant(cols, rows, color)
    } else {
        representative.variant(cols, rows, color)
    }
}

/// Expand one representative static Case::new root through the full canonical
/// matrix. The representative's sends/timeout apply to every combo; choose a
/// representative whose determinism contract is size-independent.
#[allow(dead_code)]
pub fn run_canonical(representative: &Case) {
    let mut failures = Vec::new();
    for &(cols, rows) in matrix_sizes() {
        for &color in matrix_colors() {
            let case = representative.variant(cols, rows, color);
            let name = case.name.to_string();
            if !run_matrix_combo(&name, || run_and_assert(&case)) {
                failures.push(name);
            }
        }
    }
    finish_matrix(&failures);
}

/// Expand one representative while preserving exact legacy declarations.
///
/// A root may have had distinct settings at specific old size/color combos
/// (for example, a larger boot frame or an extra readiness wait). Those full
/// declarations are passed explicitly and win for their exact combo; every
/// combo without an old declaration inherits the representative. Duplicate
/// declarations for one combo are a configuration conflict, not a precedence
/// rule.
#[allow(dead_code)]
pub fn run_canonical_with_variants(representative: &Case, variants: &[Case]) {
    validate_canonical_variants(representative, variants);

    let mut failures = Vec::new();
    for &(cols, rows) in matrix_sizes() {
        for &color in matrix_colors() {
            let case = case_for_combo(representative, variants, cols, rows, color);
            let name = case.name.to_string();
            if !run_matrix_combo(&name, || run_and_assert(&case)) {
                failures.push(name);
            }
        }
    }
    finish_matrix(&failures);
}

/// Expand a live pointer/keyboard/manual-flow root through the same matrix.
/// `interact` runs after the centrally driven boot + case sends and before the
/// centrally settled/gated capture.
#[allow(dead_code)]
pub fn run_canonical_live(representative: &Case, mut interact: impl FnMut(&mut Session, &Case)) {
    let mut failures = Vec::new();
    for &(cols, rows) in matrix_sizes() {
        for &color in matrix_colors() {
            let case = representative.variant(cols, rows, color);
            let name = case.name.to_string();
            if !run_matrix_combo(&name, || {
                let mut session = spawn_boot(&case);
                interact(&mut session, &case);
                settle_and_gate(&mut session, &case.name);
            }) {
                failures.push(name);
            }
        }
    }
    finish_matrix(&failures);
}

/// Expand a representative live root, substituting a compact send chain for
/// terminal widths at or below `max_cols`. This is for responsive layouts
/// that need an explicit drawer/detail step which the wide representative's
/// sends cannot express; coverage remains the full canonical 5×5 matrix.
#[allow(dead_code)]
pub fn run_canonical_live_with_compact_sends(
    representative: &Case,
    max_cols: u16,
    compact_sends: &'static [&'static str],
    mut interact: impl FnMut(&mut Session, &Case),
) {
    let mut failures = Vec::new();
    for &(cols, rows) in matrix_sizes() {
        for &color in matrix_colors() {
            let mut case = representative.variant(cols, rows, color);
            if cols <= max_cols {
                case.sends = compact_sends;
            }
            let name = case.name.to_string();
            if !run_matrix_combo(&name, || {
                let mut session = spawn_boot(&case);
                interact(&mut session, &case);
                settle_and_gate(&mut session, &case.name);
            }) {
                failures.push(name);
            }
        }
    }
    finish_matrix(&failures);
}

/// Run `body` for each combo of a data-driven matrix without stopping at the
/// first failure: every combo's actuals are written before the fn panics, so
/// an intentional-change run regenerates the whole matrix in one pass. The
/// fn still fails loudly, with every failed combo named.
#[allow(dead_code)]
pub fn collect_matrix(combo: &str, body: impl FnOnce()) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(()) => true,
        Err(e) => {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown panic".into());
            eprintln!("matrix combo FAILED {combo}: {msg}");
            false
        }
    }
}

/// Panic if any [`collect_matrix`] call reported a failure.
#[allow(dead_code)]
pub fn finish_matrix(failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} matrix capture(s) failed: {}",
        failures.len(),
        failures.join(", ")
    );
}

/// Spawn a case's session for the pointer group (mouse/resize captures need
/// the live session after boot).
pub fn spawn(case: &Case) -> Session {
    Session::spawn(&argv_for(case), &opts_for(case))
        .unwrap_or_else(|e| panic!("spawn `{}` failed: {e:#}", case.name))
}

/// Central live-session setup: boot needle first, then all case sends. This
/// mirrors [`run_once`] while handing the connected session back for pointer
/// or manual-flow assertions.
pub fn spawn_boot(case: &Case) -> Session {
    let mut session = spawn(case);
    boot(&mut session, case.needle);
    if !case.sends.is_empty() {
        drive(&mut session, case.sends);
    }
    session
}

/// Boot: needle first. Live clocks starve a quiet-window wait_idle.
pub fn boot(session: &mut Session, needle: &str) {
    if !needle.is_empty() {
        session
            .wait_for_text(needle)
            .unwrap_or_else(|e| panic!("boot needle `{needle}` never appeared: {e:#}"));
        return;
    }
    session
        .wait_idle(Duration::from_millis(200))
        .unwrap_or_else(|e| panic!("boot idle failed: {e:#}"));
}

/// `run_once`'s step loop on a live session, pacing included.
pub fn drive(session: &mut Session, steps: &[&str]) {
    let pace = step_pace();
    for step in steps {
        if let Some(ms) = step.strip_prefix("sleep:") {
            std::thread::sleep(Duration::from_millis(ms.parse().expect("sleep:<ms>")));
        } else if let Some(needle) = step.strip_prefix("wait:") {
            session
                .wait_for_text(needle)
                .unwrap_or_else(|e| panic!("`wait:{needle}` timed out: {e:#}"));
        } else if let Some(text) = step.strip_prefix("type:") {
            session.type_text(text).expect("type_text");
            if !pace.is_zero() {
                std::thread::sleep(pace);
            }
        } else {
            session.send_key(step).expect("send_key");
            if !pace.is_zero() {
                std::thread::sleep(pace);
            }
        }
    }
}

/// Jackin accounts audit matrix: badge readiness on short viewports.
pub fn run_jackin_accounts_matrix(case: &Case) {
    let mut session = spawn(case);
    boot(&mut session, case.needle);
    if case.rows <= 30 {
        session
            .wait_for_text("of 23")
            .unwrap_or_else(|e| panic!("accounts badge never rendered: {e:#}"));
    }
    settle_and_gate(&mut session, &case.name);
}

/// Settle the screen and gate the capture.
pub fn settle_and_gate(session: &mut Session, name: &str) {
    let frame = session
        .wait_stable(settle())
        .unwrap_or_else(|e| panic!("`{name}` never settled: {e:#}"));
    assert_gated(&gate(name, &frame));
}

/// One `#[test]` per canonical size×colour combo (25 per root). Each test
/// runs a single PTY capture so nextest can saturate the PTY pool. Module
/// names preserve the representative filter prefix (`test(holla_parity_…)`).
#[macro_export]
macro_rules! canonical_combo_tests {
    ($select:expr, $($op:tt)+) => {
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            72,
            20,
            $crate::support::Color::Truecolor,
            c72x20_truecolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            72,
            20,
            $crate::support::Color::Ansi256,
            c72x20_256
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            72,
            20,
            $crate::support::Color::Ansi16,
            c72x20_16
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            72,
            20,
            $crate::support::Color::None,
            c72x20_none
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            72,
            20,
            $crate::support::Color::NoColorEnv,
            c72x20_nocolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            80,
            24,
            $crate::support::Color::Truecolor,
            c80x24_truecolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            80,
            24,
            $crate::support::Color::Ansi256,
            c80x24_256
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            80,
            24,
            $crate::support::Color::Ansi16,
            c80x24_16
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            80,
            24,
            $crate::support::Color::None,
            c80x24_none
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            80,
            24,
            $crate::support::Color::NoColorEnv,
            c80x24_nocolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            100,
            30,
            $crate::support::Color::Truecolor,
            c100x30_truecolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            100,
            30,
            $crate::support::Color::Ansi256,
            c100x30_256
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            100,
            30,
            $crate::support::Color::Ansi16,
            c100x30_16
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            100,
            30,
            $crate::support::Color::None,
            c100x30_none
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            100,
            30,
            $crate::support::Color::NoColorEnv,
            c100x30_nocolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            120,
            40,
            $crate::support::Color::Truecolor,
            c120x40_truecolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            120,
            40,
            $crate::support::Color::Ansi256,
            c120x40_256
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            120,
            40,
            $crate::support::Color::Ansi16,
            c120x40_16
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            120,
            40,
            $crate::support::Color::None,
            c120x40_none
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            120,
            40,
            $crate::support::Color::NoColorEnv,
            c120x40_nocolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            160,
            50,
            $crate::support::Color::Truecolor,
            c160x50_truecolor
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            160,
            50,
            $crate::support::Color::Ansi256,
            c160x50_256
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            160,
            50,
            $crate::support::Color::Ansi16,
            c160x50_16
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            160,
            50,
            $crate::support::Color::None,
            c160x50_none
        );
        $crate::__canonical_combo_test!(
            $select,
            $($op)+,
            160,
            50,
            $crate::support::Color::NoColorEnv,
            c160x50_nocolor
        );
    };
}

#[macro_export]
macro_rules! __canonical_combo_test {
    ($select:expr, run_and_assert, $cols:literal, $rows:literal, $color:expr, $fn:ident) => {
        #[test]
        #[ignore = "visual baseline capture; run with --ignored"]
        fn $fn() {
            if !$crate::support::combo_in_active_matrix($cols, $rows, $color) {
                return;
            }
            let case = ($select)($cols, $rows, $color);
            $crate::support::run_and_assert(&case);
        }
    };
    ($select:expr, live($interact:expr), $cols:literal, $rows:literal, $color:expr, $fn:ident) => {
        #[test]
        #[ignore = "visual baseline capture; run with --ignored"]
        fn $fn() {
            if !$crate::support::combo_in_active_matrix($cols, $rows, $color) {
                return;
            }
            let case = ($select)($cols, $rows, $color);
            let mut session = $crate::support::spawn_boot(&case);
            let interact: fn(&mut ::tuisnap::pty::Session, &$crate::support::Case) = $interact;
            interact(&mut session, &case);
            $crate::support::settle_and_gate(&mut session, &case.name);
        }
    };
    ($select:expr, resize($capture:expr), $cols:literal, $rows:literal, $color:expr, $fn:ident) => {
        #[test]
        #[ignore = "visual baseline capture; run with --ignored"]
        fn $fn() {
            if !$crate::support::combo_in_active_matrix($cols, $rows, $color) {
                return;
            }
            let case = ($select)($cols, $rows, $color);
            ($capture)(&case, $cols, $rows);
        }
    };
    ($select:expr, jackin_accounts, $cols:literal, $rows:literal, $color:expr, $fn:ident) => {
        #[test]
        #[ignore = "visual baseline capture; run with --ignored"]
        fn $fn() {
            if !$crate::support::combo_in_active_matrix($cols, $rows, $color) {
                return;
            }
            let case = ($select)($cols, $rows, $color);
            $crate::support::run_jackin_accounts_matrix(&case);
        }
    };
}

/// One `#[test]` per canonical root, generated from the representative static
/// case tables so cargo name filters work. Expands to 25 combo tests under a
/// module named for the representative capture.
#[macro_export]
macro_rules! baseline_case {
    ($mod_name:ident => $case:expr) => {
        mod $mod_name {
            use super::*;
            $crate::canonical_combo_tests!(
                |cols, rows, color| ($case).variant(cols, rows, color),
                run_and_assert
            );
        }
    };
}

/// One module of combo tests per canonical root, with exact declarations for
/// legacy combos.
#[macro_export]
macro_rules! baseline_case_with_variants {
    ($mod_name:ident => $case:expr, [$($variant:expr),* $(,)?] $(,)?) => {
        mod $mod_name {
            use super::*;
            fn variants_checked() -> &'static [Case] {
                static VARIANTS: std::sync::OnceLock<Vec<Case>> = std::sync::OnceLock::new();
                VARIANTS
                    .get_or_init(|| {
                        let variants = vec![$($variant),*];
                        $crate::support::validate_canonical_variants(&$case, &variants);
                        variants
                    })
                    .as_slice()
            }
            $crate::canonical_combo_tests!(
                |cols, rows, color| {
                    $crate::support::case_for_combo(&$case, variants_checked(), cols, rows, color)
                },
                run_and_assert
            );
        }
    };
}

/// Live-session matrix: one PTY per combo after boot/sends + interact closure.
#[macro_export]
macro_rules! baseline_case_live {
    ($mod_name:ident => $case:expr, $interact:expr) => {
        mod $mod_name {
            use super::*;
            $crate::canonical_combo_tests!(
                |cols, rows, color| ($case).variant(cols, rows, color),
                live($interact)
            );
        }
    };
}

/// Live matrix with a compact send chain at or below `max_cols`.
#[macro_export]
macro_rules! baseline_case_live_with_compact_sends {
    ($mod_name:ident => $case:expr, $max_cols:literal, $compact_sends:expr, $interact:expr) => {
        mod $mod_name {
            use super::*;
            $crate::canonical_combo_tests!(
                |cols, rows, color| {
                    let mut case = ($case).variant(cols, rows, color);
                    if cols <= $max_cols {
                        case.sends = $compact_sends;
                    }
                    case
                },
                live($interact)
            );
        }
    };
}

/// Resize matrix: representative PTY geometry is preserved via
/// [`Case::resize_variant`].
#[macro_export]
macro_rules! baseline_case_resize {
    ($mod_name:ident => $case:expr, $capture:expr) => {
        mod $mod_name {
            use super::*;
            $crate::canonical_combo_tests!(
                |cols, rows, color| ($case).resize_variant(cols, rows, color),
                resize($capture)
            );
        }
    };
}

/// Audit fixture matrix: one PTY per combo under `$prefix/<cols>x<rows>/<color>`.
#[macro_export]
macro_rules! audit_matrix_combo {
    ($mod_name:ident, $prefix:expr, $bin:expr, $args:expr, $needle:expr $(,)?) => {
        mod $mod_name {
            use super::*;
            $crate::canonical_combo_tests!(
                |cols, rows, color| Case::dynamic(
                    $crate::support::audit_default_name($prefix, cols, rows, color),
                    $bin,
                    $args,
                    cols,
                    rows,
                    color,
                    $needle,
                ),
                run_and_assert
            );
        }
    };
}
