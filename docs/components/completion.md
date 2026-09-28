# Completion

Canonical Termrock contract for W21. `Completion` is an anchored suggestion popup owned by an editor; it is not a language server or syntax engine.

| Field | Value |
|---|---|
| Group / phase | Overlays / P4 |
| Legacy family | C30 |
| Oracle | `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` (`visual-baseline`) |
| Capture plan | [`../reference/capture-plans/completion.json`](../reference/capture-plans/completion.json) |

## Purpose and boundary

The caller supplies candidate items and a valid source-text replacement range tagged with document revision. `Completion` presents those candidates, manages highlighted key and popup scroll and returns a typed edit request. It does not invoke a language server, infer application syntax, own the document, edit text directly or use display-cell coordinates as edit ranges.

The frozen source is [`src/widgets/completion.rs`](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/completion.rs), currently exposing `Completion`, `CompletionItem`, `CompletionEvent`, `WidgetId` and `Outcome`. Showcase editor and TablePro editor consumers remain unchanged oracle applications. Current names are legacy source identifiers, not future Termrock API requirements.

## Target public API

```rust
Completion::new(
    id: Id,
    owner: Id,
    items: &'a [CompletionItem<'a>],
    revision: Revision,
) -> Completion<'a>;

Completion::anchor(Anchor)
    .replacement(TextRange)
    .patch(StylePatch);

Completion::update(&self, cx: &mut Cx<'_>, state: &mut CompletionState)
    -> Response<CompletionAction>;
Completion::draw(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    state: &CompletionState,
) -> Rect;
Completion::measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size;
```

`CompletionState` owns highlighted candidate key and scroll. `CompletionController` binds the popup to an editor `Id` and query/document revision. `CompletionAction` is `Apply { key, edit: TextEdit }` or `Dismissed`. Read-only highlighted key/scroll observations are allowed. The editor retains text, caret and editing state.

## Ownership and reconciliation

- Candidate labels, kinds, detail, match spans, insert text and replacement range are borrowed caller props. The range is source-text/grapheme aware and tagged with the document revision; it is never a display-cell range.
- If the owner revision changes, the old candidate range is invalid. Reject stale/out-of-bounds edits instead of applying them to a new document location.
- Stable candidate keys survive refresh/reorder where supplied. A removed candidate cannot become a different candidate because it kept an old index.
- `update` handles popup navigation and typed apply/dismiss actions. `draw` is semantically immutable and registers only the popup's visible rows. `measure` is pure.
- Runtime owns the popup layer, focus/capture policy, anchor placement, clipping, time and restoration. The editor owns keyboard/caret focus while the popup is open; no second hardware cursor is created.

## Visual contract

Styled parts are `container`, `row`, `kind`, `label`, `detail`, `match`, `scrollbar` and `fade`. Theme/style patches follow [`theme`](../foundations/theme.md); part customization cannot replace popup geometry or editor/runtime ownership.

Preserve the baseline anchored rounded surface, row gutter/focus marker, kind glyph, match emphasis, label/detail alignment, clipping, scrollbar and edge fade. Popup placement tracks the owner caret, flips when below space is insufficient, clamps at screen edges and remains correct in narrow bottom/right corners. Unicode match spans use grapheme/display-cell boundaries; continuation cells and cursor placement are exact parity data.

The baseline popup is 24–48 cells wide and shows at most eight candidate
rows. Each row keeps fixed columns for the focus gutter, kind, label, `…`, and
detail; compute those widths over the candidate set so scrolling cannot shift
alignment. Matched label characters are bold. These widths and columns remain
part of the frozen oracle even though callers supply kinds and details.

## Interaction contract

- Editor typing and caret ownership continue while completion is open. The popup consumes its navigation keys (Up/Down, PageUp/PageDown and the baseline `Ctrl-N`/`Ctrl-P` aliases) and keeps the highlighted key visible.
- Tab and Enter apply exactly one revision-valid splice for the highlighted key. Escape dismisses the popup first; the next Escape follows the editor's own editing policy. Applying or dismissing returns typed action data to the caller.
- Pointer click on a visible row returns `Apply` for its stable key. Wheel scroll and captured scrollbar drag move only the popup viewport and do not steal editor focus. Boundary wheels are consumed.
- Popup reanchors/clips on resize and owner-caret movement. Keyboard activity suppresses stale hover until pointer motion; release outside does not apply a row.
- Candidate replacement ranges are validated at action time. Invalid, unavailable or removed candidates produce no edit and no retargeted action.
- Completion has no invented animation; if the baseline supplies activation feedback, sample its 140 ms window and expiry from explicit time.

## Applicable state matrix

| Axis | Required cases |
|---|---|
| Geometry | normal, zero/tiny area, non-zero origin, exact fit, one-cell short, long Unicode labels/details, narrow bottom/right-corner placement |
| Focus/hover | editor-owned focus, popup row focus presentation, hovered, focus+hover, keyboard-suppressed hover, pointer restoration; no second hardware cursor |
| Activation | row pointer down/held/release inside/outside, removed target, keyboard Tab/Enter apply, Escape dismiss, stale range rejection |
| Source | empty/ready candidates, highlighted differs from editor caret, insert/remove/reorder, stale/out-of-bounds range |
| Scroll/layer | no overflow/start/middle/end, wheel boundary, thumb drag, resize/reanchor, edge fade/protected row; closed/open/nested/Escape/outside/owner removed/focus restore |

There is no generic whole-surface activation. Use the shared dimensions/capabilities and applicable state policy in [`../verification/visual-parity.md`](../verification/visual-parity.md).

## Required parity and capture cases

| Case | Required observation |
|---|---|
| W21-01 | Editor continues typing while the completion list remains open. |
| W21-02 | Tab/Enter apply one revision-valid splice only. |
| W21-03 | Escape closes the popup; a second Escape follows editor policy. |
| W21-04 | Paste or another document revision invalidates the old candidate range. |
| W21-05 | Narrow bottom/right-corner popup placement flips/clamps correctly. |

Bind expected output first through [`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md). Compare exact dimensions, cells/continuation, styles/modifiers supported by the tool, editor cursor, popup owner/focus/capture/layer, candidate key, document/draft state and typed action count/target. Invalid oracle setup blocks the case.

## Dependencies and negative tests

Dependencies: [`list`](./list.md), [`text`](../foundations/text.md), [`identity`](../foundations/identity.md), [`input-actions`](../foundations/input-actions.md), [`runtime`](../foundations/runtime.md), [`layers`](../foundations/layers.md), [`layout`](../foundations/layout.md), [`theme`](../foundations/theme.md), [`author`](../foundations/author.md), and [`conformance`](../foundations/conformance.md).

Negative tests must prove: no language-server, parser or executor behavior; stale revision/range cannot edit the document; byte offsets cannot be treated as display columns; Enter/Tab apply once; Escape dismisses before editor cancel; popup cannot steal a second hardware cursor; removed candidates cannot retarget; draw cannot mutate editor/document/candidate state; hidden rows cannot receive clicks; and custom match/part styling reaches the claimed cells without overwriting the editor.
