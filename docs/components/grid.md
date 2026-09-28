# Grid

**Component ID:** W28 · **Phase:** P5 · **Legacy families:** C35, C36
**Oracle:** `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`

This is the canonical contract for the Termrock Grid surface. It describes the
future API while the implementation is still the frozen source tree. During the
implementation phase the current `src/widgets/grid.rs` and
`src/widgets/table.rs` remain read-only oracle evidence; they are not the target
public API.

## Purpose and boundary

Grid renders keyed rows and columns in either a row oriented table recipe or a
cell oriented data grid recipe. `GridModel` supplies revision, stable row and
column keys, borrowed cell values, readiness, and optional sort or fetch
metadata. `GridEditor` is an explicit opt in for local draft validation and
commit.

The caller owns records, sorting, filtering, persistence, SQL, undo policy,
fetching, and transactions. Grid owns navigation, selection, edit drafts,
projection, and typed requests. A read-only model is never mutated through a
render or interaction path.

Termrock has one Grid engine. Table row mode and editable cell/grid mode are
presentations and navigation policies over that engine. They must not become
parallel table and data-grid implementations.

Editable cell drafts use the shared [`TextInput`](./text-input.md) and
[`TextArea`](./text-area.md) contracts where their recipe requires single-line
or multiline editing.

### Non-goals

- database access, SQL generation, server side sorting, filtering, or fetches;
- a retained row collection inside the component;
- implicit mutation of a caller model from `draw`;
- application specific reference viewers, forms, or pending transaction bars;
- a generic boxed widget trait or a compatibility promise for `WidgetId`,
  `Outcome`, or `RenderCtx`.

## Frozen evidence and provenance

The baseline evidence is pinned to the annotated `visual-baseline` tag at
`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`.

- Source behavior: `src/widgets/grid.rs` (blob
  `f68c75c3d09d828c23cdf4a85faf57cdc8427b05`) and
  `src/widgets/table.rs` (blob
  `d9a718082697cd77a904f89725247908c8227e00`).
- Frozen component tests: the inline tests in those two source files, including
  sort identity, invalid edit preservation, Unicode edit windows, pending
  changes, range copy, and fetch-more cases.
- Visual/conformance consumers: Showcase datagrid, tables, and editable pages;
  TablePro workbench/grid scenarios; the protected snapshots below
  `snapshots/showcase/pages/datagrid`, `tables`, `editable`, and the matching
  `tests/visual_baseline` scenarios.

These paths are evidence only. New captures must bind to the frozen output
before candidate output is accepted. See
[`../verification/visual-parity.md`](../verification/visual-parity.md),
[`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md),
and [`../api/public-api.md`](../api/public-api.md).

## Target public API

The notation below follows the shared API contract. It is a target for the
in-place refactor, not a claim about current Rust exports.

```rust
Grid::new(id: Id) -> Grid<'a>

update(
    &self,
    cx: &mut Cx<'_>,
    state: &mut GridState,
    model: &dyn GridModel,
) -> Response<GridAction>

update_editable(
    &self,
    cx: &mut Cx<'_>,
    state: &mut GridState,
    model: &mut dyn GridEditor,
) -> Response<GridAction>

draw(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    state: &GridState,
    model: &dyn GridModel,
) -> Rect

measure(
    &self,
    cx: &MeasureCx<'_>,
    model: &dyn GridModel,
    constraints: Constraints,
) -> Size
```

Builders are constrained to declared parts and policies:

```rust
navigation(NavUnit::Row | NavUnit::Cell)
presentation(GridPresentation::Table | GridPresentation::DataGrid)
selection_mode(SelectionMode)
cell(&GridCellPainter)
column_fit(ColumnFit)
readiness(Readiness<'a>)
patch(StylePatch)
```

`GridModel` exposes stable `RowKey` and `ColumnKey` values, a source revision,
column metadata, row count or estimate, and borrowed values. Ragged rows and
empty sources are valid inputs. `GridEditor` adds explicit validation, draft
read/write, and commit operations; the read-only constructor cannot obtain that
capability by accident.

### Durable state and actions

`GridState` owns only interaction state:

- current `CellKey` and optional range-selection anchor;
- row and column scroll state;
- selected stable row keys;
- optional edit draft, error, and edit phase;
- requested sort or filter intent when the caller has not yet reconciled it;
- any local fetch-more/readiness marker needed to avoid duplicate requests.

It does not own rows, values, geometry, database metadata, or runtime hit
regions. Safe accessors return keys and phases, never mutable domain rows.

The target action family is typed and caller handled:

```rust
GridAction::Activate { cell: CellKey, origin }
GridAction::SortRequested { column: ColumnKey, direction: SortDirection }
GridAction::SelectionRequested(GridSelection)
GridAction::Edited { cell: CellKey }
GridAction::CopyRequested(GridSelection)
GridAction::FetchMore
GridAction::FilterRequested { column: ColumnKey, value: CellValue }
GridAction::Refresh
```

An action identifies a stable key. Display indices are never used as the
semantic target of an edit, selection, or activation.

## Update, draw, and measure

`update` is the only phase that consumes input and changes `GridState`. It
resolves runtime focus, hit, pointer capture, keyboard hover suppression,
selection, scrolling, editing, validation requests, and typed actions. It may
request a model operation but does not perform I/O.

`draw` is read-only with respect to state and model. It obtains fresh layout
facts, resolves the effective row/column projection, paints the declared parts,
and registers hit regions whose geometry is the same geometry painted. A first
click on a different cell only moves the cursor. A second click on the current
editable cell enters editing according to the baseline. Pointer-down alone is
not an edit.

`measure` computes column widths and required content size from the current
borrowed model and constraints. It may publish layout facts to the runtime but
cannot mutate durable selection, cursor, draft, or sort state. Position labels
and fetch sentinels consume the current measured facts, including after resize.

## Identity and reconciliation

- `RowKey` and `ColumnKey` survive reorder, filtering, insertion, and removal.
- Cursor, selection, pending edit, and validation error are reconciled by key;
  a stale key is cleared or reported, never clamped onto a different row.
- Sorting is a request unless the caller explicitly supplies a local sortable
  model. Local order is a projection over source keys and cannot rewrite the
  source.
- Source revision changes invalidate stale draft ranges. A pending edit may be
  committed only to the same cell key and compatible revision.
- Removal of an edited row or column cancels that draft with a typed outcome;
  it cannot retarget another visible row.
- The fetch-more sentinel has no data key and cannot be treated as a row.

## Interaction contract

### Navigation and selection

Plain Up/Down or `j`/`k` moves rows; Left/Right or `h`/`l` moves cells in cell
mode and pages the horizontal projection through the shared layout policy when
the presentation is row mode. Home/End address the current row, and
Ctrl+Home/Ctrl+End address the data extent. Page keys move by the measured
viewport. Shift extends a range where the selected mode permits it.

Space toggles the current stable row key. Escape clears selection. Enter
activates a read-only cell or starts the explicit editable flow. `y` and `Y`
request current-cell or selected-range copy; the copied representation is
source data, not gutter or truncation decoration.

### Editing

Editable cells use the shared text editing core. Enter/F2 begins editing only
when the cell is writable. Escape rolls back the draft. Enter or the configured
commit key validates and commits; invalid input keeps the draft and error on the
same stable cell. Tab commits and advances to the next writable stable column;
at the boundary it emits a leave action. Read-only columns, primary keys, and
deleted rows reject mutation. Boolean and viewer-style cells may use their
specialized action recipe while preserving the same typed cell identity.

### Pointer, focus, hover, and capture

Runtime owns focus, hit testing, and pointer capture. The painted cell, header,
scrollbar, and fetch sentinel hit regions share their exact rectangles.
Hover may underline an editable cell only while keyboard hover suppression is
off; pointer motion restores hover. Press, drag, and release outside are
captured by the originating scroll or selection owner. Grid itself is a focus
owner only when its presentation is interactive; decorative table use creates
no extra focus stop.

The scrollbar is the shared [`ScrollRegion`](./scroll-region.md) mechanism.
Vertical and horizontal scroll reach exact endpoints, preserve grab offset, and
consume boundary gestures according to the runtime routing contract.

## Visual contract

The future default theme reproduces the baseline cells, glyphs, spacing,
clipping, and color/capability behavior. Advertised parts are:

`container`, `header`, `row`, `gutter`, `cell`, `cursor`, `selection`,
`editor`, `sort-marker`, `footer`, `scrollbar`, `fade`, `empty`.

Part patches change only the declared part. They cannot replace layout,
focus/capture ownership, or the whole surface. Preserve the baseline row anatomy
including focus gutter, selection marker, pending-state glyphs (`•`, `+`, `−`,
`!`), current-cell reverse treatment, dirty/error tones, numeric alignment,
sort marker, empty/loading/error/readiness presentations, and the pending footer
where the consuming recipe includes it.

Display text handles wide graphemes, combining marks, tabs, controls, JSON, IDs,
null/default values, ragged rows, and narrow cells without splitting a terminal
cell or writing outside the allocation. Truecolor, 256-color, 16-color, and
no-color modes use the shared theme resolver. Fade leaves cursor, selected,
marked, and error emphasis readable. Where a baseline row/cell recipe has
activation feedback, its phase uses the shared 140 ms timing. Grid has no
generic whole-surface activation animation.

Preserve the database-grid recipe's header slots: `▪` for a primary-key
column, `∇` for an applied column filter, `▴`/`▾` for sort direction, and
`‹N`/`N›` for hidden columns. The row prefix distinguishes focus, selection,
pending change, and row number. `NULL` and `DEFAULT` are muted italic; empty
strings show faint `''`; changed values use warning tone; invalid values use
error plus `!`; references end in `→`; the current cell uses a reversed
white-on-canvas treatment (error background while invalid); range selection
uses the popover plane; queued deletions are faint and struck through. A
caller-supplied partial model may end with a `↓` fetch-more row, which has no
data identity. Pending transaction bars and domain-specific column glyphs are
caller composition, documented for TablePro in its [application contract](../applications/tablepro.md).

In Table row mode, header activation cycles ascending → descending → none and
sorting permutes a stable-key projection so selection and edits survive. For
the frozen data-grid recipe, estimate widths from the 95th percentile of the
first 200 rows, clamp by column type, never go below the header width, and
clip the final visible column rather than leaving blank space. Horizontal and
vertical wheel input both move their respective bounded viewport without
moving the semantic cursor.

## Applicable state matrix

| Axis | Required states | Applies when |
|---|---|---|
| Geometry | `normal`, `zero_area`, `tiny_area`, `nonzero_origin`, `exact_fit`, `one_cell_short`, `unicode_long_content`, `narrow_then_wide` | Always |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover | Interactive presentation or cell/header owner |
| Activation | No generic whole-surface activation; cell, row, sort, copy, and fetch actions are typed | Child/grid recipe only |
| Editing | navigation, editing, selected text, invalid, read-only, commit, cancel, blur, tab traversal, source revision conflict | Editable presentation |
| Source | empty, ready, selected differs from cursor, disabled entry, reorder, insert, remove, filter, stale target | Model provides the state |
| Readiness | loading, partial, error, retry/refresh when offered | Caller supplies readiness |
| Scroll | no overflow, start, middle, end, wheel boundary, thumb drag, resize while scrolled, edge fade, protected row | Overflow exists; shared ScrollRegion applies |
| Motion | every applicable activation phase, paused/reduced motion | A recipe advertises feedback |

Decorative or unavailable states are marked not applicable. They are never
fabricated to fill a matrix.

## Capture and acceptance contract

Use [`../reference/capture-plans/grid.json`](../reference/capture-plans/grid.json). It is a planned
capture manifest with no candidate-approved expected artifacts. Capture trusted
output from the pinned baseline first, then classify evidence as
`ExistingOracle`, `ExtractedOracle`, or `Extension` before candidate testing.

Run each required case at dimensions `72×20`, `80×24`, `100×30`, `120×40`, and
`160×50`, with truecolor, 256-color, 16-color, `none`, and `nocolor`
capabilities where the case applies.

| Case | Required observation |
|---|---|
| W28-01 | Row-table and rich cell-grid presentations remain baseline-identical. |
| W28-02 | First click on a new cell selects; second click on the current editable cell edits. |
| W28-03 | Read-only activation cannot mutate the model; include a compile-fail consumer. |
| W28-04 | Valid/invalid commit, Escape rollback, and Tab traversal preserve the cell key. |
| W28-05 | Sort, reorder, removal, and column removal cannot retarget a draft. |
| W28-06 | Empty, ragged, and large models; horizontal/vertical scroll; fetch-more sentinel. |
| W28-07 | Header/cell/row patches affect only declared actual cells. |

Every capture records dimensions, symbols, wide-cell continuation, foreground,
background, supported modifiers, cursor position/visibility, focus and capture
owners, selected stable key, navigation key, draft and committed state, action
count, and action target. Invalid oracle setup is a blocked capture, never a
passing result.

## Required negative tests

- A `&dyn GridModel` consumer cannot call editor mutation or commit methods.
- `draw` cannot change cursor, selection, sort, draft, model values, or action
  count.
- Reorder/remove/column removal during editing cannot apply a draft to a new
  key.
- A pointer release outside the grid cannot leave capture or a pressed cell.
- Rags, empty data, zero/tiny allocations, wide glyphs, and long JSON cannot
  panic or write outside the buffer.
- Duplicate ids, stale source revisions, and unavailable rows fail closed.
- A custom part painter cannot replace focus, capture, layout, or another part.

## Foundation dependencies

- [`../foundations/identity.md`](../foundations/identity.md): stable semantic
  IDs, RowKey, ColumnKey, and CellKey.
- [`../foundations/input-actions.md`](../foundations/input-actions.md): keys,
  pointer phases, typed responses, and copy requests.
- [`../foundations/runtime.md`](../foundations/runtime.md): focus, hit,
  capture, time, and hover suppression.
- [`../foundations/layout.md`](../foundations/layout.md): measurement,
  clipping, wide cells, and layout facts.
- [`../foundations/theme.md`](../foundations/theme.md): semantic parts,
  capability resolution, and fade/motion policy.
- [`../foundations/text.md`](../foundations/text.md): grapheme-safe editing,
  display mapping, and source-preserving copy.
- [`../foundations/collections.md`](../foundations/collections.md): keyed
  reconciliation and source revision handling.
- [`../foundations/author.md`](../foundations/author.md): constrained parts and
  borrowed props.
- [`../foundations/conformance.md`](../foundations/conformance.md): oracle
  binding, snapshots, traces, and negative gates.

Grid acceptance requires a compiling external consumer, source-state tests,
exact applicable snapshots, and independent review of the evidence. A painted
placeholder or a custom call that is later overwritten fails acceptance.
