//! Deterministic data used by the Showcase pages.
//!
//! The data is deliberately app-owned. The component crate receives only
//! borrowed rows and never learns that these records describe tasks, files,
//! or a code editor.

use tui_next::{ItemKey, TreeNode};

/// A task row used by table and grid demonstrations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TaskRow {
    /// Stable identity.
    pub id: u32,
    /// Human-readable task title.
    pub name: &'static str,
    /// Assignee handle.
    pub owner: &'static str,
    /// Current state.
    pub status: TaskStatus,
    /// Branch name.
    pub branch: &'static str,
    /// Number of changed lines.
    pub changes: u32,
    /// Simulated duration in seconds.
    pub duration_s: u32,
}

/// A deterministic task state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TaskStatus {
    /// Waiting to start.
    Queued,
    /// Currently running.
    Running,
    /// Completed successfully.
    Done,
    /// Failed.
    Failed,
    /// Paused by the user.
    Paused,
}

/// The full 24-row task fixture from the legacy Showcase.
pub(crate) const TASKS: &[TaskRow] = &[
    TaskRow { id: 1040, name: "Add rate limiting to auth endpoints", owner: "mira", status: TaskStatus::Done, branch: "feat/rate-limit", changes: 14, duration_s: 412 },
    TaskRow { id: 1041, name: "Migrate sessions table to UUID keys", owner: "jonas", status: TaskStatus::Running, branch: "chore/uuid-sessions", changes: 31, duration_s: 96 },
    TaskRow { id: 1042, name: "Fix flaky checkout integration test", owner: "ana", status: TaskStatus::Failed, branch: "fix/checkout-flake", changes: 3, duration_s: 58 },
    TaskRow { id: 1043, name: "Write release notes for 3.2", owner: "mira", status: TaskStatus::Queued, branch: "docs/release-3.2", changes: 0, duration_s: 0 },
    TaskRow { id: 1044, name: "Replace deprecated Vue mixins", owner: "kai", status: TaskStatus::Done, branch: "refactor/mixins", changes: 87, duration_s: 1330 },
    TaskRow { id: 1045, name: "Upgrade Postgres driver to 0.9", owner: "jonas", status: TaskStatus::Paused, branch: "chore/pg-driver", changes: 5, duration_s: 240 },
    TaskRow { id: 1046, name: "Extract billing service module", owner: "sofia", status: TaskStatus::Done, branch: "refactor/billing", changes: 52, duration_s: 908 },
    TaskRow { id: 1047, name: "Add OpenTelemetry tracing spans", owner: "kai", status: TaskStatus::Running, branch: "feat/otel", changes: 22, duration_s: 130 },
    TaskRow { id: 1048, name: "Remove legacy feature flags", owner: "ana", status: TaskStatus::Queued, branch: "chore/flags", changes: 0, duration_s: 0 },
    TaskRow { id: 1049, name: "Generate API client from OpenAPI", owner: "sofia", status: TaskStatus::Done, branch: "feat/api-client", changes: 118, duration_s: 2210 },
    TaskRow { id: 1050, name: "Harden CSP headers", owner: "mira", status: TaskStatus::Done, branch: "sec/csp", changes: 4, duration_s: 77 },
    TaskRow { id: 1051, name: "Speed up cold start of worker", owner: "jonas", status: TaskStatus::Failed, branch: "perf/worker-boot", changes: 9, duration_s: 601 },
    TaskRow { id: 1052, name: "Localize onboarding emails", owner: "kai", status: TaskStatus::Queued, branch: "feat/i18n-emails", changes: 0, duration_s: 0 },
    TaskRow { id: 1053, name: "Add pagination to audit log", owner: "ana", status: TaskStatus::Done, branch: "feat/audit-pages", changes: 16, duration_s: 344 },
    TaskRow { id: 1054, name: "Refactor retry helper into crate", owner: "sofia", status: TaskStatus::Running, branch: "refactor/retry", changes: 11, duration_s: 45 },
    TaskRow { id: 1055, name: "Rotate signing keys quarterly", owner: "mira", status: TaskStatus::Queued, branch: "sec/key-rotation", changes: 0, duration_s: 0 },
    TaskRow { id: 1056, name: "Fix timezone bug in scheduler", owner: "jonas", status: TaskStatus::Done, branch: "fix/tz-scheduler", changes: 7, duration_s: 188 },
    TaskRow { id: 1057, name: "Document webhook retry semantics", owner: "kai", status: TaskStatus::Done, branch: "docs/webhooks", changes: 2, duration_s: 65 },
    TaskRow { id: 1058, name: "Add dark mode to admin panel", owner: "ana", status: TaskStatus::Paused, branch: "feat/admin-dark", changes: 40, duration_s: 720 },
    TaskRow { id: 1059, name: "Bump minimum Node to 22", owner: "sofia", status: TaskStatus::Queued, branch: "chore/node-22", changes: 0, duration_s: 0 },
    TaskRow { id: 1060, name: "Cache dependency graph between runs", owner: "mira", status: TaskStatus::Running, branch: "perf/dep-cache", changes: 19, duration_s: 210 },
    TaskRow { id: 1061, name: "Clean up unused SQL views", owner: "jonas", status: TaskStatus::Done, branch: "chore/sql-views", changes: 12, duration_s: 155 },
    TaskRow { id: 1062, name: "Add health endpoint for gateway", owner: "kai", status: TaskStatus::Done, branch: "feat/health", changes: 3, duration_s: 42 },
    TaskRow { id: 1063, name: "Investigate memory growth in parser", owner: "ana", status: TaskStatus::Running, branch: "perf/parser-mem", changes: 6, duration_s: 380 },
];

/// The complete language fixture from the original list/picker page.
pub(crate) const LANGUAGES: &[&str] = &[
    "Rust", "TypeScript", "Python", "Kotlin", "Go", "Java", "Swift", "C#", "Ruby", "Scala",
    "Elixir", "Haskell", "Zig", "Dart", "PHP", "C++", "Lua", "OCaml", "Clojure", "Erlang",
];

/// Stable pre-order tree fixture. Labels are resolved by key in the page;
/// the public tree component owns only depth and expansion state.
pub(crate) const TREE: &[TreeNode] = &[
    TreeNode::parent(0).keyed(ItemKey::num(1)),
    TreeNode::parent(1).keyed(ItemKey::num(2)),
    TreeNode::leaf(2).keyed(ItemKey::num(3)),
    TreeNode::leaf(2).keyed(ItemKey::num(4)),
    TreeNode::leaf(2).keyed(ItemKey::num(5)),
    TreeNode::parent(1).keyed(ItemKey::num(6)),
    TreeNode::leaf(2).keyed(ItemKey::num(7)),
    TreeNode::leaf(2).keyed(ItemKey::num(8)),
    TreeNode::leaf(0).keyed(ItemKey::num(9)),
    TreeNode::leaf(0).keyed(ItemKey::num(10)),
];

/// A tree node label.
pub(crate) fn tree_label(node: &TreeNode) -> &'static str {
    match node.key() {
        Some(ItemKey::Num(1)) => "src",
        Some(ItemKey::Num(2)) => "api",
        Some(ItemKey::Num(3)) => "auth.rs",
        Some(ItemKey::Num(4)) => "billing.rs",
        Some(ItemKey::Num(5)) => "config.rs",
        Some(ItemKey::Num(6)) => "webhooks",
        Some(ItemKey::Num(7)) => "dispatch.rs",
        Some(ItemKey::Num(8)) => "retry.rs",
        Some(ItemKey::Num(9)) => "Cargo.toml",
        Some(ItemKey::Num(10)) => "README.md",
        _ => "item",
    }
}

/// The short code fixture retained by the original package contract.
pub(crate) const CODE: &str = "fn main() {\n    println!(\"hello from showcase\");\n}\n";

/// Deterministic code for the editor page.
pub(crate) const EDITOR_CODE: &str = "fn retry_with_backoff(attempt: u32) -> Result<Response, Error> {\n    let delay = BASE_DELAY * 2_u64.pow(attempt);\n    tracing::debug!(attempt, ?delay, \"retrying request\");\n    client.send().or_else(|error| {\n        if attempt < MAX_ATTEMPTS {\n            sleep(delay);\n            retry_with_backoff(attempt + 1)\n        } else {\n            Err(error)\n        }\n    })\n}";

/// The twenty-eight-line runbook used by the text-area page.
pub(crate) const TEXTAREA_CONTENT: &str = "1. Read the task description\n2. Inspect the current implementation\n3. Write down the smallest safe change\n4. Run the focused tests\n5. Check the public facade boundary\n6. Compare the rendered frame\n7. Review keyboard navigation\n8. Review mouse navigation\n9. Check the empty state\n10. Check the loading state\n11. Check the error state\n12. Check the disabled state\n13. Check the narrow terminal\n14. Check the wide terminal\n15. Run formatting\n16. Run clippy\n17. Run integration tests\n18. Inspect diagnostics\n19. Inspect allocations\n20. Inspect the diff\n21. Confirm no skips\n22. Confirm stable keys\n23. Confirm focus restore\n24. Confirm modal dismissal\n25. Confirm color downgrade\n26. Capture the reviewed frame\n27. Record the baseline change\n28. Run the final gate";

/// Long prose for the scrolling viewport.
pub(crate) const PROSE: &str = "Junie works through a task the way a careful engineer would: it reads the relevant code, forms a plan, makes focused changes, runs the tests, and reports back with a summary you can review before anything is merged.\n\nEach step is visible. You can pause, redirect, or take over at any point, and every change lands as an ordinary diff in your working tree.\n\nThe design system in this prototype exists so that the terminal version of that experience feels as deliberate as the web version: quiet surfaces, one accent, clear focus, and no decoration that does not carry information.\n\nScroll with the mouse wheel, PageUp/PageDown, or the arrow keys while this panel has focus. The scrollbar on the right shows where you are and how much remains.";

/// Deterministic terminal log lines.
pub(crate) fn log_lines(count: usize) -> Vec<String> {
    let steps = [
        ("info", "Resolving workspace members"),
        ("info", "Fetching crates.io index"),
        ("info", "Compiling proc-macro2 v1.0.86"),
        ("info", "Compiling serde v1.0.210"),
        ("warn", "unused import: `std::fmt` in src/api/mod.rs:3"),
        ("info", "Compiling tokio v1.40.0"),
        ("info", "Running unittests src/lib.rs"),
        ("info", "test api::auth::tests::rejects_expired ... ok"),
        ("info", "test db::pool::tests::reuses_connections ... ok"),
        ("error", "test checkout::places_order ... FAILED"),
        ("info", "test workers::scheduler::tests::respects_timezone ... ok"),
        ("info", "Linking target/debug/deps/app-4f2c1b"),
    ];
    (0..count)
        .map(|i| {
            let (level, message) = steps[i % steps.len()];
            format!("{:>7.2}s  {level:<5}  {message}", i as f64 * 0.37)
        })
        .collect()
}
