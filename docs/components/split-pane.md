# SplitPane

**Component ID:** W33 · **Phase:** P2 · **Legacy family:** C42
**Oracle:** `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`

SplitPane is one axis-parameterized two-pane allocation and seam interaction
mechanism. It belongs to the Termrock library refactor in this repository. The
current layout and splitter source remain frozen oracle evidence.

## Purpose and boundary

SplitPane allocates first and second caller-supplied panes along a horizontal or
vertical axis, paints/registers the seam, and emits typed resize/maximize
actions. The caller supplies pane content and decides what each pane means.

It owns no PTYs, processes, repositories, terminals, session routing, or child
trees. It must not have separate horizontal and vertical engines; axis is a
parameter of one mechanism.

### Non-goals

- process or terminal lifecycle;
- application-specific pane routing or product behavior;
- changing a preferred ratio merely because the current terminal is too small;
- negative/wrapped pane geometry below combined minima;
- a universal widget trait or current `WidgetId`/`Outcome`/`RenderCtx` API.

## Frozen evidence and provenance

All parity is pinned to `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`.

- Source: `src/widgets/splitter.rs` (blob
  `80c113f7585827d340da45921011c2a1953a8b34`) and split allocation helpers in
  `src/ui/layout.rs`.
- Inline layout tests cover minima, maximize, drag clamping, nudge, and
  directional allocation. Splitter source covers seam registration,
  hover/pressed glyphs, and drag dispatch.
- Visual consumers: Jackin Preview capsule and accounts compositions, plus
  protected snapshots under `snapshots/jackin/capsule` and
  `snapshots/jackin/accounts`. Interaction traces remain under
  `tests/visual_baseline`.

See [`../verification/visual-parity.md`](../verification/visual-parity.md),
[`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md),
and [`../api/public-api.md`](../api/public-api.md).

## Target public API

```rust
SplitPane::new(id: Id, axis: Axis) -> SplitPane

update(
    &self,
    cx: &mut Cx<'_>,
    state: &mut SplitPaneState,
) -> Response<SplitAction>

draw<R>(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    state: &SplitPaneState,
    body: impl FnOnce(&mut Ui<'_>, SplitAreas) -> R,
) -> R

measure(
    &self,
    cx: &MeasureCx<'_>,
    constraints: Constraints,
) -> Size
```

Configuration:

```rust
minima(first: u16, second: u16)
seam_width(u16)
resizable(bool)
patch(StylePatch)
```

`SplitPaneState` owns desired ratio/fixed allocation and optional maximized
child `ItemKey`. Runtime owns pointer-grab offset, capture, and hit geometry.
The typed actions are:

```rust
SplitAction::Resized { ratio: SplitRatio }
SplitAction::Maximized { child: Option<ItemKey> }
```

The caller decides whether to persist or reconcile those preferences.

## Update, draw, and measure

`update` handles focus, seam pointer down/drag/release, keyboard increments,
zoom/maximize commands, and typed actions. Pointer capture keeps the initial
grab offset under the pointer and release outside ends the drag safely.

`draw` derives clamped displayed allocations from current area, axis, seam, and
minima, paints/registers the same seam rectangle, and invokes each nonempty
body exactly once. It cannot overwrite the preferred ratio or mutate durable
state. Below combined minima it uses the explicit responsive/drawer policy;
there is no negative or wrapped dimension.

`measure` computes pane rectangles and seam from constraints without changing
ratio, maximized key, or body state. Narrowing clamps the display allocation;
widening restores the preferred ratio. Nested splits reconcile removed or
maximized children by stable key.

## Interaction contract

Horizontal and vertical axes use identical rules. The seam is the only pointer
resize target. A pointer press on the seam enters runtime capture; drag follows
the pointer with its original offset, clamps both minima, and emits one resize
action per effective ratio change. Pressing first/last seam cells and dragging
outside are required cases.

Keyboard resize nudges the same preferred ratio used by pointer resize. Zoom or
maximize selects a stable child key; toggling the same key restores both panes.
Removing a maximized child clears the stale key and returns to the valid
nonmaximized composition according to caller policy.

Focus/hover styling belongs to the seam runtime owner. Keyboard suppression and
pointer-motion restoration follow shared runtime rules. SplitPane has no generic
activation animation; pressed/dragging is a seam interaction state.

## Visual contract

Advertised parts are `container`, `first-body`, `seam`, and `second-body`.

The baseline seam glyph is `│` or `─` when idle/hovered according to axis and
`┃` or `━` while pressed/dragged. Hover and pressed semantic border tones must
survive monochrome capabilities. The seam is quiet when idle and visibly strong
while captured. Empty seam (gap zero or maximized pane) paints/registers
nothing.

Pane body clipping, nonzero origins, minima, and allocation pixels remain
baseline exact. Part patches cannot replace layout, focus/capture, or the whole
surface. Colors resolve through shared theme capability policy; motion is only
the shared press/resize timing if a recipe advertises it.

## Identity and reconciliation

- The SplitPane identity and each child `ItemKey` are stable semantic IDs.
- Maximized state is keyed, not a display index.
- A child removal while maximized clears the stale key rather than granting the
  other child an accidental maximized identity.
- Preferred ratio is durable; displayed clamp is derived from current geometry.
- Nested splits use the same identity and ratio reconciliation rules.

## State matrix

| Axis | Required states | Applies when |
|---|---|---|
| Geometry | `normal`, `zero_area`, `tiny_area`, `nonzero_origin`, `exact_fit`, `one_cell_short`, `unicode_long_content`, `narrow_then_wide` | Always |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover | Seam or child interactive |
| Activation | No generic whole-surface activation; resize and maximize are typed | Split recipe |
| Pointer | seam press, held drag, release inside, release outside, first/last cell | Resizable |
| Resize | keyboard nudge, ratio clamp, minima, below-minimum responsive policy, restore after widen | Resizable |
| Maximization | none, first, second, toggle restore, removed maximized child | Zoom policy |
| Motion | applicable pressed/drag phase, paused/reduced motion | Theme policy |

Generic content editing, selection, and scroll states belong to child panes.

## Capture and acceptance contract

Use [`../reference/capture-plans/split-pane.json`](../reference/capture-plans/split-pane.json). It is
planned with no expected artifacts. Bind the frozen source as
`ExistingOracle`/`ExtractedOracle`; API/safety extensions are explicit
`Extension` cases.

Run applicable cases at `72×20`, `80×24`, `100×30`, `120×40`, and `160×50`,
with truecolor, 256-color, 16-color, `none`, and `nocolor` capabilities.

| Case | Required observation |
|---|---|
| W33-01 | Horizontal and vertical allocations use identical rules. |
| W33-02 | Seam drag at first/last cell and resize during capture. |
| W33-03 | Narrow below minima then widen restores preferred ratio. |
| W33-04 | Keyboard increments and maximize/unmaximize share state. |
| W33-05 | Nested panes with one child removed while maximized. |

Record seam cells, pane dimensions, ratio/preferred ratio, min clamps,
focus/capture/layer owners, action count and action target, capability, and
release result. Compare symbols, continuations, colors/modifiers, and exact
dimensions.

## Required negative tests

- The painted seam and registered hit rectangle must be identical.
- Pointer release outside cannot leave capture or pressed seam state.
- Drag cannot violate minima or overwrite the preferred ratio with a temporary
  narrow allocation.
- Below combined minima cannot create negative, wrapped, or overlapping panes.
- Keyboard resize and pointer resize cannot diverge in ratio semantics.
- Removing a maximized child cannot retarget maximization to another key.
- `draw`/`measure` cannot mutate ratio, maximized key, or child state.
- Part customization cannot replace layout, seam hit geometry, or capture.

## Foundation dependencies

- [`../foundations/identity.md`](../foundations/identity.md): SplitPane and
  stable child ItemKey identity.
- [`../foundations/input-actions.md`](../foundations/input-actions.md): key,
  pointer, drag, and typed resize/maximize actions.
- [`../foundations/runtime.md`](../foundations/runtime.md): focus, hit,
  pointer capture, and hover/press phases.
- [`../foundations/layers.md`](../foundations/layers.md): nested panes and
  modal/overlay composition.
- [`../foundations/layout.md`](../foundations/layout.md): axis allocation,
  minima, seam, clipping, and responsive policy.
- [`../foundations/theme.md`](../foundations/theme.md): seam parts, semantic
  border tones, capability, and motion.
- [`../foundations/collections.md`](../foundations/collections.md): stable
  child reconciliation in nested compositions.
- [`../foundations/author.md`](../foundations/author.md): constrained pane body
  slots and callback ownership.
- [`../foundations/conformance.md`](../foundations/conformance.md): exact
  snapshots, interaction traces, and negative gates.

Acceptance requires an external two-pane consumer, exact applicable snapshots,
interaction traces, source-state tests, and independent review.
