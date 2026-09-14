# Campaign iteration guide — visual baseline performance

**ADDENDUM** to [Campaign execution prompt](campaign-execution-prompt.md). Stored **2026-09-15** alongside the frozen campaign catalog on `visual-baseline`.

This guide clarifies how executors iterate quickly during development without weakening the fail-closed visual oracle. It does **not** relax acceptance boundaries: the full matrix still gates task acceptance, application-chain closure, cross-application joins, and final closure (TASK-069).

Related docs: [Visual validation during refactoring](../../refactoring-tasks/visual-validation.md), [tuisnap coverage](../baseline/tuisnap-coverage.md), [nextest config](../../.config/nextest.toml).

---

## Suite shape (why filters work)

The visual baseline is `tests/visual_baseline/`: **7,550** per-combo `#[ignore]` PTY tests (one process per capture; nextest `pty` group, max 16 threads). Default `cargo nextest run` compiles the suite but runs only the cheap non-PTY [`store_integrity`](../../tests/visual_baseline/main.rs) check (~544 other unit/lib tests in the default inventory).

Canonical roots expand to **25 combos** each (5 terminal sizes × 5 colour modes). Combo tests are named:

```text
<app>::<capture_root>::c<cols>x<rows>_<color>
```

Example: `holla::holla_concept_first_use_default_120x40_truecolor::c120x40_truecolor`.

List names for a surface:

```sh
cargo nextest list --run-ignored only -E 'binary(visual_baseline) & test(showcase_)'
```

---

## Tiered gates (when to run what)

| Phase | Command | Scope | Typical wall time* |
| --- | --- | --- | --- |
| **Edit loop** (per task) | Targeted filter + `TUISNAP_FAST=1` | Affected app/module/scenario only | seconds–few minutes |
| **Ordinary regression** | `cargo nextest run` | ~544 non-ignored tests (no PTY captures) | ~1–3 min |
| **PR / CI smoke** | `cargo nextest run --profile ci --run-ignored only -E 'binary(visual_baseline)'` | ~302 captures (120×40 truecolor only) | **~1–2 min** |
| **Nightly / pre-acceptance** | `TUISNAP_FAST=1 cargo nextest run --run-ignored only -E 'binary(visual_baseline)'` | Full 7,550 combos; tiered gate skips PNG/HTML when `.ansi`+`.txt` match | **~30–45 min** |
| **Pre-release fidelity** | `cargo nextest run --run-ignored only -E 'binary(visual_baseline)'` | Full matrix; every combo renders PNG/HTML | **~45–60 min** (current tree); **~2 h** legacy pre-per-combo path |
| **Task acceptance / integration boundary** | Same as nightly (fast) or fidelity (final) | Full suite — **mandatory** | see above |

\*Measured on a 16-thread PTY pool (2026-09-15 benchmarks on `visual-baseline`). CI runners with fewer cores may take longer; smoke remains ≪ full matrix.

**Rule:** During an edit loop, never run all 7,550 captures. Run the **smallest filter** that covers the changed surface, then escalate through the tiers above at boundaries.

---

## 1. During development iteration (per task)

After a production edit that can affect rendering or interaction:

1. Run focused unit/lib tests for the touched crate or app (`cargo nextest run -E 'test(…)'` as named by the task contract).
2. Run a **targeted visual filter** for the affected app/family/scenario with fast mode:

```sh
TUISNAP_FAST=1 cargo nextest run --run-ignored only \
  -E 'binary(visual_baseline) & test(<filter>)'
```

Pick `<filter>` from the table below. Prefer a single capture root or one combo when debugging a specific regression.

Do **not** substitute a green targeted run for the mandatory full gate at task acceptance (see §4).

---

## 2. PR / CI smoke

PR CI uses the **`ci` nextest profile**, which sets `NEXTEST_PROFILE=ci` → smoke matrix (120×40 truecolor only) via [`support.rs`](../../tests/visual_baseline/support.rs). No `TUISNAP_FAST` required for smoke.

```sh
cargo nextest run --profile ci --run-ignored only -E 'binary(visual_baseline)'
```

~302 active captures, **~1–2 min** wall time. This is the default automated gate on pull requests; it is **not** sufficient for task acceptance or final closure.

---

## 3. Nightly / full gate

Full acceptance and closure boundaries require the complete matrix:

```sh
# Recommended nightly / task-acceptance (tiered gate — skips PNG/HTML when ansi+txt match)
TUISNAP_FAST=1 cargo nextest run --run-ignored only -E 'binary(visual_baseline)'

# Pre-release / maximum fidelity (full PNG/HTML every combo)
cargo nextest run --run-ignored only -E 'binary(visual_baseline)'
```

| Mode | Behaviour | Timing |
| --- | --- | --- |
| `TUISNAP_FAST=1` | 100 ms settle, 0 ms step pacing, tiered grouped gate (`full_render: false`) | **~30–45 min** |
| Default (legacy path) | 400 ms settle, 120 ms pacing, full PNG/HTML gate every combo | **~45–60 min** current tree; **~2 h** on pre-per-combo / pre-fast infrastructure |

Heavy flows (long `wait:` needles, e.g. `holla_flows_docker_done`) dominate single-combo time (~15–20 s); fast mode only trims settle/pacing/render overhead there.

Optional HTML index after bless/review (not the capture gate):

```sh
cargo nextest run --run-ignored only --ignore-default-filter -E 'test(rebuild_review_html)'
```

---

## 4. When the prompt says “mandatory full visual gate”

In [Campaign execution prompt](campaign-execution-prompt.md), **“mandatory full visual gate”** means:

```sh
cargo nextest run --run-ignored only -E 'binary(visual_baseline)'   # full matrix
# or, for acceptance speed with equivalent ansi/txt coverage:
TUISNAP_FAST=1 cargo nextest run --run-ignored only -E 'binary(visual_baseline)'
```

**Required at:**

- Task **acceptance** after canonical host verification (when the task can affect product output).
- **Failure/repair loop** step 8 — after a candidate revision, before a fresh independent verifier.
- **Parallel sibling integration** — union of impacted checks on the combined tree (not each sibling in isolation).
- Application-chain closures **039 / 050 / 057 / 064**.
- Cross-application joins and **final closure** (TASK-069).
- **Definition of done** — zero unintended visual differences on the final tree.

**Not required on every edit loop.** The prompt already says “run targeted feedback first if useful”; this guide makes that explicit: implementers should use §1 filters while iterating, then run the full gate once per acceptance boundary.

Receipts must record which tier ran (smoke / fast full / fidelity full) and the exact command.

---

## 5. Example nextest filters

| Intent | Example `-E` expression |
| --- | --- |
| Whole app | `binary(visual_baseline) & test(showcase_)` |
| Holla flows family | `binary(visual_baseline) & test(holla_flows_)` |
| One capture root (25 combos) | `binary(visual_baseline) & test(holla_concept_first_use)` |
| Single combo | `binary(visual_baseline) & test(holla_concept_first_use_default_120x40_truecolor::c120x40_truecolor)` |
| Smoke slice of one app (CI profile) | `cargo nextest run --profile ci --run-ignored only -E 'binary(visual_baseline) & test(holla_)'` |
| Store inventory (non-PTY, always cheap) | `cargo nextest run -E 'test(store_integrity)'` |
| Pointer / mouse group | `binary(visual_baseline) & test(pointer_)` |
| Audit fixtures | `binary(visual_baseline) & test(audit_)` |

Combine with `\|` for a small set:

```sh
TUISNAP_FAST=1 cargo nextest run --run-ignored only \
  -E 'binary(visual_baseline) & (test(holla_concept_first_use) | test(holla_concept_docker_cleanup))'
```

---

## 6. Ordinary `cargo nextest run` scope and timing

Default profile ([`.config/nextest.toml`](../../.config/nextest.toml)):

- Runs **~544** tests across the workspace **excluding** ignored visual captures and `rebuild_review_html`.
- Includes `store_integrity` (validates committed `snapshots/` inventory vs suite).
- Does **not** spawn PTY captures.
- Typical wall time: **~1–3 minutes** (compile excluded).

Use after production changes unless canonical host verification already proves an equivalent complete inventory (per prompt § Functional non-regression). This is the fast “did I break unit tests?” gate; it is not a substitute for visual regression on rendering tasks.

---

## 7. Environment variables

| Variable | When set | Effect |
| --- | --- | --- |
| **`TUISNAP_FAST=1`** | Local iteration, nightly full gate, task acceptance (recommended) | 100 ms settle; 0 ms inter-step pacing; tiered grouped gate — skips PNG/HTML render when `.ansi` and `.txt` both match ([`GroupedCheckOptions { full_render: false }`](../../tests/visual_baseline/support.rs)). |
| **`TUISNAP_MATRIX=smoke`** | PR CI or manual smoke | Restricts matrix expansion to **120×40 truecolor** only. Equivalent to `--profile ci` (which sets `NEXTEST_PROFILE=ci` → same hook). |
| **`TUISNAP_BLESS=1`** | Host bless/regeneration workflows only | Matrix runners continue after combo failure so all actuals are written before panicking. **Candidates must never set this**; executors must never bless. |
| **`NEXTEST_PROFILE=ci`** | Set by `--profile ci` | Enables smoke matrix via `smoke_matrix()` in support.rs. |

Blessing approved snapshots remains **host-only**:

```sh
ln -sfn target/tuisnap/actual snapshots.actual
tuisnap accept --grouped --store snapshots --name <group/…/size/color>
```

See [visual-validation.md](../../refactoring-tasks/visual-validation.md).

---

## Prompt amendments (coordinator review)

Exact locations in [campaign-execution-prompt.md](campaign-execution-prompt.md) that should reference this guide:

| Location | Current text (summary) | Recommended amendment |
| --- | --- | --- |
| **`# Visual regression is a hard fail-closed gate`** — paragraph before the mandatory full visual gate block (~L519–525) | “run targeted feedback first if useful, then the mandatory full visual gate” with only the full-suite command | Add: “See [Campaign iteration guide](campaign-iteration-guide.md). During edit loops use targeted filters + `TUISNAP_FAST=1` (§1); run the mandatory full gate only at acceptance/integration boundaries (§4).” Split the code block into **iteration** vs **acceptance** commands. |
| Same section — after “Also retain ordinary regression coverage” (~L527–532) | `cargo nextest run` only | Add note that default nextest excludes PTY captures (~544 tests, ~1–3 min); link §6. |
| Same section — closure bullets (~L548–554) | “complete … visual closure” / “rerun the complete visual suite” | Clarify: full matrix with `TUISNAP_FAST=1` minimum; fidelity (no fast) before TASK-069 / merge readiness. |
| **`# Parallelism`** — post-sibling integration (~L211) | “applicable visual regression checks” | Replace with: “visual checks per [iteration guide](campaign-iteration-guide.md) §4 for the combined impacted surface; full matrix if any sibling touched cross-app rendering.” |
| **`# Failure / repair loop`** — step 8 (~L341) | “rerun the applicable functional and visual gates” | Cross-link §4: targeted visual while repairing; full gate before re-freeze and fresh verifier. |
| **`# Mandatory per-task agent protocol` → IMPLEMENTER** (~L273) | “ordinary local tests for feedback” | Add bullet: may run targeted `TUISNAP_FAST=1` visual filters; must not claim acceptance from them. |
| **`# Definition of done`** — visual bullet (~L802) | “visual baseline has zero unintended differences” | Footnote: proven by full-suite command in §3/§4, not smoke alone. |
| **`# Final report`** — visual result line (~L838) | “full visual-baseline result” | Require reporting exact command, env (`TUISNAP_FAST`, profile), and tier (smoke / fast full / fidelity). |

**Header cross-link (applied):** the execution prompt’s visual-regression section should link here at first mention (see edit in `campaign-execution-prompt.md`).
