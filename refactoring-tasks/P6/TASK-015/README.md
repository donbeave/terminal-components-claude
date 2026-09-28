---
schema: task/v5
id: TASK-015
title: "Migrate preserved consumers and prove composed conformance"
kind: refactor
---

# TASK-015 — Migrate preserved consumers and prove composed conformance

## Goal

The four preserved applications adopt the refactored reusable components through narrow call-site changes while retaining their frozen observable behavior.

## Context

The applications are reference consumers and integration fixtures, not product targets. This is the only package with application source scope. Its verifier enumerates exact existing presentation files that call reusable TUI APIs; it does not grant an entire application directory. The files may change only where needed to adopt the public Termrock components and caller-owned state. Scenarios, product behavior, source data, and outputs remain unchanged.

Dependencies: TASK-008, TASK-010, TASK-014. Canonical contracts: [applications/README.md](../../../docs/applications/README.md), [applications/showcase.md](../../../docs/applications/showcase.md), [applications/tablepro.md](../../../docs/applications/tablepro.md), [applications/jackin-preview.md](../../../docs/applications/jackin-preview.md), [applications/holla.md](../../../docs/applications/holla.md), [verification/conformance.md](../../../docs/verification/conformance.md), [verification/visual-parity.md](../../../docs/verification/visual-parity.md).

## Preconditions

- **P-001:** The caller has supplied a committed descendant of annotated `visual-baseline` (`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`) on `termrock-refactor`.
- **P-002:** The linked canonical architecture, API, component, design, and verification documents are present.
- **P-003:** The future implementation environment provides `rtk` and `cargo nextest`.

## Scope

In scope:

- The exact application presentation call-site files listed in `verify.toml`: Showcase app/page composition; TablePro app, connection, tab, workbench, and the Grid-to-existing-preview adapter; Jackin Preview app/screen composition; Holla app/screen composition; library-level composition probes; preserved application conformance traces; and integration evidence against the immutable oracle.
- Generic additions under `tests/termrock/` and `tests/conformance/` needed to exercise these compositions.

Out of scope:

- Any shared library implementation path (`src/lib.rs`, `src/core/`, `src/runtime.rs`, `src/theme.rs`, `src/ui/`, or `src/widgets/`); implementation gaps return to their owning component/foundation task.
- Every application file absent from the exact `verify.toml` allowlist, including app entrypoints/CLI scenario selection, app tests, Showcase data, Jackin/Holla domains, simulators, scenarios, clocks, and art helpers, and TablePro database/SQL modules.
- New routes, services, providers, databases, Docker, authorization, Git operations, product redesign, fixture/data changes, and application behavior changes.

## Requirements

- **R-001 (MUST):** Deliver the scoped mechanism according to the linked canonical contracts.
- **R-002 (MUST):** Preserve caller-owned state, borrowed props, typed actions, stable identities, and shared foundation ownership required by the API.
- **R-003 (MUST):** Application edits are limited to the exact existing presentation files enumerated in `verify.toml`; app entrypoints, app tests, fixture/data/domain/simulation/scenario files, database/SQL modules, snapshots, and visual-baseline tests remain unchanged. The four applications retain their scenarios, observable interaction, and rendered output.
- **R-004 (MUST NOT):** No product implementation, service integration, repository migration, or application redesign is added to this library task.
- **R-005 (MUST):** The completion gate succeeds.

## Acceptance criteria

### AC-001 — Composed library scenarios match expected focus, capture, selection, navigation, state, and rendering outcomes for all four applications.
```gherkin
Given the linked canonical contracts and the accepted dependency work
When the scoped Termrock mechanism is exercised
Then its update, measure, draw, and typed-action behavior matches the contract
```

**Verification**

- **Type:** scenario
- **Covers:** `R-001`
- **Check:** `CHK-001`

### AC-002 — Nested overlays, resize, narrow geometry, motion phases, source reconciliation, and cross-component event routing preserve parity.
```gherkin
Given an applicable boundary, interaction, or reconciliation state
When the future implementation is exercised
Then the documented state, identity, geometry, and ownership invariants hold
```

**Verification**

- **Type:** scenario
- **Covers:** `R-002`
- **Check:** `CHK-002`

### AC-003 — Frozen references and scope remain protected
```gherkin
Given the four explicit migration paths and the immutable visual-baseline outputs
When the candidate tree is checked after implementation
Then only component-adoption call sites changed and application interaction, output, snapshots, baseline tests, and product integrations are unchanged
```

**Verification**

- **Type:** invariant
- **Covers:** `R-003, R-004`
- **Check:** `CHK-003`

### AC-004 — Completion gate passes

**Verification**

- **Type:** gate
- **Check:** `CHK-004`

## Fixed decisions

- **D-001:** Applications remain reference consumers; the exact file allowlist permits only existing reusable-library call sites and the TablePro Grid value adapter. It excludes app entrypoints, app tests, fixtures, domains, simulators, scenarios, and product/backend modules.
- **D-002:** The work stays in this repository on `termrock-refactor`; it never creates a separate implementation repository.
- **D-003:** The package's verifier scope is the enumerated presentation files plus generic library/conformance tests; no whole application directory or library implementation path is writable. Forbidden paths additionally protect known app tests, fixtures, product modules, and frozen oracle output.

## Checklist

<!-- checklist:start -->
- [ ] **1** Prepare.
    - [ ] **1.1** Read the linked canonical contracts and confirm the accepted starting tree. (`R-001`, `AC-001`, `CHK-001`)
- [ ] **2** Implement.
    - [ ] **2.1** Deliver the scoped Termrock mechanism. (`R-001`, `AC-001`, `CHK-001`)
    - [ ] **2.2** Prove shared ownership, identity, and applicable edge behavior. (`R-002`, `AC-002`, `CHK-002`)
    - [ ] **2.3** Keep edits inside the exact presentation-file allowlist, leave app tests/data/product modules untouched, and preserve frozen behavior/output. (`R-003`, `R-004`, `AC-003`, `CHK-003`)
- [ ] **3** Verify.
    - [ ] **3.1** Run the one completion gate. (`R-005`, `AC-004`, `CHK-004`)
<!-- checklist:end -->
