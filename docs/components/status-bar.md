# StatusBar

**Inventory:** W40 · chrome · P5 · baseline component\
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

StatusBar paints one full-width status row from caller-owned left, center and
right groups. It carries labels, values, priorities, tones and optional action
metadata. It is the single future engine for the old StatusBar and Segments
surfaces.

### Non-goals

- Service polling, account knowledge or provider interpretation.
- A second Segments painter.
- Focus stops for noninteractive metadata.
- Synthesizing success from missing or stale values.

## Oracle and provenance

- [Frozen statusbar source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/statusbar.rs)
- [Frozen segments source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/segments.rs)
- Snapshot discovery: Showcase chrome and Jackin capsule.
- [StatusBar capture plan](../reference/capture-plans/status-bar.json)

The second source is a legacy family disposition: Segments is absorbed into
StatusBar. It must not remain a competing rendering engine.

## Target public API

    StatusBar::new(
        id: Id,
        groups: &'a [StatusGroup<'a>],
        revision: Revision,
    ) -> StatusBar<'a>

    update(&self, cx: &mut Cx<'_>) -> Response<StatusAction>
    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    animation(AnimationSample)
    patch(StylePatch)
    patch_part(Part, StylePatch)

    StatusAction::Invoke {
        key: ItemKey,
        action: ActionKey,
        origin: ActivationOrigin,
    }

Groups and items are borrowed. Every interactive item has a stable ItemKey;
noninteractive items are not focus stops. Runtime owns keyed hover/press and
feedback state; StatusBar stores no value state.

## Ownership and phases

| Concern | Owner |
| --- | --- |
| Group contents, labels, values, priorities and action metadata | Caller |
| Stable item identity and reconciliation | Identity/collections foundations |
| Focus, hit testing, hover, press and pointer capture | Runtime |
| Priority fit and left/center/right geometry | StatusBar |
| Spinner phase | Runtime/caller AnimationSample |
| Service/account state | Application |

update resolves runtime events to stable item keys and returns typed actions.
draw is pure with respect to the borrowed groups and runtime state.
measure and draw share truncation, priority-drop and gap calculations.

## Visual and interaction contract

Parts are container, group, item, label, value, icon and separator. Preserve
the frozen row's group spacing and tones. There are no separator glyphs
between groups; spacing carries the separation. The baseline uses a
three-cell gap and one-cell edge inset.

At narrow widths, drop the lowest-priority surviving item in the baseline
order: center first, then right, then left; the strongest left item survives
and truncates if necessary. A spinner changes only its declared cells and
cannot change item width unpredictably.

Hover lifting, press feedback, focus gutter and pointer capture apply only to
interactive items. Keyboard-suppressed hover and release outside follow the
[runtime](../foundations/runtime.md) contract. Unicode labels/readouts and
capability fallback use [text](../foundations/text.md) and
[theme](../foundations/theme.md).

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, Unicode content, narrow then wide |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover |
| Activation | pointer down, held, release inside/outside, removed target, disabled item, keyboard activation, feedback timing |
| Source | empty group, dynamic update, reorder/removal, stale item key |
| Motion | all unique spinner phases, phase boundary, paused/reduced, stable width |
| Color/capability | semantic item tones across all capture capabilities |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W40-01 | Left/center/right alignment at exact fit |
| W40-02 | Narrow width drops items in declared priority order |
| W40-03 | Interactive item hover/click versus noninteractive item |
| W40-04 | Spinner frames preserve gaps and stable width |
| W40-05 | Dynamic removal during pointer press cannot retarget |

Record exact cells/styles, selected stable item key, navigation key, cursor,
focus owner, capture owner, action target/count, revision and animation sample.

## Foundations and negative tests

Depends on [identity](../foundations/identity.md), [runtime](../foundations/runtime.md),
[layout](../foundations/layout.md), [theme](../foundations/theme.md),
[collections](../foundations/collections.md), [input/actions](../foundations/input-actions.md),
[authoring](../foundations/author.md) and [conformance](../foundations/conformance.md).

Required negative tests:

- Segments cannot create a parallel painter or conflicting fit order;
- a noninteractive item cannot receive focus or emit an action;
- removal during capture cannot activate a replacement item;
- duplicate/missing item keys fail reconciliation;
- priority dropping cannot change a surviving item's stable geometry unexpectedly;
- draw cannot poll services, mutate groups or advance animation.
