# Termrock implementation plan

This is the canonical P0–P7 sequence for the future Rust refactor. The current
branch contains documentation and planning only. Work begins from the frozen
`visual-baseline` commit `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` and stays
on `termrock-refactor`.

## Rules for every phase

- Work in this repository and preserve its four reference applications.
- Treat the baseline source, approved snapshots, and observable interaction at
  the pinned commit as immutable oracle material.
- Implement generic library behavior only. Domain logic, services, PTYs,
  providers, persistence, product routes, and application redesign are out of
  scope.
- Use the contracts in [`../architecture/`](../architecture/),
  [`../api/`](../api/), [`../foundations/`](../foundations/), and
  [`../components/`](../components/) as the specification. A task may narrow
  or sequence a contract, but may not redefine it.
- Keep update, draw, and measure distinct. Durable state belongs to the
  caller; props are short-lived borrows; actions are typed responses.
- Keep focus, hit testing, pointer capture, layers, geometry, scrolling, text
  editing, theme resolution, and identity in their shared owners.
- Expected output is created only by a trusted `ExistingOracle` or
  `ExtractedOracle` path. Candidate code cannot bless its own snapshot.
- Every applicable behavior case needs exact visual and interaction evidence;
  non-applicable states need a recorded reason.
- Implementation quality, toolchain, dependency isolation, performance
  evidence, and independent-review requirements follow
  [`quality-gates.md`](quality-gates.md).

The source pack's inventory is the coverage floor: 45 component/API surfaces,
12 shared foundations, all 54 legacy-family dispositions, and 45 capture plans.
The repository documentation reconciles those inventories in the canonical
component, foundation, and verification sections.

## Phase DAG

| Phase | Scope | Required outputs | Gate |
|---|---|---|---|
| **P0 — reference qualification** | Pin the source commit, tool/profile inputs, and approved snapshot corpus. Inventory public exports, parts, source behavior, application compositions, capture plans, missing states, and legacy-family dispositions. Establish the repository-local Mise toolchain pin and nonpublished conformance package; build the comparator and oracle adapters before candidate implementation. | Immutable source/provenance manifest; one public library plus test-only conformance package in this workspace; pinned stable toolchain; sealed dimensions and event programs; component/foundation coverage matrix; comparator fixtures including TablePro pending-edit preview SQL vectors; baseline mutation probes. | Deliberate one-cell, style, cursor, action-target, duplicate-action, missing-case, and wrong-oracle-input mutations fail. The production dependency graph excludes conformance tools. Candidate code cannot create expected output. |
| **P1 — foundations and public API** | Establish the Termrock facade and external-consumer harness. Implement identity/revisions, normalized input and typed responses, runtime focus/hover/capture/press/time, layers, measurement/layout, semantic theme, and shared text/model contracts. | Public signature probes; generic scene driver; ownership and purity tests; stale-geometry, duplicate-key, overlay, color-capability, and draw-repeatability fixtures. | External examples use only public exports; update/draw/measure boundaries hold; geometry invalidation, identity, layer ownership, and capability cases pass. |
| **P2 — basic components and collections** | Implement Button, Brand, Panel, SplitPane, ScrollRegion, List, NavList, Tree, Tabs, Empty, KeyHint, HintBar, and TooSmall on the shared mechanisms. | Generic list/detail and nested/resize/pointer compositions; collection reconciliation cases; per-component oracle captures and negative mutations. | Baseline recipes and applicable focus/hover/press/scroll/resize behavior match without domain modules or duplicate runtime mechanisms. |
| **P3 — editing, choices, forms, and secrets** | Implement Field, TextInput, TextArea, Checkbox, Toggle, RadioGroup, Select, ChipBar, Form, Props, PropsList, and secret validation on the shared text, identity, and response contracts. | Controlled-value and revision fixtures; draft/commit/cancel/conflict cases; Unicode and selection tests; secret leak/clone/debug negatives; form composition consumer. | TextInput and TextArea retain their distinct baseline Escape/paste policies. Choices remain caller-controlled. Secret values never enter output, traces, or debug artifacts. |
| **P4 — overlays, menus, pickers, and completion** | Implement Dialog, Menu, ContextMenu, MenuBar, FilterList, Picker, CommandPalette, PickerChain, Completion, and Wizard using one layer/menu/picker mechanism. | Nested modal and anchored-popup compositions; focus restoration and Escape ladder; paste/typing; stable-target and outside-release cases; completion and chained-selection fixtures. | Modal ownership, click-through barriers, reanchoring, dismissal, and typed actions match the oracle. No component invents a second menu, picker, or layer engine. |
| **P5 — rich output and status** | Implement TextViewport, one Grid engine (table-row and cell/grid modes), CodeEditor, DiffView, Steps, ProgressBar, Spinner, Meter, StatusBar, and HelpOverlay. Apply shared text, scroll, theme, layout, and motion contracts. | Output retention and projection fixtures; grid selection/editing capability cases; diff fallback; read-only paths; all unique animation phases and boundary timings; status/help compositions. | Exact symbols, wide-cell continuation, cursor, scroll/fade, status, and motion behavior match. Variants remain visually distinct despite shared mechanisms. |
| **P6 — terminal edge and composed conformance** | Implement TerminalView as a borrowed prepared-cell presentation component with typed input/copy/link requests. Finish generic composed fixtures and optional terminal/session adapter tests. | Synthetic cell/cursor/selection fixtures; key/mouse/paste/resize laboratory; cleanup tests; public-API composed scenes covering all reusable mechanisms. | Cell ownership, continuation, cursor, selection, and request routing pass without a PTY, shell, emulator, parser, or daemon dependency in core. |
| **P7 — closure and independent review** | Audit public API and parts; reconcile every component/foundation/family row; run exact parity, negative mutation, performance, and independent review gates. Remove temporary compatibility paths only when consumers pass. | Complete traceability; final exact comparison reports; mutation-gate results; cold/warm build and frame measurements; independent API, visual, coverage, runtime, and adversarial reviews. | Zero unresolved required cases; no approved-oracle edits; no duplicate painters or parallel mechanisms; public examples compile; all application parity gates remain green. |

## Dependency order inside phases

P0 is a prerequisite for all implementation. Build and negatively qualify the
comparator before accepting any candidate frame. Establish the one public
library and nonpublished conformance package inside the existing workspace,
and pin the stable toolchain through Mise. Source and capture inventory are
independent from candidate code.

Within P1, settle identity, revision, response, clock, model, theme, geometry,
and text signatures before downstream component work. Runtime/layers and
layout/theme/text may proceed in separate work packets only when their shared
contracts are integrated before P2 acceptance.

Within P2, establish ScrollRegion and Button before collection rows and
composed shells; establish KeyHint before HintBar. Within P3, establish shared
text, Field, and secret handling before TextInput/TextArea; then choices and
Form. Within P4, establish Dialog and FilterList before Picker; establish Menu
before ContextMenu/MenuBar; establish Picker before PickerChain and completion.
Within P5, establish Spinner before Steps/Meter/StatusBar, and TextViewport
before CodeEditor/DiffView/HelpOverlay. P6 consumes the stable cell, input,
theme, and runtime contracts. P7 follows all phase gates.

The machine-readable component dependency graph in the imported reference is
an input to the task catalog. The final task catalog must validate an acyclic
dependency closure and link each task to the canonical repository docs.

## Work package contract

The task catalog should group work by shared mechanisms rather than create a
task for every widget. A package may cover a foundation plus its dependent
components when ownership is clear. Each package names:

1. canonical architecture/foundation/API/component/verification references;
2. exact writable implementation paths for the future phase;
3. prerequisite task IDs and the acceptance gate it closes;
4. external-consumer or generic-lab coverage;
5. source/oracle references and capture-plan IDs;
6. positive behavior and exact frame evidence;
7. at least one paint mutation and one action/target mutation;
8. scope exclusions, especially application product behavior.

If the catalog uses `task-format`, it must record the exact upstream revision
that was inspected and run that revision's verifier. The catalog may not carry
forward an obsolete schema or invent a private incompatible format.

Implementers may not add a new runtime, painter, identity scheme, scrolling
engine, menu engine, picker engine, or text editor core to bypass a shared
contract. Shared API changes belong to the contract owner and must update the
canonical document before dependent tasks proceed.

An accepted package leaves implementation, separate-file tests, source
references, exact actual/diff artifacts, and a factual result. Progress logs,
generated coordination state, stubs, unreviewed compatibility layers, and
candidate-generated expected output are not deliverables.

## Proof required for each applicable component

The implementation must exercise `update`, `draw`, and `measure` with the
documented controlled-state semantics. Run headless state/action tests,
draw-purity and repeatability checks, keyboard and pointer traces in the
generic lab, resize and source-reconciliation cases, and the required
`tui-snap` oracle comparison. Verify every documented part/slot affects only
its assigned rectangle and retains the shared style/ownership rules.

Capture all applicable axes from the verification contract: geometry,
focus/hover, pointer lifecycle, controlled choice, editing, data states,
scroll, layers, motion, capability/color, and boundary timing. Decorative
components receive a justified non-applicability disposition rather than
fabricated interaction cases. New adapter, theme, robustness, or terminal
states use a separately reviewed extension lane.

## Scope exclusions and stop line

No task may implement real Jackin services, Docker, provider integrations,
account authorization, Git operations, Holla product behavior, TablePro
database backends, new product flows, or new application visual design.
Application changes are limited to later reusable-component adoption and must
retain exact scenarios, snapshots, and observable interaction.

The library milestone ends when all 45 component/API references, 12
foundations, 54 legacy-family rows, and applicable capture plans have passing
evidence or an explicitly reviewed scope disposition; public examples compile;
the core has no backend requirement; generic compositions use public exports;
and the four frozen applications remain parity consumers. Future product
integration needs a separate approved plan.
