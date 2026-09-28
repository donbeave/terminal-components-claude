# TextViewport

**Component ID:** W31 · **Phase:** P5 · **Legacy families:** C39, C40
**Oracle:** `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`

This is the canonical Termrock contract for a selectable, scrollable text
projection. The future component is a borrowed view over caller-owned text. The
frozen `src/widgets/viewport.rs` and `src/widgets/panel.rs` implementation is
oracle evidence, not the target ownership model.

## Purpose and boundary

TextViewport projects revisioned logical lines and styled spans into terminal
cells. It supports prose and log recipes, wrapping, selection, copying, marks,
caret display, scrollbar interaction, and optional tail following. `TextSource`
provides borrowed lines, stable line identities, source revision, and any
caller-defined retention/eviction facts.

The caller owns the retained output, append/replace/evict policy, source bytes,
search service, and clipboard. TextViewport owns only view state and typed
requests. It never becomes a terminal emulator, PTY, shell, escape parser, or
session manager; [`TerminalView`](./terminal-view.md) is the separate prepared
cell consumer for terminal presentation.

### Non-goals

- retaining the complete application output buffer in the component;
- silently changing caller retention or rewriting source lines;
- treating prose `End` as tail follow;
- converting display tabs/control stand-ins back into copied source text;
- adding a second scroll, fade, text segmentation, or selection engine.

## Frozen evidence and provenance

All parity claims bind to `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`.

- Source: `src/widgets/viewport.rs` (blob
  `da21128c721fc9d748386441f12aac2b9647f8c8`) and the baseline panel
  composition in `src/widgets/panel.rs` (blob
  `14a9edef27f16e463a635f314fa37d5825f57fd9`).
- Inline tests cover tail follow, drag and keyboard selection, source-preserving
  copy, wrapping, retention, stable identity after eviction/replacement,
  Unicode graphemes, tabs/control display, marks, incremental layout, and
  nonzero-origin clipping.
- Visual consumers: Showcase scrolling and terminal pages plus Jackin Preview
  cockpit output. Discovery roots include
  `snapshots/showcase/pages/scrolling`, `snapshots/showcase/pages/terminal`,
  and `snapshots/jackin/cockpit`; protected conformance traces are under
  `tests/visual_baseline`.

See [`../verification/visual-parity.md`](../verification/visual-parity.md),
[`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md),
and [`../api/public-api.md`](../api/public-api.md).

## Target public API

```rust
TextViewport::new(id: Id, source: &'a dyn TextSource) -> TextViewport<'a>

update(
    &self,
    cx: &mut Cx<'_>,
    state: &mut ViewportState,
) -> Response<ViewportAction>

draw(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    state: &ViewportState,
) -> Rect

measure(
    &self,
    cx: &MeasureCx<'_>,
    constraints: Constraints,
) -> Size
```

Configuration is borrowed and constrained:

```rust
mode(ViewportMode::Prose | ViewportMode::Log)
wrap(WrapMode)
selectable(bool)
marks(&'a [TextMark])
patch(StylePatch)
```

`TextSource` supplies stable `LineKey`s, spans or source text, revision, and
retention notifications. The component may query display projection facts but
does not own source storage.

### Durable state and typed actions

`ViewportState` owns:

- shared [`ScrollState`](./scroll-region.md) and follow mode;
- optional source-keyed caret and selection anchors;
- marks or current mark identity when configured;
- the retained reading anchor while follow is off;
- only projection/cache metadata that can be rebuilt from the source revision.

The target action family is:

```rust
ViewportAction::CopyRequested(TextSelection)
ViewportAction::SelectionChanged
ViewportAction::FollowChanged(bool)
ViewportAction::MarkChanged { key: ItemKey }
```

`CopyRequested` carries source ranges, not painted strings. The caller resolves
those ranges to clipboard text, preserving tabs, controls, trailing source
semantics, and line endings according to the API contract.

## Update, draw, and measure

`update` handles input, focus, selection, scroll, follow, pointer drag, and
typed copy/follow actions. It reconciles all anchors against source revision
and stable line keys before applying input. A keyboard selection pauses follow;
scrolling to the real tail in Log mode resumes it.

`draw` is immutable over state and source. It builds or reuses a projection for
the measured width, paints visible rows, cursor/selection/marks, fades, and the
shared scrollbar, and registers the exact hit rectangles. It cannot move the
offset, clear a selection, or toggle follow as a side effect of painting.

`measure` computes wrapped visual rows, content extent, visible range, and
position facts for the current width and capabilities. The panel/title badge
may consume the result after measurement; it must not display stale first-frame
counts. Reflow on resize preserves a stable reading anchor where possible.

## Follow, retention, and reconciliation

Log mode follows only while the viewport is at the real tail and follow is
enabled. Appending while following keeps the end visible. Scrolling away,
selecting, or dragging into history enters retained reading mode. Returning to
the end by wheel, key, or thumb resumes follow. Prose mode never follows merely
because `End` was pressed; `End` is a jump.

Append, in-place tail replacement, source replacement, and eviction are distinct
operations:

- append extends the visual index without re-segmenting unchanged lines;
- replacing a line keeps its `LineKey`, invalidates its projection, and clips
  selection/caret to the new width;
- replacing the source revision resets anchors whose identity does not survive;
- evicting a selected first line clips to the first retained key; evicting both
  selection endpoints clears the selection;
- an evicted drag anchor is rebound to the first retained line, never to an
  unrelated index;
- an evicted producer caret is cleared, and an evicted reading anchor moves to
  the first retained key by explicit policy;
- marks reconcile by key/range and never silently move to a newly appended
  line.

## Interaction contract

### Keyboard and pointer selection

Up/Down or `j`/`k` scrolls one visual row; PageUp/PageDown scrolls a viewport;
Home/`g` goes to the start; End/`G` goes to the end and enables follow in Log
mode; `f` toggles follow where offered; `y` requests copy; Escape clears an
active selection. Shift+arrows/Home/End extend keyboard selection. Ctrl/Alt
with Shift selects by word where supported.

Pointer-down anchors a drag without creating a selection. Drag extends source
selection and auto-scrolls at vertical edges. A double click selects a word
using grapheme-safe source boundaries. Pointer capture belongs to the viewport
until release, including release outside. Scrollbar thumb capture preserves the
grab offset and reaches exact endpoints.

Focus is runtime-owned. A viewport used as a read-only output surface may be a
focus stop only when it has no focusable child; it does not create a decorative
focus stop inside a containing Panel. Keyboard hover suppression and pointer
motion behavior follow the shared runtime contract.

## Text, Unicode, and capability rules

Segment each logical line as one grapheme stream before applying span styles.
A cluster crossing span boundaries takes the style of its first byte. Tabs
expand to four display cells; C0 controls use visible stand-ins. Copy resolves
the original source byte range, so tabs and controls remain source bytes. Wide
graphemes, combining marks, flags, ZWJ sequences, and clipped rows never split
terminal cells. Zero and tiny allocations emit safely without out-of-bounds
writes.

The default theme preserves baseline text hierarchy, current/selection reverse
style, marks, cursor, edge fade, scrollbar glyphs, clipping, and empty state.
Truecolor mixes the edge fade toward the majority background; 256-color,
16-color, and no-color modes use the shared dim policy. Fade protects the
cursor row, selected/reversed cells, marks, and any caller-declared protected
row. Motion uses shared timing and reduced-motion policy; no generic viewport
activation is invented.

When a log is scrolled away from its tail, its owner metadata shows the
scrollback cue `▲ N`. Prose recipes wrap; terminal-pane recipes do not. Log
history is bounded by caller-selected `max_lines` retention and cannot grow
without limit. These recipe choices do not change the shared projection,
selection, copy, scrollbar, or fade mechanisms.

## Visual parts and composition

Advertised parts are `container`, `text`, `selection`, `cursor`, `mark`,
`prefix`, `scrollbar`, `fade`, and `empty`. Part patches affect only the
declared part. Geometry, focus/capture ownership, source retention, and whole
surface painting are not replaceable slots.

[`Panel`](./panel.md) plus TextViewport is the replacement for the old
ScrollPanel mechanism. Panel owns chrome and clipped body composition;
TextViewport owns text projection, selection, and scroll state. Dynamic position
metadata is derived from fresh layout facts and painted after the body when the
recipe requires it.

## Applicable state matrix

| Axis | Required states | Applies when |
|---|---|---|
| Geometry | `normal`, `zero_area`, `tiny_area`, `nonzero_origin`, `exact_fit`, `one_cell_short`, `unicode_long_content`, `narrow_then_wide` | Always |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover | Interactive owner or selection surface |
| Activation | No generic whole-surface activation; selection, copy, and scroll gestures are typed | Not applicable at viewport surface |
| Scroll | no overflow, start, middle, end, wheel boundary, thumb drag, resize while scrolled, edge fade, protected row | Overflow exists |
| Output | source replaced, source appended, selection, source copy, cursor clipped | Source/selection configured |
| Retention | history evicted, evicted selection, retained reading anchor | Source evicts history |
| Motion | all applicable fade/cursor phases, paused/reduced motion | Theme advertises motion |

No activation or editing state is fabricated for this read-only surface.

## Capture and acceptance contract

Use [`../reference/capture-plans/text-viewport.json`](../reference/capture-plans/text-viewport.json).
The manifest is `planned_not_captured` and contains no approved expected
artifacts. Bind baseline output first as `ExistingOracle` or `ExtractedOracle`;
new safety/API behavior is `Extension`.

Run applicable cases at `72×20`, `80×24`, `100×30`, `120×40`, and `160×50`,
across truecolor, 256-color, 16-color, `none`, and `nocolor` capabilities.

| Case | Required observation |
|---|---|
| W31-01 | Log append while following versus while reading history. |
| W31-02 | Prose `End` remains non-following. |
| W31-03 | Evict first/middle selected line and replace a source revision. |
| W31-04 | Keyboard/drag selection, marks, and copy across styled spans. |
| W31-05 | Tabs and control display preserve original copied text. |
| W31-06 | Position badge is fresh on first draw and after resize. |
| W31-07 | Scrollbar endpoints, edge fade, and protected caret row. |

Every case records exact source keys and display cells, cursor position and
visibility, focus/capture/layer owners, offset/follow/selection/mark state,
action count and typed action target. Draw must not mutate semantic state.

## Required negative tests

- `draw` cannot toggle follow, move offset, change selection, mutate marks, or
  alter source revision.
- An evicted selection/caret/drag anchor cannot silently retarget another line.
- A source replacement cannot reuse stale byte offsets or cached highlight rows.
- Copy from a tab/control/wide grapheme returns source text, not display glyphs.
- Zero/tiny/nonzero-origin areas and narrow clipping cannot panic or write out of
  bounds.
- A scrollbar press/release outside leaves no stale pointer capture.
- A part override cannot replace the projection, fade protection, or hit
  geometry.
- Cache reuse must be invalidated on width, wrap, revision, append, replace,
  eviction, or mark changes according to the declared cache policy.

## Foundation dependencies

- [`../foundations/identity.md`](../foundations/identity.md): stable LineKey,
  ItemKey, and source identity.
- [`../foundations/input-actions.md`](../foundations/input-actions.md): key,
  wheel, drag, copy, and typed response semantics.
- [`../foundations/runtime.md`](../foundations/runtime.md): focus, hit,
  capture, cursor, time, and hover suppression.
- [`../foundations/layers.md`](../foundations/layers.md): viewport ownership
  under overlays and clipping order.
- [`../foundations/layout.md`](../foundations/layout.md): wrapped rows,
  measurement, cell continuation, and resize facts.
- [`../foundations/theme.md`](../foundations/theme.md): semantic text parts,
  fade, cursor, color capability, and motion.
- [`../foundations/text.md`](../foundations/text.md): grapheme segmentation,
  source/display mapping, selection, and copy.
- [`../foundations/collections.md`](../foundations/collections.md): revision,
  retention, and identity reconciliation.
- [`../foundations/conformance.md`](../foundations/conformance.md): exact
  snapshot/trace comparison and oracle authority.

Acceptance requires a compiling external consumer, source-state tests, exact
applicable snapshots, and independent review. Painted placeholders and
candidate-controlled expected output are failures.
