# DiffView

**Component ID:** W30 · **Phase:** P5 · **Legacy family:** C38
**Oracle:** `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`

DiffView is the canonical read-only presentation of caller-provided diff rows.
It is part of the in-place Termrock refactor. The current source and snapshots
remain immutable evidence.

## Purpose and boundary

DiffView consumes already computed hunks, line kinds, original source ranges,
file status, and path metadata through `DiffSource`. It renders a unified
listing or a side-by-side review presentation, with shared TextViewport
selection, scrolling, copy, and Unicode projection.

Computing Git diffs, reading repositories, accepting patches, mutating files,
or deciding review policy are caller responsibilities. DiffView emits typed
selection, copy, and hunk activation requests. It does not perform those
operations.

### Non-goals

- Git/patch generation, repository access, or file mutation;
- a second text, scroll, selection, or grapheme projection engine;
- silently replacing requested Review mode with a different durable mode;
- treating an empty diff as a fabricated success screen;
- a universal widget trait or legacy `WidgetId`/`Outcome`/`RenderCtx` API.

## Frozen evidence and provenance

All required parity is pinned to `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`.

- Source: `src/widgets/diff.rs` (blob
  `e0b5bdefa3b7fa514295a245de80ebf0f3bcf438`) and the Jackin Preview inspect
  composition in `src/bin/jackin_preview/screens/inspect.rs`.
- Inline tests cover unified/review lines, hunk/header counts, narrow fallback,
  resize recovery, scrollbar-aware width, Unicode intraline emphasis, tabs,
  selection, and raw-source copy.
- Visual consumers: Showcase audit/diff flows and Jackin Preview inspect views;
  discovery roots are `snapshots/showcase/audit` and `snapshots/jackin` with
  interaction traces in `tests/visual_baseline`.

See [`../verification/visual-parity.md`](../verification/visual-parity.md),
[`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md),
and [`../api/public-api.md`](../api/public-api.md).

## Target public API

```rust
DiffView::new(id: Id, source: &'a dyn DiffSource) -> DiffView<'a>

update(
    &self,
    cx: &mut Cx<'_>,
    state: &mut DiffViewState,
) -> Response<DiffAction>

draw(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    state: &DiffViewState,
) -> Rect

measure(
    &self,
    cx: &MeasureCx<'_>,
    constraints: Constraints,
) -> Size
```

Builders:

```rust
mode(DiffMode::Unified | DiffMode::Review)
wrap(WrapMode)
patch(StylePatch)
```

The source model supplies stable hunk/line keys, original text/ranges, line
kind (`Context`, `Add`, `Remove`), file status, and source revision. The target
actions are:

```rust
DiffAction::SelectionChanged
DiffAction::CopyRequested(TextSelection)
DiffAction::HunkActivated { key: ItemKey }
```

Copy resolves original source content according to the selected side and mode;
it never returns decorated gutters, diff markers, separator glyphs, or tab
expansion.

### Durable state

`DiffViewState` owns requested `DiffMode`, shared `ViewportState`, keyed hunk
and line navigation, selection, and any source revision projection marker.
Effective narrow fallback is derived from available width and scrollbar budget;
it does not overwrite requested Review mode. No `DiffFile`, source rows, or
repository state is retained as mutable domain data.

## Update, draw, and measure

`update` routes mode toggle, selection, copy, hunk activation, scrolling,
keyboard, pointer drag, focus, and capture. Shared TextViewport mechanics own
selection/copy/scroll behavior. Source revision changes invalidate generated
display lines and stale selections by stable key.

`draw` is immutable. It computes current available width, reserves scrollbar
space before choosing Review, projects the source into styled lines, then
delegates visible text, selection, cursor (if configured), fade, and scrollbar
painting to the shared viewport. It must not toggle mode or alter offset as a
render side effect.

`measure` computes the file header, hunk/line projection, gutters, separator,
and width threshold. The Review breakpoint includes both side gutters, minimum
content cells, separator, and scrollbar width. A resize from wide to narrow
draws the unified fallback; widening restores requested Review automatically.

## Presentation and behavior

### Unified mode

Unified mode renders file/status header, hunk header, old/new line-number
gutter, context lines, additions, and deletions with baseline markers and tones.
An empty file uses the shared Empty/readiness presentation and `(no textual
changes)` semantics from the oracle.

### Review mode

Review mode pairs old and new columns, leaves a blank side for unpaired lines,
keeps a visible separator, and bolds changed grapheme runs. Tabs expand through
the shared display mapping before width and emphasis calculations. Wide
graphemes and combining marks remain complete on both sides.

The requested mode survives all resizes. Widths around the exact fallback
threshold are tested with and without the scrollbar. A narrow terminal never
produces wrapped or negative pane geometry.

### Input, pointer, and focus

Keyboard and pointer selection, wheel, page movement, Home/End, drag selection,
scrollbar capture, copy, and Escape selection clearing follow
[`TextViewport`](./text-viewport.md). Hunk activation is keyed and typed. Click
and keyboard selection produce the same source range. Pointer release outside
ends capture safely.

DiffView does not invent a generic activation animation. Focus and hover belong
to the runtime/viewport owner; keyboard hover suppression and pointer restoration
follow the shared runtime contract. It is read-only, so editing states do not
apply.

## Visual contract

Advertised parts are `container`, `header`, `old-gutter`, `new-gutter`,
`context`, `addition`, `deletion`, `emphasis`, `selection`, `scrollbar`,
`fade`, and `empty`.

Part style patches alter only declared rows/cells. Geometry, source mapping,
focus/capture, and whole-surface ownership are not slots. Preserve baseline
header text, line-number spacing, `+`/`-` markers, context tones, added/deleted
surfaces, intraline bolding, separator, empty message, scrollbars and fades.
Default semantic colors resolve through the shared truecolor/256/16/no-color
policy. Fades protect selected or otherwise emphasised cells and cursor rows.

## Identity and reconciliation

- Hunk and line identities are stable semantic keys from `DiffSource`.
- Selection and current line reconcile by key across source refreshes and mode
  projection; removed keys clear instead of clamping to another row.
- Mode changes may rebuild display lines but preserve requested mode, valid
  selection, and scroll intent where source keys still exist.
- A changed source revision invalidates stale byte ranges, display caches, and
  copy mappings before update or draw.
- Intraline emphasis maps original grapheme ranges to display cells; it cannot
  change source text or selection boundaries.

## State matrix

| Axis | Required states | Applies when |
|---|---|---|
| Geometry | `normal`, `zero_area`, `tiny_area`, `nonzero_origin`, `exact_fit`, `one_cell_short`, `unicode_long_content`, `narrow_then_wide` | Always |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover | Interactive selection surface |
| Activation | No generic whole-surface activation; hunk activation and copy/selection are typed | Diff recipe |
| Scroll | no overflow, start, middle, end, wheel boundary, thumb drag, resize while scrolled, edge fade, protected row | Overflow exists |
| Output | source replaced, source appended, selection, source copy, cursor clipped | Source/selection configured |
| Review mode | requested Unified, requested Review, effective unified fallback, restored Review | Review is requested |
| Empty/readiness | empty/no-change, loading/error when supplied | Source supplies state |

Editing, disabled, and generic whole-surface activation states are not
applicable and must not be fabricated.

## Capture and acceptance contract

Use [`../reference/capture-plans/diff-view.json`](../reference/capture-plans/diff-view.json). It is
`planned_not_captured` with no approved expected artifacts. Baseline output is
bound as `ExistingOracle`/`ExtractedOracle`; new API/safety guarantees are
`Extension`.

Run applicable cases at `72×20`, `80×24`, `100×30`, `120×40`, and `160×50`,
with truecolor, 256-color, 16-color, `none`, and `nocolor` capabilities.

| Case | Required observation |
|---|---|
| W30-01 | Unified/review with additions, deletions, context, and intraline emphasis. |
| W30-02 | Wide → narrow → wide restores requested Review. |
| W30-03 | Exact width around the split/scrollbar threshold. |
| W30-04 | Selection crosses tabs, wide graphemes, and hunk boundaries. |
| W30-05 | Empty/no-change and source revision replacement. |
| W30-06 | Click/keyboard selection equivalence and raw-source copy. |

Record exact symbols, wide-cell continuation, colors/modifiers, dimensions,
cursor/focus/capture/layer owners, selected stable key, source revision,
requested/effective mode, offset, action count and typed action target. Invalid
oracle setup blocks capture.

## Required negative tests

- `draw` cannot change requested/effective mode, scroll offset, selection, or
  source revision.
- Review fallback cannot overwrite durable requested Review mode.
- Scrollbar width is included in breakpoint calculation; no negative or wrapped
  side geometry is possible.
- Copy cannot return gutter markers, separators, expanded tabs, or unrelated
  source text.
- Source refresh cannot apply stale selection/ranges to a different hunk/line.
- Unicode emphasis and selection cannot split graphemes or continuation cells.
- Empty diff cannot fabricate a success state.
- Part overrides cannot replace source mapping, scrollbar, focus, or capture.

## Foundation dependencies

- [`../foundations/identity.md`](../foundations/identity.md): stable hunk/line
  and source keys.
- [`../foundations/input-actions.md`](../foundations/input-actions.md): typed
  copy, selection, activation, key, wheel, and drag responses.
- [`../foundations/runtime.md`](../foundations/runtime.md): focus, hit,
  capture, cursor, and hover suppression.
- [`../foundations/layers.md`](../foundations/layers.md): composition under
  overlays and source-owned focus.
- [`../foundations/layout.md`](../foundations/layout.md): breakpoint,
  scrollbar budget, clipping, and measurement.
- [`../foundations/theme.md`](../foundations/theme.md): semantic diff tones,
  emphasis, fade, capability, and motion.
- [`../foundations/text.md`](../foundations/text.md): shared viewport,
  grapheme mapping, selection, and source-preserving copy.
- [`../foundations/collections.md`](../foundations/collections.md): source
  revision and keyed reconciliation.
- [`../foundations/conformance.md`](../foundations/conformance.md): exact
  snapshots, traces, oracle lanes, and negative gates.

Acceptance requires a compiling external consumer, source-state tests, exact
applicable snapshots/traces, and independent review.
