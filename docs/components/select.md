# Select

**Component ID:** W10\
**Group:** Forms\
**Phase:** P3\
**Contract status:** canonical Termrock target; implementation is future work on `termrock-refactor`.

## Purpose and scope

`Select` is a controlled field with a runtime-owned popup list. The caller supplies keyed options and committed choice. Termrock owns the open/highlight/scroll interaction state, popup placement, dismissal, and focus restoration. Choosing is explicit; merely highlighting an option never commits it.

### Non-goals

- No option persistence, domain lookup, query parser, or product-specific filtering policy.
- No orphan overlays when an anchor is removed or hidden.
- No second popup, list, focus, or scroll engine.

## Oracle and provenance

The immutable source/output oracle is [`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`](https://github.com/donbeave/terminal-components-claude/commit/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b). Current implementation is [`src/widgets/select.rs`](../../src/widgets/select.rs), legacy `Select`, source blob `3b6f43f8f642ccce512ac53754d114dc978bd149`. Showcase select pages and forms are reference consumers; they remain untouched.

The capture contract is [`capture-plans/select.json`](../reference/capture-plans/select.json), W10, planned and with no approved artifacts. Bind ExistingOracle/ExtractedOracle first.

## Public API and ownership

```rust
Select::new(
    id: Id,
    options: &'a [ChoiceItem<'a>],
    revision: Revision,
) -> Select<'a>

select.update(&mut cx, &mut state) -> Response<SelectAction>
select.draw(&mut ui, area, &state) -> Rect
select.measure(&measure_cx, constraints) -> Size
```

Builders: `selected(Option<ItemKey>)`, `label(&'a str)`, `placeholder(&'a str)`, `disabled(bool)`, `validation(Option<&'a ValidationMessage>)`, and `patch(StylePatch)`.

`SelectState` owns open/closed status, highlighted key, optional query, and popup `ScrollState`; it never owns committed choice. Actions are `SelectAction::Choose { key, origin: ActivationOrigin }` and `SelectAction::Dismissed`.

## Update, draw, and measure

- `update` opens from the field, navigates stable keys, chooses on Enter/Space or eligible row click, and dismisses on Escape/outside. Dismissal restores the previous controlled choice and parent focus.
- `draw` paints the Field chrome and popup as real layered rows. It never commits a choice, mutates semantic state, or paints decorative substitutes for children.
- `measure` reports closed field geometry and popup constraints separately; placement flips/clamps at screen edges and re-anchors after resize.

## Identity, source reconciliation, and layers

The field `Id` and option `ItemKey`s are stable semantic identities. Reorder/removal while open preserves a surviving highlight or resolves it to a valid neighbor. A stale pointer row cannot choose a replacement. If the anchor disappears, runtime closes or safely reanchors the layer and restores focus without leaving an orphan capture. Parent focus/capture owner and layer owner are recorded in conformance traces.

## Visual contract

Parts are `container`, `label`, `gutter`, `value`, `disclosure`, `popup`, `row`, `marker`, `help`, and `error`. Preserve the baseline field three-row layout, selected marker, disclosure glyph, elevated popup surface, row density, disabled/error styles, edge clipping, scrollbar, and fade. [`field`](field.md), [`list`](list.md), [`layers`](../foundations/layers.md), and [`theme`](../foundations/theme.md) own their respective shared mechanisms; this page defines Select composition and differences only.

The closed field ends in `▾`; while open it uses `▴`. The anchored rounded
popup is 12–40 cells wide, contains at most ten rows including its frame, and
flips above the field when there is not enough room below. Its selected option
keeps the `›` marker; the keyboard cursor uses the row focus treatment. The
popup remains anchored to the one Select focus stop; it does not become a
modal or steal focus from the field.

## Interaction and state rules

| Area | Required behavior |
|---|---|
| Keyboard | Closed plain Up/Down/Left/Right request the previous/next controlled value without opening. Enter/Space opens. Open Up/Down or `j`/`k` move the popup cursor; Enter/Space requests that choice. Escape restores the cursor to the committed choice and closes. Modified unknown chords do not trigger plain actions. |
| Pointer | Field toggles/open; popup row click chooses a stable key; outside click dismisses without replacing value. Removed/disabled targets never choose. |
| Focus/hover | The field remains the single focus stop while closed or open; the anchored popup supplies row hit regions without a modal focus trap. Losing field focus closes it. Keyboard suppresses stale hover. |
| Disabled/read-only | Disabled field cannot open or capture; disabled rows remain visible but cannot choose. Read-only eligibility is caller policy. |
| Editing | Query/edit input is optional and belongs to the configured picker policy; it never bypasses layer priority or choice control. |
| Scroll/resize | Popup shares ScrollState, thumb/fade rules, edge clipping, and anchor flip/clamp. Resize re-anchors deterministically. |
| Unicode | Labels, values, options, and query are measured/truncated by display cells. |
| Capability/color | Field, elevated popup, selected/current, disabled, error, and fade styles map semantically across color capabilities. |
| Motion | Use shared feedback/fade policy; do not add a Select animation loop. |

## Applicable state matrix

| Axis | Cases |
|---|---|
| Geometry | normal, zero area, tiny area, nonzero origin, exact fit, one cell short, long Unicode, narrow then wide |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover |
| Activation | pointer down, held press, inside/outside release, removed/disabled target, keyboard activation, feedback active/expired when applicable |
| Source | empty, ready, selected differs from cursor, disabled entry, reorder, insert, remove, filter, stale target |
| Scroll | no overflow, start, middle, end, wheel boundary, thumb drag, resize while scrolled, edge fade, protected row |
| Layer | closed, open, nested, Escape, outside, owner removed, resize re-anchor, focus restore, modal-first paste |

## Required capture cases

| Case | Exact requirement |
|---|---|
| W10-01 | Closed selected/placeholder, focused, and disabled. |
| W10-02 | Open at top/bottom edge and after resize. |
| W10-03 | Highlight differs from committed option; Escape preserves chosen value. |
| W10-04 | Reorder/delete options while open; click uses stable key. |
| W10-05 | Empty/all-disabled options; popup scrollbar and edge fade. |

Capture W10 at all five dimensions and all five capabilities. Record field/popup cells, cursor, focus owner, capture owner, layer owner, chosen/highlight keys, action count and target, and pre/post committed value.

## Foundation and component dependencies

- [`identity`](../foundations/identity.md), [`collections`](../foundations/collections.md) — keyed options and reconciliation.
- [`input-actions`](../foundations/input-actions.md), [`runtime`](../foundations/runtime.md) — typed events, focus, hit, capture, hover.
- [`layers`](../foundations/layers.md) — popup ownership, dismissal, restoration, and placement lifecycle.
- [`layout`](../foundations/layout.md), [`theme`](../foundations/theme.md), [`text`](../foundations/text.md) — geometry, semantic styles, display widths.
- [`author`](../foundations/author.md), [`conformance`](../foundations/conformance.md) — constrained parts and oracle proof.
- [`Field`](field.md) and [`List`](list.md) — shared field chrome and list-row rendering contracts.

## Negative tests and acceptance

- Escape/outside dismissal never changes the controlled selection.
- Highlighting, opening, drawing, or focus arrival cannot commit a choice.
- Disabled/removed/stale rows cannot choose; stale pointer IDs cannot retarget.
- Anchor removal closes/reanchors and releases capture; no orphan layer remains.
- Popup cannot escape screen bounds or overwrite the parent surface.
- Candidate output cannot become expected output.

Accept after the external API example, source-state/reconciliation tests, exact layer and snapshot cases, and independent review pass.
