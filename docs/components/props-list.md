# PropsList

**Inventory:** W35 · data and text · P3 · baseline composition  
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

PropsList is the interactive, scrollable property sheet. It reuses the Props
row painter and the shared List and ScrollRegion mechanisms. The caller owns
row data, safe copy payloads and protected-value policy.

### Non-goals

- A second list, scrollbar or row-rendering engine.
- Copying clipped display text or revealing protected data.
- Exposing mutable domain rows, runtime geometry or a raw index as identity.
- Polling services or interpreting property values.

## Oracle and provenance

- [Frozen widget source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/props.rs)
- Snapshot discovery: frozen Jackin accounts and manager views.
- [PropsList capture plan](../reference/capture-plans/props-list.json)
- [List component contract](./list.md), [Props contract](./props.md) and
  [visual parity proof](../verification/visual-parity.md)

The frozen implementation currently uses index-backed rows internally. The
target API replaces that accidental identity with stable semantic row keys.

## Target public API

    PropsList::new(
        id: Id,
        rows: &'a [PropsRow<'a>],
        revision: Revision,
    ) -> PropsList<'a>

    update(
        &self,
        cx: &mut Cx<'_>,
        state: &mut PropsState,
    ) -> Response<PropsAction>
    draw(&self, ui: &mut Ui<'_>, area: Rect, state: &PropsState) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    label_width(LabelWidth)
    copy_policy(CopyPolicy)
    patch(StylePatch)

PropsState is caller-owned durable state containing a keyed cursor, selection
where applicable and shared ScrollState. Fields remain private; read-only
observations expose the selected key and scroll position without exposing
geometry.

    PropsAction::CopyRequested { key: ItemKey }

CopyRequested identifies source data. It never contains rendered or truncated
text. Activation of a row, if a future consumer needs it, is a separate typed
action and must not be inferred from copying.

## Ownership and update/draw/measure

| Concern | Owner |
| --- | --- |
| Rows and safe copy data | Caller, borrowed for the frame |
| Cursor, selected key and scroll offset | PropsState |
| Focus, hit testing, hover and pointer capture | Runtime |
| Reconciliation across revision | Collections foundation |
| Copy policy and protected-value rejection | Caller policy plus component gate |
| Painting and measured row geometry | Shared Props painter and ScrollRegion |

update handles keyboard navigation, pointer row targeting, wheel scrolling,
thumb presses/drags and copy eligibility. It returns a typed response and
never mutates the source slice. draw is read-only with respect to PropsState;
measure and draw share wrapped-row geometry so hitboxes and scroll ranges
agree.

The baseline key behavior includes Up/Down, j/k, PageUp/PageDown, Home,
End/g/G, Enter and y for an eligible copyable row. The final keymap is owned
by [input/actions](../foundations/input-actions.md); preserve observable
baseline behavior while allowing caller remapping through typed bindings.

## Visual and interaction contract

Parts are container, row, gutter, marker, label, value, status, scrollbar and
fade. Props supplies label/value alignment and wrapping. List and
ScrollRegion supply focus gutter, row marker, scrollbar thumb, edge fade and
pointer capture. Style patches cannot replace hit geometry or runtime
ownership.

Protected rows may show a locked or masked state. Read-only means no editing;
it does not make every value safe to export. A protected copy request is
rejected with no action. Reordering, insertion, removal and filtering
reconcile by stable key; a stale pointer target cannot retarget a new row.

Unicode grapheme widths, combining marks and wrapped values must match between
measurement, painting and hit testing. Preserve semantic tones and capability
fallbacks through [theme](../foundations/theme.md). Scroll thumb drag,
resize-while-scrolled, top/middle/end offsets and protected-row fades follow
[runtime](../foundations/runtime.md), [layout](../foundations/layout.md) and
[text](../foundations/text.md).

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, long Unicode, narrow then wide |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover |
| Activation | pointer down, held press, release inside/outside, removed target, disabled target, keyboard copy/activate, feedback timing |
| Source | empty, ready, selected differs from cursor, disabled row, reorder, insert, remove, filter, stale target |
| Scroll | no overflow, start/middle/end, wheel boundary, thumb drag, resize while scrolled, edge fade, protected row |
| Editing/motion | no text editing; no component-owned animation |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W35-01 | Normal copy returns source identity, never clipped display text |
| W35-02 | Protected row copy is rejected |
| W35-03 | Reorder/delete preserves or explicitly drops the keyed hovered/selected property |
| W35-04 | Wrapped rows keep accurate hit and scroll geometry with protected-row fade |

Record exact frame cells/styles, dimensions and capabilities, cursor, focus
owner, capture owner, selected stable key, navigation key, source revision,
draft/selection state and typed action target/count.

## Foundations and negative tests

Depends on [identity](../foundations/identity.md), [input/actions](../foundations/input-actions.md),
[runtime](../foundations/runtime.md), [layout](../foundations/layout.md),
[text](../foundations/text.md), [collections](../foundations/collections.md),
[theme](../foundations/theme.md), [authoring](../foundations/author.md) and
[conformance](../foundations/conformance.md).

Required negative tests:

- protected values never enter CopyRequested;
- rendered truncation cannot become an action payload;
- row removal during a pointer press cannot activate a different row;
- measure/draw/hit geometry cannot disagree for wrapped Unicode rows;
- draw cannot mutate cursor, scroll or source rows;
- duplicate or missing stable keys fail reconciliation;
- pointer capture ends on release outside the list.

