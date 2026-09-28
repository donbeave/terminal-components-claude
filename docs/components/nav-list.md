# NavList

Canonical Termrock contract for W14. `NavList` is a reusable navigation presentation; it does not own a router or page lifecycle.

| Field | Value |
|---|---|
| Group / phase | Collections / P2 |
| Legacy family | C24 |
| Oracle | `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` (`visual-baseline`) |
| Capture plan | [`../reference/capture-plans/nav-list.json`](../reference/capture-plans/nav-list.json) |

## Purpose and boundary

The caller supplies navigation labels, stable item keys, section headers, icons, badge values, disabled state and the controlled active key. `NavList` provides full and compact presentation, keyboard/pointer navigation and scroll. It returns typed navigation requests; it never changes a route, creates a page, owns application content or duplicates sidebar scroll/focus code.

The frozen consumers are Showcase navigation in [`src/bin/showcase/app.rs`](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/showcase/app.rs) and the Jackin Preview manager in [`src/bin/jackin_preview/screens/manager.rs`](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/jackin_preview/screens/manager.rs). Their visible navigation and keyboard behavior are oracle evidence. Any current `WidgetId`/`Outcome` names remain legacy implementation identifiers.

## Target public API

```rust
NavList::new(id: Id, items: &'a [NavItem<'a>], revision: Revision) -> NavList<'a>

NavList::active(Option<ItemKey>)
    .mode(NavMode) // Full or Compact
    .row(&NavRowPainter)
    .patch(StylePatch);

NavList::update(&self, cx: &mut Cx<'_>, state: &mut NavListState)
    -> Response<NavAction>;
NavList::draw(&self, ui: &mut Ui<'_>, area: Rect, state: &NavListState) -> Rect;
NavList::measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size;
```

`NavListState` owns only cursor `ItemKey` and shared `ScrollState`; active route/key is controlled by the caller. `NavAction` is `Navigate { key, origin }` or `EnterContent { key }`. Fields, geometry, caller rows and runtime handles stay private. Read-only cursor, active-key and scroll observations may be exposed.

## Ownership and reconciliation

- Stable item keys, order, labels, section metadata, badge values and disabled state are borrowed props.
- Section headers are presentation rows, never selectable or activatable. Disabled entries remain visible when supplied but are skipped and cannot emit a route request.
- Cursor, controlled active key and pointer hover are separate. A route request transfers responsibility to the caller; `NavList` never routes internally.
- Full and compact modes use one identity model, one selection rule and one row painter. Changing mode at a breakpoint preserves active key, cursor key and scroll where possible.
- Source revision reconciliation retains surviving keys through insertion, removal and reorder. A removed target is stale and cannot activate a new occupant at the old index.
- `update` handles input and typed requests; `draw` only paints/registers hit areas; `measure` is pure. Runtime owns focus, hover, capture, geometry, layers and time.

## Visual contract

Styled parts are `container`, `section`, `row`, `gutter`, `marker`, `icon`, `label`, `badge`, `scrollbar` and `fade`. Theme and patch precedence follow [`theme`](../foundations/theme.md); patches cannot replace geometry, focus/capture or whole-surface ownership.

Preserve the baseline sidebar hierarchy: section rows, focus gutter, active marker, icon/label spacing, badge alignment, disabled treatment, full/compact visibility rules, clipping and edge fades. Full and compact are presentation modes, not separate engines. Clipped badges must not overwrite labels. Protected active/cursor rows remain readable through fades.

## Interaction contract

- Keyboard navigation moves among selectable item keys, skipping section headings and disabled entries according to the baseline key map. Enter returns `EnterContent` or `Navigate` with a stable key; it never mutates active state itself.
- Pointer hover lifts an eligible row and may update the visual cursor. Completed click activation resolves the captured key. Release outside, a removed target or a target disabled while pressed produces no request.
- Wheel scrolls the navigation region under the pointer without transferring focus. Scrollbar thumb press/drag uses the shared runtime capture owner. Boundary wheel events are consumed.
- Keyboard input suppresses stale pointer hover until pointer motion restores it. No decorative section row creates a focus stop.
- Resize and full/compact transitions preserve key identity and reanchor scroll. Narrow widths clip according to the baseline policy rather than introducing a new layout.
- Where navigation activation has baseline feedback, verify the 140 ms sample and expiry explicitly; motion never advances from repaint count.

## Applicable state matrix

| Axis | Required cases |
|---|---|
| Geometry | normal, zero/tiny area, non-zero origin, exact fit, one-cell short, long Unicode labels, narrow-to-wide mode round trip |
| Focus/hover | unfocused, focused cursor, hovered row, focus+hover, keyboard-suppressed hover, pointer restoration |
| Activation | pointer down/held/release inside/outside, removed target, disabled target, keyboard activation and applicable activation feedback |
| Source | empty list, ready list, active differs from cursor, disabled entry, insert/remove/reorder, filtered/stale target |
| Scroll | no overflow, start/middle/end, wheel boundary, thumb drag, resize while scrolled, edge fade and protected active row |

Readiness states apply only when the caller supplies them. Exact dimensions and terminal capabilities are shared by [`../verification/visual-parity.md`](../verification/visual-parity.md); do not fabricate irrelevant states for a static section-only presentation.

## Required parity and capture cases

| Case | Required observation |
|---|---|
| W14-01 | Controlled active item differs from the keyboard cursor, with both distinctions visible. |
| W14-02 | Section headings and disabled entries are skipped appropriately and cannot activate. |
| W14-03 | Full/compact breakpoint round trip preserves active key and scroll. |
| W14-04 | Empty list, overflow and clipped badges retain baseline geometry. |
| W14-05 | Focus and scroll ownership is shared; no second sidebar implementation is introduced. |

The oracle is the frozen application output and interaction trace. Bind expected output before candidate testing through [`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md). Compare exact cells, wide-cell continuation, colors/modifiers supported by the tool, cursor, focus/capture owner, stable active/cursor key and typed action target/count. Candidate rendering cannot bless its own snapshots.

## Dependencies and negative tests

Dependencies: [`list`](./list.md), [`identity`](../foundations/identity.md), [`input-actions`](../foundations/input-actions.md), [`runtime`](../foundations/runtime.md), [`layout`](../foundations/layout.md), [`theme`](../foundations/theme.md), [`collections`](../foundations/collections.md), [`author`](../foundations/author.md), and [`conformance`](../foundations/conformance.md).

Negative tests must prove: section rows never receive focus or actions; disabled/removed keys never route; index changes cannot retarget a held pointer; compact mode does not create a second selection/scroll engine; draw cannot mutate active/cursor/source state; wheel cannot transfer focus; hidden rows are not hit targets; Unicode widths remain correct; and supported part patches alter their claimed cells without overwriting neighboring content.
