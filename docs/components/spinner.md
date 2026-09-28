# Spinner

**Inventory:** W38 · feedback · P5 · baseline component  
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

Spinner paints one deterministic phase supplied by the caller/runtime. It
does not read a wall clock, create a task or decide cadence.

### Non-goals

- Owning time, timers or an async job.
- An implicit universal cadence for every application.
- Focus, hover, press or click behavior.
- Replacing an application status or error policy.

## Oracle and provenance

- [Frozen widget source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/progress.rs)
- Snapshot discovery: Showcase progress and Jackin cockpit.
- [Spinner capture plan](../reference/capture-plans/spinner.json)
- [Runtime/time contract](../foundations/runtime.md) and
  [visual parity proof](../verification/visual-parity.md)

## Target public API

    Spinner::new(id: Id, sample: AnimationSample) -> Spinner

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    status(SpinnerStatus)
    patch(StylePatch)

There is no update method, durable SpinnerState or typed action. The sample
determines the phase and explicit status determines stopped, paused or
reduced-motion rendering.

## Ownership and phases

| Concern | Owner |
| --- | --- |
| Epoch, tick, cadence and motion policy | Runtime/caller |
| Status and semantic tone | Caller |
| Phase glyph selection | Spinner |
| Focus, hit and capture | Parent/runtime; Spinner owns none |
| Surrounding padding and separators | Parent component |

draw is a pure projection of the sample. Rendering twice with the same sample
produces the same cell. measure uses the same glyph width as draw.

## Visual contract

The baseline has exactly ten frames, in order:

    ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏

Frame selection is modulo ten. Preserve the glyphs, width, tone and clipping.
The phase schedule comes from the owner; it is not hard-coded as one
wall-clock interval. When embedded in StatusBar or Meter, the frame occupies
its declared cells and cannot change layout width.

Spinner is display-only. There is no independent focus, hover, pressed,
active/clicked state. Unicode width and terminal capability fallback follow
[text](../foundations/text.md) and [theme](../foundations/theme.md).

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, narrow then wide |
| Focus/hover | Not applicable; no independent focus stop |
| Activation | Not applicable |
| Motion | all ten phases, wrap 9→0, before/at/after owner boundary, paused, reduced, stopped, same-time repaint |
| Composition | embedded in StatusBar and Meter with exact gaps and stable width |
| Color/capability | status/theme fallback in every capture capability |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W38-01 | All ten phases plus wrap from 9 to 0 |
| W38-02 | Immediately before, at and after each owner cadence boundary |
| W38-03 | Same time/sample rendered twice gives identical cells |
| W38-04 | Spinner inside StatusBar and Meter preserves separator gaps |
| W38-05 | Paused, reduced and stopped semantics |

Record exact cells/styles, sample/time input, cursor (none), focus/capture
owners (none), dimensions and capability. Timing is an input to proof, never
inferred from draw count.

## Foundations and negative tests

Depends on [runtime](../foundations/runtime.md), [theme](../foundations/theme.md),
[text](../foundations/text.md), [layout](../foundations/layout.md),
[authoring](../foundations/author.md) and [conformance](../foundations/conformance.md).

Required negative tests:

- draw cannot call wall-clock APIs or mutate the sample;
- equal samples produce equal output;
- frames cannot be skipped or reordered;
- a spinner cannot register focus/hit/capture;
- a narrow area cannot panic or paint outside bounds;
- embedding cannot change measured width across phases.

