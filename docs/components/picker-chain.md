# PickerChain

Canonical Termrock contract for W20. `PickerChain` composes the shared Picker mechanism across keyed stages; it does not know any backend or product domain.

| Field | Value |
|---|---|
| Group / phase | Overlays / P4 |
| Legacy family | C29 |
| Oracle | `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` (`visual-baseline`) |
| Capture plan | [`../reference/capture-plans/picker-chain.json`](../reference/capture-plans/picker-chain.json) |

## Purpose and boundary

The caller owns stage availability, keyed result data, readiness and asynchronous generation. `PickerChain` remembers a stable stage path plus each stage's query, highlighted key and scroll, and returns stage/key actions. It does not know vaults, providers, directories, credentials, network state or backend loaders.

The frozen composition is represented by the picker-chain flows in [`src/bin/jackin_preview/screens/modals.rs`](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/jackin_preview/screens/modals.rs), with Picker behavior from [`src/widgets/picker.rs`](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/picker.rs). Current source identifiers are legacy evidence only.

## Target public API

```rust
PickerChain::new(
    id: Id,
    stages: &'a [PickerStage<'a>],
    revision: Revision,
) -> PickerChain<'a>;

PickerChain::current(ItemKey)
    .readiness(Readiness<'a>)
    .patch(StylePatch);

PickerChain::update(&self, cx: &mut Cx<'_>, state: &mut PickerChainState)
    -> Response<PickerChainAction>;
PickerChain::draw(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    state: &PickerChainState,
) -> Rect;
PickerChain::measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size;
```

`PickerChainState` owns stable stage path and per-stage query/cursor/scroll. No backend result, credential or loader state is stored. Typed actions are `Accept { stage: ItemKey, key: ItemKey }`, `Back`, `Retry { stage: ItemKey }` and `Cancel`. Read-only current stage/result observations are allowed.

## Ownership and reconciliation

- Stage and result data are borrowed caller props. Caller generation/revision checks decide whether returned asynchronous data is current; the chain accepts only the supplied accepted revision.
- `Back` restores the previous stage's query, highlighted stable key and scroll. `Cancel` exits without resetting a dirty underlying form or other caller state.
- A changed earlier choice invalidates incompatible later-stage view state. It must clear or reinitialize later stages according to the caller's stage revision; it must never bind an old key to a new result.
- Retry emits the stable stage key. Loading, partial, error and retry are presentation/readiness states, not backend behavior.
- `update` performs stage navigation and typed requests. `draw` is read-only and composes one Picker surface per current stage; `measure` is pure. Runtime owns modal layer, focus trap, capture, placement, time and restoration.

## Visual contract

Styled parts are `container`, `breadcrumb`, `title`, `query`, `row`, `footer`, `empty` and `status`. Use [`theme`](../foundations/theme.md) and Picker styling; no chain-local palette or query engine.

Preserve the baseline modal surface, breadcrumb/stage path, query field, row hierarchy, status/loading/error/empty text, footer actions, clipping, backdrop, focus trap and narrow-width collapse. Breadcrumbs may collapse at narrow widths only according to the specified baseline recipe. Stage-specific result styling remains the Picker recipe.

## Interaction contract

- The current stage owns query focus and result navigation through Picker semantics. Enter emits `Accept` with both stable stage and result keys. Search-disabled stages retain navigation/dismiss behavior.
- Back (including empty-query Backspace where supplied) rewinds one stage and restores its durable view state. Cancel closes the chain and restores the prior owner focus without clearing that owner's drafts.
- Retry returns the stable stage key. A stale completion for an abandoned stage is ignored by caller generation checks and cannot alter selection, breadcrumb or scroll.
- Pointer row activation uses completed captured gestures. Outside click/Escape follows the runtime modal policy. Wheel/thumb drag stay in the current stage and do not transfer focus. Resize reanchors/clamps while preserving stage path.
- Keyboard activity suppresses stale hover until pointer motion. Disabled/unavailable rows cannot activate; changed earlier choices invalidate later keys deterministically.
- Where a stage action has baseline feedback, sample the 140 ms window and expiry explicitly; redraw frequency never advances it.

## Applicable state matrix

| Axis | Required cases |
|---|---|
| Geometry | normal, zero/tiny area, non-zero origin, exact fit, one-cell short, long Unicode content, narrow breadcrumb collapse |
| Focus/hover | unfocused, focused query/row, hovered, focus+hover, keyboard-suppressed hover, pointer restoration |
| Activation | stage/result pointer down/held/release inside/outside, removed/disabled target, keyboard accept, applicable feedback |
| Source/readiness | empty, ready, selected differs from cursor, disabled entry, insert/remove/reorder/filter, stale target, loading/partial/error/retry at every stage |
| Scroll/layer | no overflow/start/middle/end, wheel boundary, thumb drag, resize/reanchor, edge fade/protected row; closed/open/nested/Escape/outside/owner removed/focus restore/modal-first-paste |

Only states supplied by a stage apply. Shared dimensions/capabilities are in [`../verification/visual-parity.md`](../verification/visual-parity.md).

## Required parity and capture cases

| Case | Required observation |
|---|---|
| W20-01 | Stage 1 → stage 2 → Back restores query, cursor key and scroll. |
| W20-02 | Loading/error/retry is represented at each applicable stage. |
| W20-03 | Stale completion for an abandoned stage produces no selection change. |
| W20-04 | Nested form retains all drafts after chain Cancel. |
| W20-05 | Breadcrumb collapse at narrow widths preserves stage identity and action reachability. |

Bind expected output before candidate testing with [`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md). Compare exact cells/continuation, styles/modifiers supported by the tool, cursor, focus/capture/layer owner, stage path, query drafts, selected keys, readiness and typed action count/target. Invalid oracle setup is blocked.

## Dependencies and negative tests

Dependencies: [`picker`](./picker.md), [`identity`](../foundations/identity.md), [`input-actions`](../foundations/input-actions.md), [`runtime`](../foundations/runtime.md), [`layers`](../foundations/layers.md), [`layout`](../foundations/layout.md), [`theme`](../foundations/theme.md), [`collections`](../foundations/collections.md), [`author`](../foundations/author.md), and [`conformance`](../foundations/conformance.md).

Negative tests must prove: no credentials/backend/network behavior; stale generation cannot mutate an abandoned stage; Back restores the prior stage by key; changed earlier choices cannot rebind later state; Cancel cannot clear the underlying form; disabled/stale results cannot activate; draw cannot advance stages or mutate drafts; outside/Escape focus restoration is stable; and one shared Picker engine handles each stage.
