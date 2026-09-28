# TooSmall

**Inventory:** W43 · chrome · P2 · baseline composition  
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

TooSmall is a generic minimum-size notice. The caller supplies the threshold
and exact message. The library does not impose one product's minimum on every
component.

### Non-goals

- Redesigning the application when the terminal is small.
- Owning quit/exit handling.
- Reconstructing child scenes or discarding their durable state.
- Assuming the Jackin threshold applies to Showcase, TablePro or Holla.

## Oracle and provenance

- [Frozen Jackin composition](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/jackin_preview/app.rs)
- Snapshot discovery: frozen Jackin views.
- [TooSmall capture plan](../reference/capture-plans/too-small.json)
- [Layout contract](../foundations/layout.md), [runtime contract](../foundations/runtime.md)
  and [visual parity proof](../verification/visual-parity.md)

The baseline-like Jackin fixture uses 72 by 20. That is an application
composition case, not a global Termrock default.

## Target public API

    TooSmall::new(minimum: Size, message: &'a str) -> TooSmall<'a>

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builder:

    patch(StylePatch)

TooSmall has no durable state or typed action. The caller/runtime retains
previous component state while the notice is shown; exit handling stays in
the host binding scope.

## Ownership and rendering

| Concern | Owner |
| --- | --- |
| Minimum size and exact message | Caller |
| Whether to show the notice | Host/application composition |
| Existing focus, capture, draft and modal state | Runtime/application |
| Painting and clipping | TooSmall |

draw is bounded by area and safe for zero/tiny rectangles. Growing back above
the threshold reveals the existing scene and restores prior focus/draft state;
it does not reconstruct the scene.

## Visual and interaction contract

Parts are container, message and dimensions. Preserve baseline notice text,
alignment, dimensions and surface treatment supplied by the caller. The
component is display-only and does not consume quit or other host bindings.

Unicode message width and clipping follow [text](../foundations/text.md);
semantic styling and no-color behavior follow [theme](../foundations/theme.md).
Resize behavior is an explicit geometry state, with no motion.

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, 71/72/73 columns × 19/20/21 rows for the baseline-like fixture, 0×0, 1×1, nonzero origin |
| Focus/hover | Not applicable; notice is not a focus stop |
| Activation | Not applicable; host remains responsible for quit |
| Continuity | shrink with modal open, then grow; preserve focus and draft |
| Color/capability | semantic fallback across all capture capabilities |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W43-01 | 71/72/73 columns crossed with 19/20/21 rows |
| W43-02 | 0×0, 1×1 and nonzero-origin notices |
| W43-03 | Shrink while modal open then grow restores focus/draft |
| W43-04 | Quit remains available while too small |

Record exact cells/styles, threshold, actual size, host focus/capture and
draft state. The last case is an application/runtime trace, not a TooSmall
action.

## Foundations and negative tests

Depends on [layout](../foundations/layout.md), [runtime](../foundations/runtime.md),
[theme](../foundations/theme.md), [text](../foundations/text.md),
[authoring](../foundations/author.md) and [conformance](../foundations/conformance.md).

Required negative tests:

- unsigned geometry arithmetic cannot underflow;
- draw cannot write outside area;
- showing the notice cannot discard focus, pointer capture or drafts;
- TooSmall cannot consume host quit or invent an action;
- a product-specific threshold cannot become a library default;
- grow-after-shrink cannot create a fresh state object.

