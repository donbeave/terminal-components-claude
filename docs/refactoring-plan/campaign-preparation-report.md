# Campaign preparation report

**Date:** 2026-09-15  
**Branch:** `visual-baseline` @ `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`  
**Purpose:** Evidence-based readiness assessment for starting the refactoring campaign. **Preparation only — not execution.**

---

## 1. Stored artifacts

| Artifact | Path | Git status |
| --- | --- | --- |
| Campaign execution prompt | [`docs/refactoring-plan/campaign-execution-prompt.md`](campaign-execution-prompt.md) | **Untracked** (written 2026-09-15) |
| Campaign iteration guide | [`docs/refactoring-plan/campaign-iteration-guide.md`](campaign-iteration-guide.md) | **Untracked** (written 2026-09-15) |
| Related executor protocol | [`docs/refactoring-plan/campaign-executor-protocol.md`](campaign-executor-protocol.md) | Tracked |
| Visual validation tiers (task AGENTS reference) | [`refactoring-tasks/visual-validation.md`](../../refactoring-tasks/visual-validation.md) | **Modified, uncommitted** |

The execution prompt is the frozen `/goal` body for campaign arming. The iteration guide is an addendum covering tiered visual gates, filters, and timing. Both must be committed alongside perf infrastructure before campaign start.

---

## 2. Git identities verified (Phase 0 audit)

Verified locally on 2026-09-15 (`git rev-parse`, `git show-ref`, `git ls-remote`).

| Authority | Expected SHA | Verified | Notes |
| --- | --- | --- | --- |
| **HEAD / `visual-baseline` branch tip** | `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` | **Match** | Current checkout |
| **Annotated tag `visual-baseline`** | peels to `4a79c0a2…` | **Match** | Tag object `1ee5ebdc…` → commit `4a79c0a2…`; `origin/visual-baseline` agrees |
| **Architectural `main` starting point** | `7b27732a8c3c131760ec3438f641cb3c11343a42` | **Exists** | Object present locally; production worktrees must start here |
| **Immutable product oracle (peeled freeze)** | `02f5294bfdbf38004cc49130d0aff1d01f31434c` | **Exists** | Distinct from branch tip — correct separation |
| **task-format authority** | `52d9f1eb7721f409bc47beb9fced7997b5c13ede` | **Match** | Installed `taskfmt 0.2.0` and `git ls-remote …/task-format.git refs/heads/main` both report this SHA |

**Stale prose warning:** [`PROGRESS.md`](PROGRESS.md) still records an older live-tag identity (`5e533943`). Current git refs supersede that line. Do not treat stale planning prose as authoritative over `git rev-parse`.

**Remote tag note:** `git ls-remote origin refs/tags/visual-baseline` returns tag object `1ee5ebdc…` (annotated), which peels to commit `4a79c0a2…` — consistent with branch tip. Do not move, delete, or retarget this tag per repository rules.

---

## 3. Phase 0 status: NOT READY

**Verdict:** `NOT READY FOR REFACTORING EXECUTION`

Source: [`PROGRESS.md`](PROGRESS.md) (“execution readiness reopened”), [`reaudit-plan.md`](reaudit-plan.md) (“completion unproven”), [`planning-acceptance.md`](planning-acceptance.md) (“reopened; previous pass is historical only”), and live validator runs below.

### Top blockers (ranked)

| # | Blocker | Evidence |
| --- | --- | --- |
| 1 | **Re-audit incomplete — 49 findings, 0 fully closed** | [`reaudit-findings.tsv`](reaudit-findings.tsv): every row has non-empty `remaining`; 7 rows have `independent_evidence = pending` |
| 2 | **Cross-artifact integrity gate fails** | `validate-plan.py --summary`: `passed: false`, **150 errors** (25 unique themes); parity join drift, AGENTS protocol drift TASK-001–020, index scope drift |
| 3 | **Task graph / assembly broken** | `derive-task-graph.py`: `Stored machine graph is stale`; `assemble-plan.py`: `No authored owner for ('DEC', 'ADJ-14')` |
| 4 | **Whole-branch diff integration incomplete** | `BRANCH-01`: 7,885 changed paths inventoried; Jackin/TablePro partition and 7,019 artifact dispositions not integrated into task graph |
| 5 | **Bootstrap assets not refrozen** | `ROOT-12`: 151-row snapshot stale; +32 broker assets pending independent review and freeze |
| 6 | **Production proof harness absent** | `/proof/bin/tc-proof*` and `tc-proof-host` do not exist; all 71 production `verify.toml` gates are non-runnable |
| 7 | ~~External tui-snap dependency open~~ | **Resolved** — [PR #1](https://github.com/donbeave/tui-snap/pull/1) merged; campaign pin `0a2e490…` in `Cargo.toml` |
| 8 | **Unresolved authority decisions** | `ROOT-08` (F03/F06/F08c/F17 open), `ROOT-09` (Showcase Settings crash — needs user direction on narrow exception) |
| 9 | **Perf/doc prep uncommitted** | 7 modified files + 2 untracked campaign docs; `Cargo.toml` uses local `path = "../tui-snap"` — not campaign-safe until pushed and pinned |
| 10 | **Smoke gate currently red** | `tablepro_connections_form_advanced_120x40_truecolor` fails under `--profile ci` (observed during verification audit) |

### Live Phase 0 validator snapshot (2026-09-15)

| Validator | Result |
| --- | --- |
| `taskfmt project lint terminal-components` | **PASS** — 73/73 packages, 0 errors |
| `validate-plan.py --summary` | **FAIL** — 150 errors |
| `derive-task-graph.py` | **FAIL** — stale machine graph |
| `assemble-plan.py` | **FAIL** — missing ADJ-14 owner |
| `proof-comparator-bootstrap.py --self-test` | **PASS** — 72 cases, 888 schema rejections |

---

## 4. Verification command audit

Measured on `visual-baseline`, 2026-09-15, warm build cache, 16-thread PTY pool (`pty.max-threads = 16` in uncommitted `.config/nextest.toml`). Estimates marked **est.** where not run end-to-end.

| Category | Command | Typical duration | Blocker risk | Fast alternative |
| --- | --- | --- | --- | --- |
| Default nextest | `cargo nextest run` | **~11 s** measured (545 run, 7,551 skipped) | Low | Crate/filter `-E 'test(…)'` |
| store_integrity | `cargo nextest run -E 'test(store_integrity)'` | **~4 s** wall / **~0.4 s** test | Low | Included in default nextest |
| Visual — edit loop | `TUISNAP_FAST=1 cargo nextest run --run-ignored only -E 'binary(visual_baseline) & test(<filter>)'` | **Seconds–few min** | Low (not acceptance) | Single combo or one capture root (25 combos) |
| Visual — CI smoke | `cargo nextest run --profile ci --run-ignored only -E 'binary(visual_baseline)'` | **~85 s** measured (~302 active captures) | Medium (PR gate; 1 fail today) | Per-app: `… & test(holla_)` |
| Visual — acceptance (fast full) | `TUISNAP_FAST=1 cargo nextest run --run-ignored only -E 'binary(visual_baseline)'` | **~30–45 min est.** (7,550 combos) | High at boundaries only | Required at task acceptance / integration — not every edit |
| Visual — fidelity full | `cargo nextest run --run-ignored only -E 'binary(visual_baseline)'` | **~45–60 min est.**; legacy pre-perf **~2 h** | High at TASK-069 only | Run once before merge readiness |
| taskfmt project lint (Phase 0) | `taskfmt --config …/experiment.toml project lint …` | **~0.3 s** | Low | Per-package lint |
| validate-plan.py (Phase 0) | `python3 …/validate-plan.py --summary` | **~0.4 s** | **High — currently FAIL** | Fix asset/sync drift first |
| Comparator bootstrap (TASK-001 prep) | `python3 -B -O …/proof-comparator-bootstrap.py --self-test` | **~1.1 s** | Low | — |
| Host bootstrap self-test | `python3 -B -O …/host-bootstrap-driver.py --self-test` | **~5.8 s** | Medium | `--observer-test` for real gates |
| Runner bootstrap self-test (TASK-070) | `python3 -B -O …/runner-bootstrap-driver.py --self-test` | **~42 s** | Medium | Milestone-only during harness dev |
| taskfmt standalone verify (campaign) | `taskfmt verify --root … --task-dir …/NNN --base … --progress …` | **Est. 10 s–minutes+** per task | **Critical — harness not built** | Advisory dev runs only |
| tc-proof-host verify (campaign) | `tc-proof-host verify --run …` | **Est. minutes–hours** | **Critical — not implemented** | None until TASK-001/070 accepted |
| Per-task verify.toml (production) | 7-check pattern incl. `account-tests` | **Est. 10 min–hours+** per task | **Critical** once harness exists | Scoped nextest + targeted visual during dev |
| rebuild_review_html | `cargo nextest run --run-ignored only --ignore-default-filter -E 'test(rebuild_review_html)'` | **Heavy est.** | Low for dev | Publish step only |

**Iteration guidance is no longer a prompt blocker** after amendments (§6). Full visual fidelity at every edit would still dominate agent turns; the prompt now directs edit-loop filters and reserves full matrix for acceptance boundaries.

---

## 5. Visual baseline performance (uncommitted perf work)

All changes below are **modified in working tree, not committed** (2026-09-15 perf goal session).

### Files touched

| File | Change |
| --- | --- |
| `tests/visual_baseline/support.rs` | Per-combo test macros, `TUISNAP_FAST=1`, smoke matrix, tiered gate hooks |
| `tests/visual_baseline/holla.rs`, `pointer.rs`, `audit.rs` | Per-combo split |
| `.config/nextest.toml` | `pty.max-threads = 16`, `[profile.ci]` smoke matrix |
| `Cargo.toml` | `tuisnap = { git = "…/tui-snap", rev = "0a2e490…" }` — **pinned** |
| `Cargo.lock` | Lockfile drift from path dep |

Companion **tui-snap** changes (+147/−32 at `~/Projects/tui-snap`): tiered `GroupedStore::check_with`, `PtyOptions::input_pace`, `run_once` settled-frame return, PNG fast path. **Not pushed; not pinned in campaign catalog.**

### Before / after (measured 2026-09-15)

| Scenario | Before (legacy) | After (uncommitted) | Speedup |
| --- | --- | --- | --- |
| Test inventory | 302 tests × 25 serial PTY each | **7,550** per-combo tests (1 PTY each) | Better parallelism |
| `showcase_pages_overview` (25 combos) | **~96 s** | **~4 s** (`TUISNAP_FAST=1`) | **~25×** |
| `holla_concept_first_use` (25 combos) | **~120 s** | **~4 s** fast / **~12 s** default | **~30× / ~10×** |
| **Full suite (extrapolated)** | **~2 h** | **~30–45 min** (fast) / **~45–60 min** (fidelity) | **~3–4×** |
| **PR CI smoke** (`--profile ci`) | ~15–20 min est. | **~78–85 s** measured | **~12–15×** |
| `docker_done` (single combo) | ~20 s | ~18 s | ~1.1× (simulation-bound) |

### Recommendation

**Commit perf changes on `visual-baseline` before campaign execution**, in this order:

1. Push tui-snap, bump `Cargo.toml` to pinned git rev (not `path = …`).
2. Commit infra + docs together: `support.rs`, nextest config, `campaign-execution-prompt.md`, `campaign-iteration-guide.md`, `visual-validation.md`.
3. Verify green smoke: `cargo nextest run --profile ci --run-ignored only -E 'binary(visual_baseline)'` (fix `tablepro_connections_form_advanced` failure first).
4. Record new baseline timings in campaign receipts template.

Campaign TASK-001+ must not run against a tree where visual gate timing is undefined or depends on uncommitted local path deps.

---

## 6. Prompt amendments applied

Coordinator-approved amendments from [`campaign-iteration-guide.md` §Prompt amendments](campaign-iteration-guide.md) were applied by subagent on 2026-09-15 (session `606f3d63`).

| # | Location | Status |
| --- | --- | --- |
| 1 | Visual regression — iteration vs acceptance split + link | **Applied** |
| 2 | Visual regression — default nextest scope note (~544 tests, §6) | **Applied** |
| 3 | Closure bullets — `TUISNAP_FAST=1` minimum; fidelity before TASK-069 | **Applied** |
| 4 | Parallelism — post-sibling integration (§4 cross-link) | **Applied** |
| 5 | Failure/repair loop step 8 — targeted vs full gate | **Applied** |
| 6 | IMPLEMENTER protocol — targeted `TUISNAP_FAST=1` allowed, not acceptance | **Applied** |
| 7 | Definition of done — visual bullet footnote (§3/§4) | **Applied** |
| 8 | Final report — command/env/tier required | **Applied** |
| — | `refactoring-tasks/visual-validation.md` — smoke/ci/fast tiers | **Applied** (modified, uncommitted) |

**Pending doc sync (non-blocking for prompt, blocking for catalog freeze):** `docs/holla-fable-comparison.md` still says “303 PTY captures”; task-package `AGENTS.md` templates may still mandate unconditional full visual gate — reconcile when refreezing bootstrap assets.

---

## 7. Pre-campaign checklist (before TASK-001)

### A. Close Phase 0 planning (required)

- [ ] Resolve all 49 rows in [`reaudit-findings.tsv`](reaudit-findings.tsv) with independent rereview evidence
- [ ] Complete whole-branch diff integration (`BRANCH-01`) including Jackin/TablePro partition
- [ ] Fix and pass `validate-plan.py --summary` (zero errors)
- [ ] Regenerate and verify `derive-task-graph.py` / `assemble-plan.py` (including ADJ-14 owner)
- [ ] Refreeze bootstrap assets (`freeze-bootstrap-assets.py --check` all groups)
- [ ] Rerun qualification harnesses under new compiler-identity and broker contracts
- [ ] Obtain user/authority decisions on `ROOT-08` / `ROOT-09`
- [ ] Fresh independent reviewers issue **`READY FOR REFACTORING EXECUTION`** against frozen catalog bytes
- [ ] Re-prove all 20 items in [`planning-acceptance.md`](planning-acceptance.md) on current artifacts

### B. Commit preparation artifacts (required before arming `/goal`)

- [ ] Commit `campaign-execution-prompt.md` and `campaign-iteration-guide.md`
- [ ] Commit visual perf infrastructure + `visual-validation.md` updates
- [x] Push tui-snap; pin git rev in `Cargo.toml` (`0a2e490802b7b048cd96349c6af860f8a3a05c3d`)
- [ ] Fix CI smoke failure (`tablepro_connections_form_advanced_120x40_truecolor`)
- [ ] Regenerate planning artifact manifest / hash after commits

### C. Execution prerequisites (TASK-001 bootstrap — not Phase 0, but must be planned)

- [ ] Build and qualify `/proof/bin/tc-proof-host` and bootstrap comparators per TASK-001/070 contracts
- [x] Merge or formally pin tui-snap PR #1 — merged; pin `0a2e490802b7b048cd96349c6af860f8a3a05c3d`
- [ ] Create architectural-main worktree from `7b27732a…` (never from `visual-baseline` for production edits)

---

## 8. Recommended next step for Alexey

**Do not arm the campaign `/goal` or dispatch TASK-001 yet.**

Recommended sequence:

1. **Commit perf + campaign docs first** (short, independent win) — unblocks fast iteration during Phase 0 repair work and removes ambiguity about visual gate timing. Fix the smoke failure and tui-snap pin in the same commit series.

2. **Continue closing Phase 0** — prioritize validator repair (`validate-plan.py`, graph stale, ADJ-14 owner) and the 49-finding register over starting production work. Phase 0 est. **3–6 weeks** focused planning with parallel review.

3. **User decision needed:** `ROOT-09` Showcase Settings crash after env deletion — narrow oracle exception vs full preservation. Blocks final adjudication sync.

4. **After Phase 0 green + commits:** arm campaign from committed `campaign-execution-prompt.md` copy; coordinator runs Phase 0 gate checklist again, then TASK-001 bootstrap only.

---

## Appendix: current git working tree (2026-09-15)

```
 M .config/nextest.toml
 M Cargo.lock
 M Cargo.toml
 M refactoring-tasks/visual-validation.md
 M tests/visual_baseline/audit.rs
 M tests/visual_baseline/holla.rs
 M tests/visual_baseline/pointer.rs
 M tests/visual_baseline/support.rs
?? docs/refactoring-plan/campaign-execution-prompt.md
?? docs/refactoring-plan/campaign-iteration-guide.md
?? docs/refactoring-plan/campaign-preparation-report.md   ← this file
```

**Honest status:** Campaign **documentation and perf infrastructure are drafted but not committed**. Phase 0 **planning validators mostly fail**. Prompt amendments **are applied in working-tree copies**. Execution **is not ready**.

### Follow-up (2026-09-15, committed)

| Action | Status |
| --- | --- |
| Prep bundle + Phase 0 validators + snapshots | **Committed** — `32c238e1`, `c5384043`, `f395f284` on `visual-baseline` |
| tui-snap tiered-gate | **Pushed** — `0a2e490`; pinned in `Cargo.toml` |
| CI smoke (`--profile ci`) | **PASS** — 7550 passed, ~56 s wall |
| `validate-plan.py --summary` | **PASS** — 0 errors |
| Re-audit findings closed | **5 / 49** (ROOT-02, ROOT-03, CA-06, ROOT-11, ROOT-12) |
| Phase 0 execution readiness | **Still NOT READY** — 20 findings open (29 closed); BRANCH-01 + independent witness round pending; ROOT-08/09 closed |
| CI smoke + store_integrity | **PASS** — 7550/7550; 9×120x40 fidelity snapshots re-blessed |
| validate-plan.py | **PASS** — 0 errors after F03/F06/F08c/F17 sync |
