# Empty

**Inventory:** W36 · feedback · P2 · baseline composition\
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

Empty presents a caller-controlled readiness state inside a component surface.
Empty, loading, partial and error are distinct states. An optional retry or
action affordance is a real Button child with a typed action.

The same readiness presentation may be composed by List, Tree, Grid, Picker
and text viewport consumers while inheriting the owner surface and part
patches.

### Non-goals

- Fetching, retrying, polling or interpreting domain errors.
- Replacing partial data with a blank screen.
- Treating an error, loading state or empty result as the same boolean.
- Painting action-looking text without a focusable Button contract.

## Oracle and provenance

- [Frozen widget source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/empty.rs)
- Snapshot discovery: Showcase overview and frozen Jackin compositions.
- [Empty capture plan](../reference/capture-plans/empty.json)
- [Button contract](./button.md), [visual contract](../design/visual-contract.md) and
  [interaction parity](../verification/interaction-parity.md)

The frozen source currently exposes EmptyState with empty/error title and
hint variants. The target Readiness model makes loading, partial and retry
eligibility explicit; each new state is verified as an extension where the
baseline has no equivalent.

## Target public API

    Empty::new(id: Id, state: Readiness<'a>) -> Empty<'a>

    update(&self, cx: &mut Cx<'_>) -> Response<EmptyAction>
    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    title(&'a str)
    detail(&'a str)
    action(Option<ActionMeta<'a>>)
    patch(StylePatch)

    EmptyAction::Invoke {
        action: ActionKey,
        origin: ActivationOrigin,
    }

Readiness is borrowed and controlled. Empty stores no durable readiness state.
A message-only state has no focus or hit region. With action, the child Button
owns focus, hover, pointer capture and activation feedback; Empty forwards its
typed action without performing the domain operation.

## Ownership and phases

| Concern | Owner |
| --- | --- |
| Readiness, title, detail and retry metadata | Caller, borrowed |
| Durable focus and Button interaction | Runtime plus child Button state |
| Layout and wrapping | measure, shared layout and text foundations |
| Status styling | Theme and explicit part patches |
| Retry/network/domain work | Application |

update delegates eligible events to the optional Button. draw paints the status
and child without semantic mutation. measure includes detail wrapping and the
Button only when present. Partial readiness leaves existing rows visible and
adds a status/sentinel through the owning collection.

## Visual and interaction contract

Parts are container, icon, title, detail and action. Preserve the frozen quiet
centered title/detail treatment, error marker and baseline spacing. Empty does
not infer a new illustration or product-specific copy. Part patches inherit
the owner surface and affect actual cells.

Use [theme](../foundations/theme.md) semantic tones for empty, loading, partial
and error. Text measurement respects grapheme boundaries and narrow wrapping.
Zero-area drawing is a no-op; no unsigned subtraction may overflow. There is
no component-owned motion. A loading spinner, if composed, receives its
explicit animation sample from the caller/runtime.

## State matrix

| Axis | Required states |
| --- | --- |
| Readiness | empty, loading, partial with real rows, error; retry absent/present |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, narrow detail, narrow then wide |
| Focus/hover | none for message-only; child Button focused/hovered/focus+hover when action exists |
| Activation | pointer down, held, release inside/outside, removed/disabled action, keyboard activation and feedback for child Button |
| Resize | reflow detail and preserve caller/runtime state |
| Color/capability | semantic fallback across all capture capabilities |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W36-01 | Empty/loading/error with and without retry |
| W36-02 | Partial-data sentinel does not erase real rows |
| W36-03 | Owner theme/part override reaches empty-state cells |
| W36-04 | Narrow detail wrapping and defined zero-area behavior |

Record exact cells/styles, cursor visibility, focus/capture owner, typed action
count/target, readiness value and child state. Extension cases must be marked
as such when the frozen source has no corresponding state.

## Foundations and negative tests

Depends on [identity](../foundations/identity.md), [runtime](../foundations/runtime.md),
[layout](../foundations/layout.md), [theme](../foundations/theme.md),
[text](../foundations/text.md), [layers](../foundations/layers.md),
[authoring](../foundations/author.md) and [conformance](../foundations/conformance.md).

Required negative tests:

- partial state cannot discard or hide real rows;
- message-only Empty cannot register focus/hit/capture;
- an action-looking label without a Button child cannot emit an action;
- retry action cannot run during draw;
- zero/tiny rectangles cannot panic or write outside area;
- theme/part patches cannot silently be ignored or recolor unrelated cells.
