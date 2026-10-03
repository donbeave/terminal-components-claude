# Termrock refactor goal

## Mission

Refactor the existing implementation in this repository into a minimal reusable Rust TUI library named Termrock. The work begins from frozen commit `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` and continues in place on `termrock-refactor`. This repository is the future Termrock project; no new or clean-room repository is part of the plan.

## Current phase

This phase establishes the complete Tuiscotti visual and interaction testing baseline and Velnor CI infrastructure as the mandatory prerequisite before P1 and any production refactor. It does not rename the repository, Cargo package, crate, source, or applications, and it does not implement the Rust refactor. Future implementation begins only after this verification foundation is established.

## Frozen visual and interaction contract

The applications `showcase`, `tablepro`, `jackin-preview`, and `holla`, with their fixtures, scenarios, output, keyboard and pointer behavior, tests, and snapshots, are immutable reference consumers. Their frozen appearance and observable interactions are the target for future Termrock work. Preserve them one-for-one; any product change needs a separate explicit approval. The annotated `visual-baseline` tag is immutable; its peeled commit above is the baseline authority.

## Target architecture

- Caller owns durable domain values; components receive short-lived borrowed props and caller-owned interaction state.
- Components separate `update(...)`, immutable `draw(...)`, and `measure(...)`; semantic results use typed actions.
- Stable semantic IDs, item keys, column keys, and revisions preserve identity across source changes.
- The shared runtime owns focus, hit testing, hover, pointer capture, press feedback, geometry publication, layers, and time.
- Shared foundations own layout, theme resolution, text editing, collections, and scrolling.
- The public author API exposes constrained parts and slots; applications do not rebuild focus, layers, text editing, or painting around components.

Detailed contracts live once under `docs/`. The implementation task catalog derives from those contracts and does not redefine them.

## Scope

The product scope is reusable Termrock components, foundations, and generic conformance compositions. Jackin, Holla, TablePro, and Showcase product development, service integrations, product redesigns, and new application flows are excluded. Future application changes are limited to adopting the refactored library while keeping the frozen behavior and output intact.

## Verification

Use the frozen source and approved snapshots as an immutable oracle. Future implementation uses `tuiscotti` (pinned at `a47c9aaefb34e4c00026f99d8a8dd7ee5916b274`), deterministic interaction traces, exact applicable state coverage, and negative mutation gates. Keep `ExistingOracle`, trusted `ExtractedOracle`, and `Extension` evidence separate. Candidate code cannot author or approve expected output. All visual checkpoints produce six formats (.ansi, .html, .png, .ascii, .txt, .frame.json) from one observation. CI is generated via Velnor Actions (pinned release `velnor-actions-0.1.0`). See [visual parity](docs/verification/visual-parity.md), [interaction parity](docs/verification/interaction-parity.md), and [oracle provenance](docs/verification/oracle-and-provenance.md).

## Success

Termrock is produced by progressively replacing reusable internals in this repository. All documented components and shared foundations have one public contract and one implementation owner. The four applications continue to render and interact exactly as at the frozen baseline, and the library remains independent of product-specific business logic. The future package and crate identity may become Termrock during implementation; the baseline code has not been renamed by this planning goal.

