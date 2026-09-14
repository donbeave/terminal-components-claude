# Campaign preparation report

**Date:** 2026-09-15 (refreshed after subagent audit)  
**Branch:** `visual-baseline` @ `623a1a5985ad08baa99dfa397500927a35db2074`  
**Frozen tag:** `visual-baseline` → `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` (unmoved; branch is **5 commits ahead**)  
**Purpose:** Evidence-based readiness assessment for arming the campaign `/goal`. **Preparation only — not execution.**

---

## 1. Stored artifacts

| Artifact | Path | Status |
| --- | --- | --- |
| Campaign execution prompt | [`campaign-execution-prompt.md`](campaign-execution-prompt.md) | **Stored** — committed base `32c238e1`; local edits (iteration cross-refs, header) |
| Campaign iteration guide | [`campaign-iteration-guide.md`](campaign-iteration-guide.md) | **Stored** — tiered gates, filters, measured timings |
| Preparation report | This file | Updated 2026-09-15 |
| Executor protocol | [`campaign-executor-protocol.md`](campaign-executor-protocol.md) | Tracked |
| Visual validation tiers | [`refactoring-tasks/visual-validation.md`](../../refactoring-tasks/visual-validation.md) | Updated locally |

Copy the `/goal` body from `campaign-execution-prompt.md` verbatim when arming. Read `campaign-iteration-guide.md` before any production edit loop.

---

## 2. Git identities verified

Re-verified 2026-09-15 (git rev-parse + remote ls-remote).

| Authority | Expected SHA | Verified | Notes |
| --- | --- | --- | --- |
| **Architectural `main`** | `7b27732a8c3c131760ec3438f641cb3c11343a42` | **Match** | Local + `origin/main` |
| **Tag `visual-baseline` (peeled)** | `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` | **Match** | Annotated tag object `1ee5ebdc…`; **not retargeted** |
| **Branch `visual-baseline` tip** | (ahead of tag) | `623a1a59` | 5 commits post-freeze campaign prep — catalog bytes to freeze at arm time |
| **Product oracle** | `02f5294bfdbf38004cc49130d0aff1d01f31434c` | **Match** | Distinct from branch tip — correct |
| **task-format authority** | `52d9f1eb7721f409bc47beb9fced7997b5c13ede` | **Match** | Remote + installed `taskfmt 0.2.0` |
| **tui-snap campaign pin** | `0a2e490802b7b048cd96349c6af860f8a3a05c3d` | **Match** | `Cargo.toml` git rev |

**Stale prose:** [`PROGRESS.md`](PROGRESS.md) may cite older tag/branch SHAs. **`git rev-parse` wins.**

---

## 3. Phase 0 status: NOT READY

**Verdict:** `NOT READY FOR REFACTORING EXECUTION`

Last audited: 2026-09-15 (subagent round + local verification).

### Mechanical gates (green)

| Validator | Result |
| --- | --- |
| `validate-plan.py --summary` | **PASS** — 0 errors |
| `assemble-plan.py` | **PASS** — idempotent |
| `derive-task-graph.py` | **PASS** — 73 tasks, depth 35 |
| `taskfmt project lint terminal-components` | **PASS** — 73/73 |
| `freeze-bootstrap-assets.py --check` (6 groups) | **PASS** — 202 assets |
| `proof-comparator-bootstrap.py --self-test` | **PASS** — prep only; `"qualified_production_harness": false` |
| `cargo nextest run -E 'test(store_integrity)'` | **PASS** |

### Process gates (open)

| Gate | Status |
| --- | --- |
| Re-audit register | **15 open / 34 closed** (49 total; 5 closed this session) |
| Independent `READY FOR REFACTORING EXECUTION` | **Not issued** |
| `planning-acceptance.md` §22 | **10 proven / 10 unproven** |
| Production harness (`tc-proof-host`, `tools/refactor-proof/`) | **Not built** — TASK-001 deliverable |
| BRANCH-01 whole-branch diff | **Incomplete** — 7,885-path census not fully in task graph |
| Campaign doc freeze | **Uncommitted** local edits to prompt/iteration/prep/visual-validation |

### Top remaining blockers

1. **15 open re-audit findings** — especially BRANCH-01, HIST-01, FND-01/02, STR-01–03, JT-JT01-JT06, 6 rows with `independent_evidence = pending`
2. **Independent witness round** — fresh adversarial reviewers must sign off on frozen catalog bytes
3. **TASK-001 bootstrap** — canonical `taskfmt verify` + `tc-proof-host` non-runnable until harness exists
4. **Catalog commit/freeze** — arm only from committed SHA; record exact tree hash at readiness

**Recently closed (no longer blockers):** ROOT-08/09 (F03/F06/F08c/F17 adjudicated); ROOT-11/12 validator repairs; tui-snap pin.

---

## 4. Verification command audit — iteration is NOT blocked

Measured 2026-09-15. Re-confirmed CI smoke locally: **7550 listed / ~302 PTY captures executed, all PASS, ~87 s**.

### Iteration tier (use during every edit loop)

| Command | Duration | Blocker? |
| --- | --- | --- |
| `cargo nextest run` | **~11 s** | No |
| `cargo nextest run -E 'test(store_integrity)'` | **~1 s** | No |
| `TUISNAP_FAST=1 … & test(<filter>)` (25 combos) | **~4 s** (e.g. `showcase_pages_overview`) | No |
| Single combo filter | **~1–2 s** | No |
| `cargo nextest run --profile ci --run-ignored only -E 'binary(visual_baseline)'` | **~85–87 s** | No — PR smoke green |

### Boundary tier (mandatory at acceptance only — not every edit)

| Command | Duration | Blocker? |
| --- | --- | --- |
| Fidelity full matrix (no `TUISNAP_FAST`) | **~45–60 min est.** | Duration at boundaries only |
| `TUISNAP_FAST=1` full matrix | **~30 min measured** | **Not acceptance** — 7350/7550 pass; 200 expected FAST-vs-fidelity drift on timing-sensitive flows |

### Campaign canonical gates (blocked until TASK-001)

| Command | Status |
| --- | --- |
| `taskfmt verify …` | **FAIL** on bare checkout — expects container paths + `tc-proof-host` |
| `tc-proof-host verify --run …` | **Binary absent** |

**Conclusion:** Rust/visual iteration commands are fast and green. The prompt's mandatory **fidelity** full matrix (~45–60 min) runs only at task acceptance, app closures, and TASK-069 — **not** during edit loops. Edit loops use targeted `TUISNAP_FAST=1` filters per [`campaign-iteration-guide.md`](campaign-iteration-guide.md) §1.

---

## 5. Visual baseline performance (committed on branch)

Infra committed in `32c238e1`, `f395f284`, `c5384043`. Pin: `tui-snap` @ `0a2e490`.

| Scenario | Wall time | Notes |
| --- | --- | --- |
| `showcase_pages_overview` (25 combos, FAST) | **~4 s** | Edit-loop representative |
| CI smoke (7550 slots, ~302 PTY captures) | **~85–87 s** | All green @ `623a1a59` |
| Full FAST matrix | **~30 min** | Optional nightly; not acceptance oracle |
| Full fidelity matrix | **~45–60 min est.** | Mandatory acceptance gate |

---

## 6. Prompt preparation applied

The stored prompt matches the coordinator specification. Preparation added (without weakening acceptance):

| Amendment | Location | Status |
| --- | --- | --- |
| Iteration vs acceptance split | `campaign-execution-prompt.md` visual section + link to iteration guide | **Applied** |
| Edit-loop targeted `TUISNAP_FAST=1` commands | execution prompt + iteration guide §1 | **Applied** |
| Fidelity-only mandatory full gate | execution prompt, iteration guide §3–§4, `visual-validation.md` | **Applied** |
| FAST full matrix ≠ acceptance | iteration guide §3 (7350/7550 measured) | **Applied** |
| IMPLEMENTER may use targeted visual for feedback | execution prompt § Mandatory per-task agent protocol | **Applied** |

---

## 7. Pre-campaign checklist

### A. Phase 0 (required before arming `/goal`)

- [ ] Close **15 open** rows in [`reaudit-findings.tsv`](reaudit-findings.tsv)
- [ ] Complete **6 pending** independent-evidence reviews
- [ ] Finish **BRANCH-01** Jackin/TablePro partition integration
- [ ] Re-prove all **20** §22 items in [`planning-acceptance.md`](planning-acceptance.md)
- [ ] Obtain fresh **`READY FOR REFACTORING EXECUTION`** on frozen catalog bytes
- [ ] Commit + hash-freeze campaign docs (`campaign-execution-prompt.md`, iteration guide, this report)

### B. Execution prerequisites (after Phase 0, before TASK-001 production)

- [ ] Create worktree from architectural `main` @ `7b27732a…`
- [ ] Implement + qualify TASK-001 bootstrap (`tc-proof-host`, comparator fixtures)
- [ ] Coordinator re-runs Phase 0 gate checklist, then dispatches TASK-001 only

### C. Explicit non-starters

- **Do not** arm campaign `/goal` yet
- **Do not** dispatch TASK-001 on `visual-baseline` for production edits
- **Do not** treat green mechanical validators as execution authorization

---

## 8. Recommended next step

1. **Commit campaign prep docs** (prompt, iteration guide, prep report, `visual-validation.md`) — removes ambiguity before catalog freeze.
2. **Continue Phase 0** — close 15 open findings + independent witness round (est. weeks, parallel subagents).
3. **When Phase 0 green:** arm `/goal` from committed `campaign-execution-prompt.md`; first production work is TASK-001 bootstrap on architectural-main worktree.

**Do not start the campaign goal in this preparation session.**

---

## Appendix: readiness summary

| Dimension | Ready? |
| --- | --- |
| Prompt stored + iteration guidance | **Yes** (uncommitted edits pending commit) |
| Verification commands block iteration? | **No** — edit loop ~4 s–87 s; full matrix only at boundaries |
| Phase 0 mechanical validators | **Yes** |
| Phase 0 process / witness / findings | **No** — 15 open |
| TASK-001 harness | **No** |
| Arm campaign `/goal` now? | **No** |
