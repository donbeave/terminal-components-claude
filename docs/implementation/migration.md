# In-place migration sequence

This document defines how the existing repository is progressively refactored
into Termrock. It is a migration of ownership and implementation boundaries,
not a repository copy or a product rewrite.

## Fixed starting point

The migration starts at the annotated `visual-baseline` tag resolved to
`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` and continues on
`termrock-refactor`. The tag, its release, source tree, applications, tests,
fixtures, and snapshots are frozen oracle material for this work.

The current Cargo package is `junie-tui`, with library name `junie_tui` and
four binaries: `showcase`, `tablepro`, `jackin-preview`, and `holla`. Current
source identifiers such as `junie_tui`, `WidgetId`, `Outcome`, and `RenderCtx`
must be described as legacy/current implementation names until an actual
implementation change replaces them. This documentation rewrite does not edit
Cargo metadata or Rust source.

The earlier specification's separate `termrock-new` destination is
superseded. There is one project: this repository, this branch, and its
existing source and applications. Do not initialize a clean-room tree, copy
the workspace into a second repository, or plan a later application migration
between repositories.

## Migration invariants

1. **Oracle immutability.** The baseline source and approved output remain
   unchanged. New instrumentation may live in a separately reviewed trusted
   adapter, but may not change rendering, layout, event handling, fixtures, or
   expected results.
2. **Application preservation.** The four applications remain reference
   consumers. Their scenarios, fixture worlds, visual output, keyboard paths,
   pointer paths, and snapshots remain the target. An application edit is
   allowed only in a later phase when required to adopt a reusable Termrock
   component, with exact parity evidence.
3. **Library-only scope.** The library owns reusable presentation and generic
   interaction. Callers own domain state, persistence, providers, filesystem,
   PTYs, clipboard emission, and product actions.
4. **Contract-first change.** A public API or shared foundation decision is
   recorded in its canonical document before dependent implementation tasks
   consume it. Tasks link to contracts instead of copying them.
5. **Reversible increments.** Each phase has an external-consumer gate and an
   oracle comparison gate. Keep old and new paths comparable until the new
   path proves the applicable behavior; remove compatibility only after the
   gate closes.
6. **No candidate authority.** Candidate frames, candidate hitboxes, or
   candidate action traces cannot establish their own expected result. Trusted
   oracle coordinates and expectations are sealed before candidate comparison.

## Sequence

### 0. Qualify the baseline and inventory

Pin the source SHA, original `tui-snap` revision, renderer/profile inputs,
fixtures, dimensions, time/motion modes, and capability modes. Inventory the
current exports, component source, parts, app-local compositions, snapshots,
tests, and observable interactions. Reconcile this inventory with the imported
45 component/API surfaces, 12 foundation contracts, 54 legacy-family rows,
and 45 capture plans.

Build the comparator, trusted oracle/extraction adapters, and mutation probes
before candidate components. Preserve provenance for every extracted frame and
interaction trace. A missing oracle state becomes capture work; it is never
silently accepted from the candidate.

Seal the baseline TablePro pending-edit SQL preview as an `ExistingOracle`
contract before the consumer migration phase. Record full ordered statement
vectors for update, insert, and delete cases, including original-key
predicates, `DEFAULT` handling, no-op reverts, composite keys, and the
no-primary-key fallback. Candidate code cannot supply or approve those expected
vectors.

### 1. Install shared ownership and the target facade

Introduce the Termrock architecture behind the existing repository boundary in
the order defined by P1: identity/revisions; normalized input, keymaps, and
typed responses; runtime focus/hover/capture/press/time; layers; measurement
and layout; semantic theme; text/editing; collections/reconciliation; and the
constrained author surface.

The target calls are `update`, `draw`, and `measure`. Durable state remains
caller-owned. Props borrow current model values. Draw receives immutable state
and must be repeatable; semantic changes belong to update. Runtime owns focus,
hit regions, capture, layers, geometry publication, and feedback. No universal
boxed `Widget` trait, `show()` method, or untyped event bus is introduced as a
compatibility shortcut.

Compile a generic external consumer at this boundary. It must use only the
public Termrock facade and must not import private runtime registries or
application modules.

### 2. Migrate mechanisms before component families

Move behavior onto shared mechanisms in dependency order:

- one identity/revision and reconciliation path for dynamic collections;
- one runtime path for focus, hover, pointer capture, press feedback, geometry,
  and layers;
- one measurement/layout and semantic theme path;
- one text editing core for TextInput, TextArea, and CodeEditor while retaining
  their distinct baseline policies;
- one ScrollRegion for bounds, thumb capture, and edge fades;
- one Grid engine for table-row and cell/grid modes;
- one Menu engine for Menu, ContextMenu, and MenuBar;
- one Picker mechanism for Picker, CommandPalette, and PickerChain;
- one StatusBar presentation that absorbs the old segments-style surface;
- Panel plus TextViewport in place of a separate ScrollPanel mechanism;
- DerivedHintBar as metadata adapter, not a second painter.

Consolidation reduces mechanisms while preserving visual recipes, interaction
policies, and component-specific state. It does not flatten distinct behavior.
Each shared mechanism closes its own foundation tests before family migration.

### 3. Migrate component families in P2–P5

Migrate basic chrome and collections first, then editing/choices/forms/secrets,
then overlays/menus/pickers/completion, then rich output/grid/editor/status.
For each component, keep an explicit mapping from current source behavior to
the canonical component contract, constructor/props/state/actions, parts,
dependencies, applicable state axes, oracle cases, and negative mutations.

Do not move product-specific composites into the public library. Re-express
those compositions as generic caller-owned data and public component calls.
Do not rename current source paths merely to make documentation appear ahead of
the implementation.

### 4. Add TerminalView and composed conformance

Implement `TerminalView` as a generic consumer of caller-provided prepared
terminal cells. It can display styles and continuation cells, cursor and
selection, and emit typed interaction, copy, and link requests. It does not
parse terminal escapes, run a PTY, act as a shell or emulator, manage a daemon,
or own an agent session. Optional terminal/session cleanup belongs at the edge
adapter and conformance harness.

Build generic list/detail, editor/inspector, nested form/picker, progress/output,
tabbed-pane, and terminal-cell scenes using public APIs. These scenes qualify
composition and API usability without rebuilding Jackin, Holla, or TablePro.

### 5. Migrate applications only for component adoption

After reusable mechanisms and components pass their own gates, migrate one
reference consumer at a time where the old implementation calls are replaced by
the new public Termrock calls. Preserve fixture data, scenario names, layout,
keyboard and pointer programs, capability/motion modes, and snapshot paths.
Compare at the same dimensions and checkpoints. A failed parity case blocks
removal of the old path and is repaired in the reusable library or its contract
owner, not hidden by an app-local painter.

TablePro has one narrow caller-side adapter exception: its existing
`preview_sql` function may change how it reads rows and pending edits when the
consumer moves from `DataGrid` to the caller-owned Termrock `GridModel` and
`GridEditor` contracts. Preserve the full ordered SQL output byte-for-byte for
equivalent table, column, original-row, and pending-edit inputs. Keep the
existing dirty-row/column ordering, original-key predicates, default handling,
no-op revert behavior, SQL literal formatting, or row targeting when keyed
rows are reordered, filtered, or removed. Do not change
`sql_literal`, query execution, Safe Mode, history, completion, switcher, or
database behavior. The P6 consumer task owns this adapter; because task-format
scope is file-level, an independent diff review must confirm that only the
`preview_sql` adapter and its directly related input fixture changed in
`model.rs`.

The applications remain conformance evidence throughout. No application gets a
new feature, service, route, backend, product model, or visual refresh as part
of this migration.

### 6. Close the identity transition and remove legacy paths

When all public consumers use the Termrock facade and the compatibility paths
are no longer needed, perform any package, crate, module, or namespace rename
as a separately reviewed implementation step. Update public examples, docs,
and task references atomically; retain literal application-visible branding and
oracle strings where they are part of the frozen output.

The rename must preserve the same API contracts, rendered cells, actions,
focus/capture observations, and snapshot provenance. It is not evidence of
completion by itself. Current package truth remains documented until this step
actually lands.

## Verification at each boundary

At every phase boundary run the applicable checks in
[`../verification/`](../verification/):

- exact dimensions, symbols, wide-cell continuation, colors, modifiers, and
  cursor state;
- focus owner, capture owner, layer path, selected stable key, navigation key,
  draft/committed values, typed action count, and typed action target;
- pointer down/held/release-inside/release-outside and keyboard-suppressed
  hover;
- editing, controlled choices, source reorder/removal, resize, overlays,
  scroll/fades, disabled/read-only, empty/loading/partial/error, and all unique
  motion phases where applicable;
- one-cell/style/cursor/action-target/duplicate-activation/missing-artifact and
  wrong-oracle negative mutations;
- external public-API examples and draw-purity/repeatability checks.

Use `ExistingOracle`, `ExtractedOracle`, and `Extension` lanes as defined by the
verification contract. Extension cases for new adapters, themes, or robustness
must be labeled separately and cannot replace baseline cases.

## Completion criteria

The in-place migration is complete when P0–P7 are accepted, every component,
foundation, family disposition, and applicable capture plan is traceable, the
core library is independent of backend/product concerns, and a fresh consumer
can use the public Termrock API. All four reference applications still match
the frozen baseline in output and observable interaction. Any remaining real
product integration or package ecosystem work is a separately approved future
plan.
