# Termrock documentation

This tree is the repository's current specification for its in-place refactor into Termrock.

## Authority

1. [GOAL.md](../GOAL.md) owns the current mission, scope, protected baseline, and success conditions.
2. `architecture/overview.md` owns the high-level map. Detailed contracts live once under `api/`, `design/`, `foundations/`, `components/`, and `verification/`.
3. [implementation/plan.md](implementation/plan.md) owns future phase ordering; [refactoring-tasks/](../refactoring-tasks/README.md) turns it into task packages that link back to contracts.
4. Frozen source and approved output at `visual-baseline` commit `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` own visual and observable interaction evidence.
5. Git history retains superseded planning decisions. Old working-tree plans are removed after migration.

The supplied `termrock-library-spec/` directory was migration input. The destination is this repository on `termrock-refactor`; no separate Termrock repository is planned. Machine-readable component, family, provenance, and capture references are under `reference/`.

## Reading map

- [Architecture overview](architecture/overview.md) — target ownership and boundaries.
- [Public API](api/README.md) — props, state, phases, types, and authoring.
- [Visual and interaction contracts](design/visual-contract.md) — baseline presentation and input grammar.
- [Foundations](foundations/README.md) — one owner for each shared mechanism.
- [Components](components/README.md) — all 45 public component/API surfaces.
- [Verification](verification/README.md) — frozen oracle, parity, provenance, and conformance.
- [Applications](applications/README.md) — preserved conformance consumers and current run commands.
- [Implementation](implementation/README.md) — in-place P0–P7 sequence and migration boundary.
- [Task catalog](../refactoring-tasks/README.md) — current task-format revision, packages, and dependency graph.

## Current source names

The frozen source still builds package `junie-tui` and library `junie_tui`. Names such as `WidgetId`, `Outcome`, and `RenderCtx` describe current implementation only. They are not future Termrock compatibility obligations. Literal app-visible strings such as `jackin❯` and `holla❯` remain part of the oracle.
