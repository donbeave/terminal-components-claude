# RadioGroup

**Component ID:** W05  
**Group:** Controls  
**Phase:** P3  
**Contract status:** canonical Termrock target; implementation is future work on `termrock-refactor`.

## Purpose and scope

`RadioGroup` presents keyed mutually exclusive choices. The caller supplies option records and the controlled selected key. Durable state stores only the navigation cursor and scroll bookkeeping; it never becomes the chosen domain value.

### Non-goals

- No index-based domain selection, persistence, filtering policy, or schema model.
- No automatic selection merely because focus arrived or an arrow moved the cursor.
- No second list, focus, or scroll engine.

## Oracle and provenance

The frozen oracle is [`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`](https://github.com/donbeave/terminal-components-claude/commit/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b). The legacy implementation is `RadioGroup` in [`src/widgets/choice.rs`](../../src/widgets/choice.rs), source blob `9405acf87c152c0766ae9bbcd71c01c9d5db4e80`; Showcase settings/choice pages are baseline consumers. The W05 source plan is [`capture-plans/radio-group.json`](../reference/capture-plans/radio-group.json), planned and uncaptured.

## Public API and ownership

```rust
RadioGroup::new(
    id: Id,
    options: &'a [ChoiceItem<'a>],
    revision: Revision,
) -> RadioGroup<'a>

group.update(&mut cx, &mut state) -> Response<RadioAction>
group.draw(&mut ui, area, &state) -> Rect
group.measure(&measure_cx, constraints) -> Size
```

Builders: `selected(Option<ItemKey>)`, `orientation(Axis)`, `disabled(bool)`, `patch(StylePatch)`, and `row(&ChoiceRowPainter)`.

`RadioGroupState` owns only cursor key and `ScrollState`. `selected` is controlled input. The typed action is `RadioAction::Choose { key: ItemKey, origin: ActivationOrigin }`. Expose read-only keyed cursor/navigation/scroll observations and invariant-preserving commands; keep fields, runtime geometry, and mutable domain rows private.

## Update, draw, and measure

- `update` reconciles options by stable `ItemKey`, moves the candidate cursor, scrolls it into view, and emits `Choose` only on explicit activation. Empty/all-disabled input emits no invalid index or action.
- `draw` paints label and visible rows, distinguishing chosen marker from cursor/focus gutter. It registers row hit targets with stable child IDs and does not choose a value or mutate state.
- `measure` accounts for label, row orientation, markers, gutter, display-cell widths, and available viewport. It clips safely at zero/tiny geometry.

## Identity and reconciliation

Group `Id` and each stable `ItemKey` own semantic identity. Reorder preserves cursor/chosen keys when they survive; removal resolves to the documented neighboring key or no cursor. A pointer press stores the key and release validates current membership and eligibility, so stale rows cannot activate a replacement. Disabled rows remain visible but cannot activate.

## Visual contract

Parts are `container`, `gutter`, `marker`, and `label`. Preserve baseline radio markers `(●)`/`( )` and compact forms, group label spacing, row density, focus gutter, selected/current distinction, disabled tone, hover lift, and clipping. [`theme`](../foundations/theme.md) owns semantic style precedence; row painter customization is constrained and cannot replace runtime ownership or the entire surface.

## Interaction and state rules

| Area | Required behavior |
|---|---|
| Keyboard | Arrow/HJK navigation changes candidate cursor only. Enter/Space chooses the surviving cursor key. |
| Pointer | A row is eligible only within its clipped rectangle. Press/release validates the same stable key; disabled, removed, or outside release emits no choice. |
| Focus/hover | One group focus stop; runtime owns hover and capture. Keyboard suppresses stale hover until pointer motion. Row focus/selection markers remain distinct. |
| Disabled/read-only | Disabled rows remain visible, cannot focus/activate, and clear hover/press. A disabled/all-disabled group emits no action. |
| Editing | Not applicable. |
| Scroll/resize | Share `ScrollState`; preserve key identity and keep cursor visible. Reconcile after resize and source updates without index panic. |
| Unicode | Truncate by display cells, preserving combining and wide glyph boundaries. |
| Capability/color | Semantic selected, current, disabled, muted, and surface styles resolve for truecolor, 256, 16, none, and nocolor. |
| Motion | Shared feedback only where baseline applies; no private animation loop. |

## Applicable state matrix

| Axis | Cases |
|---|---|
| Geometry | normal, zero area, tiny area, nonzero origin, exact fit, one cell short, long Unicode, narrow then wide |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover |
| Activation | pointer down, held press, release inside/outside, removed target, disabled target, keyboard activate, feedback active/expired when applicable |
| Source | empty, ready, selected differs from cursor, disabled entry, reorder, insert, remove, filter, stale target |
| Scroll | no overflow, start, middle, end, wheel boundary, thumb drag, resize while scrolled, edge fade/protected row where applicable |

## Required capture cases

| Case | Exact requirement |
|---|---|
| W05-01 | Chosen item differs from cursor and hovered item. |
| W05-02 | Disabled first, middle, and last options with keyboard and pointer. |
| W05-03 | Reorder or delete chosen/cursor item between pointer press and release. |
| W05-04 | Empty and all-disabled groups; narrow horizontal layout. |

Capture at every plan dimension (72×20, 80×24, 100×30, 120×40, 160×50) and capability (truecolor, 256, 16, none, nocolor). Record exact cells, cursor, focus/capture/layer owner, action count/target, chosen key, cursor key, and source revision.

## Foundation dependencies

- [`identity`](../foundations/identity.md), [`collections`](../foundations/collections.md) — stable keys and source reconciliation.
- [`input-actions`](../foundations/input-actions.md) — navigation and typed choice actions.
- [`runtime`](../foundations/runtime.md) — focus, hit, capture, hover, and time.
- [`layout`](../foundations/layout.md), [`theme`](../foundations/theme.md) — measured rows, clipping, semantic surfaces.
- [`author`](../foundations/author.md), [`conformance`](../foundations/conformance.md) — constrained row customization and oracle proof.

## Negative tests and acceptance

- Arrow navigation cannot change the controlled selected key.
- Empty/all-disabled groups cannot index, focus, or activate invalid rows.
- A stale pointer key cannot activate a replacement after reorder/removal.
- Disabled rows cannot capture or emit actions.
- Draw cannot mutate source, selection, or cursor semantics.
- Candidate output cannot be promoted to expected output.

Accept after API, reconciliation, source-state, exact snapshots, hit/capture, Unicode, and independent review pass.
