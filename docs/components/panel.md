# Panel

**Component ID:** W32 · **Phase:** P2 · **Legacy families:** C41, C40
**Oracle:** `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`

Panel is Termrock's chrome-only container. The contract covers card and framed
recipes and their open clipped body slot. It does not retain a child tree or
become an interactive widget.

## Purpose and boundary

Panel owns borrowed chrome props: kind, title, metadata, badge, focus-within
state, and semantic style patch. The caller supplies child composition and
child state. `draw` paints chrome and invokes a body slot inside the exact inner
clip; `measure` accounts for chrome around the child size.

Panel plus TextViewport is the target replacement for the old ScrollPanel
composition. TextViewport owns text, selection, and scroll semantics; Panel
owns only chrome. No separate ScrollPanel engine is retained.

### Non-goals

- retaining children or domain state;
- creating a decorative focus stop, hit target, or activation action;
- owning scrolling, focus, pointer capture, or child selection;
- inventing a layout when the body allocation is empty;
- a universal widget trait or compatibility promise for current `WidgetId`,
  `Outcome`, or `RenderCtx` internals.

## Frozen evidence and provenance

Parity is pinned to `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`.

- Source: `src/widgets/panel.rs` (blob
  `14a9edef27f16e463a635f314fa37d5825f57fd9`).
- Inline tests cover title/meta width budgets, late metadata repaint after child
  measurement, card/framed geometry, nested clipping, and scroll-panel tail or
  prose behavior now split between Panel and TextViewport.
- Visual consumer: Showcase panels, with protected snapshots under
  `snapshots/showcase/pages/panels`; child focus and scroll composition are
  also exercised by the four preserved applications and
  `tests/visual_baseline`.

See [`../verification/visual-parity.md`](../verification/visual-parity.md),
[`../verification/oracle-and-provenance.md`](../verification/oracle-and-provenance.md),
and [`../api/public-api.md`](../api/public-api.md).

## Target public API

```rust
Panel::new(id: Id) -> Panel<'a>

draw<R>(
    &self,
    ui: &mut Ui<'_>,
    area: Rect,
    body: impl FnOnce(&mut Ui<'_>, Rect) -> R,
) -> R

measure(
    &self,
    cx: &MeasureCx<'_>,
    child: Size,
    constraints: Constraints,
) -> Size
```

Configuration:

```rust
kind(PanelKind::Card | PanelKind::Framed)
title(&'a str)
meta(StyledText<'a>)
badge(Option<Badge<'a>>)
focus_within(bool)
patch(StylePatch)
```

Panel has no durable state and no typed action family. Child responses pass
through unchanged. The callback body receives a clipped allocation; border and
title are not child hit targets.

## Update, draw, and measure

There is no Panel `update`: it is display-only. Runtime focus-within facts are
read from the child composition or supplied as borrowed props. Panel must not
claim focus or capture.

`draw` paints a filled card or rounded framed surface, title row, metadata,
badge, and clipped body. It invokes the child exactly once in the returned inner
area. A zero/tiny intersection emits no invalid geometry. If metadata depends
on child layout (for example a scroll position), the composition may repaint
the title row after fresh measurement; this is a deterministic chrome pass, not
a state mutation.

`measure` reserves the card/frame insets and title row according to kind and
constraints, then returns the body allocation facts used by the caller. It must
not inspect or mutate child state.

## Visual contract

Advertised parts are `container`, `border`, `title`, `meta`, `badge`, and
`body`.

Card is a filled surface with the baseline title row and padding. Framed is the
rounded border recipe with its exact inset and title gap. Focus-within changes
the frame/title treatment only; the child remains the focus owner and keeps its
focus gutter. Title and metadata share the baseline width budget: the title
yields first to preserve metadata, then metadata truncates while the title
keeps its minimum naming cells. Badge claims space only when it fits.

Body paint inherits the selected surface and clip. Nested panels and nonzero
origins retain exact border, padding, clipping, and background behavior. No
color or motion reinterpretation is introduced. Capability/motion resolution
uses the shared theme foundation.

Part patches cannot replace geometry, child focus/capture, or whole-surface
ownership. The panel border never becomes a substitute hit target for children.

## Focus, interaction, and applicability

Panel has no independent focus, hover, press, pointer, keyboard, edit, scroll,
or activation behavior. A containing focus indication is a visual reflection of
child focus only. Child interactions continue to use their own canonical
component contracts.

| Axis | Required states | Disposition |
|---|---|---|
| Geometry | `normal`, `zero_area`, `tiny_area`, `nonzero_origin`, `exact_fit`, `one_cell_short`, `unicode_long_content`, `narrow_then_wide` | Component |
| Focus/hover | none on Panel; child focused/hovered may change `focus_within` | Child owner |
| Activation | none | Not applicable |
| Editing/scroll | none | Child owner |
| Readiness | none unless a child body renders it | Child owner |
| Motion | only shared border/theme transition if a recipe advertises it | Theme policy |

## Capture and acceptance contract

Use [`../reference/capture-plans/panel.json`](../reference/capture-plans/panel.json). It is planned
with no expected artifacts. Bind baseline output as `ExistingOracle` or
`ExtractedOracle`; any new API/safety guarantee is an `Extension`.

Run applicable cases at `72×20`, `80×24`, `100×30`, `120×40`, and `160×50`,
with truecolor, 256-color, 16-color, `none`, and `nocolor` capabilities.

| Case | Required observation |
|---|---|
| W32-01 | Card/framed title, metadata, and badge at exact-fit widths. |
| W32-02 | Child focused versus Panel remaining decorative. |
| W32-03 | Nested panels, nonzero origins, and clipping. |
| W32-04 | Empty/zero/tiny body and late scroll-count metadata. |

Record exact area, inner clip, title/meta/badge cells, child focus owner,
capability, and any late metadata repaint. Compare symbols, wide continuations,
foreground/background, modifiers, and dimensions exactly.

## Required negative tests

- Panel cannot register an independent focus stop or generic activation action.
- Panel cannot mutate child state, scroll state, or metadata during `draw`.
- Body output cannot paint outside the inner clip or over the border/title.
- Title/meta/badge truncation cannot overwrite corners, padding, or each other.
- Zero/tiny/nonzero-origin areas cannot panic or underflow dimensions.
- Child focus must not change child selection semantics; focus-within is visual
  only.
- A custom part patch cannot replace child layout, hit geometry, or capture.
- No ScrollPanel implementation may be introduced beside Panel and
  TextViewport.

## Foundation dependencies

- [`../foundations/identity.md`](../foundations/identity.md): semantic panel
  identity and child attribution scope.
- [`../foundations/runtime.md`](../foundations/runtime.md): focus-within read
  facts, clipping and child hit ownership.
- [`../foundations/layout.md`](../foundations/layout.md): insets, title row,
  measurement, nonzero origins, and clipping.
- [`../foundations/theme.md`](../foundations/theme.md): card/frame surfaces,
  borders, title/meta/badge parts, capability and motion.
- [`../foundations/author.md`](../foundations/author.md): open body slot and
  constrained part customization.
- [`../foundations/conformance.md`](../foundations/conformance.md): exact
  snapshots, compositional traces, and negative gates.

Acceptance requires an external body-slot consumer, exact applicable snapshots,
and independent review. A decorative placeholder or a panel that swallows child
responses fails acceptance.
