# ProgressBar

**Inventory:** W37 · feedback · P5 · baseline component\
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

ProgressBar paints a deterministic progress sample. The caller owns operation
progress, lifecycle, cancellation and timing. The component never starts a
job or timer.

### Non-goals

- Starting, polling, pausing or cancelling an operation.
- Emitting cancel/retry actions; those are separate Buttons.
- Advancing animation from draw count or wall-clock access.
- Replacing the baseline bar with a new visual style.

## Oracle and provenance

- [Frozen widget source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/progress.rs)
- Snapshot discovery: Showcase progress and Jackin cockpit.
- [ProgressBar capture plan](../reference/capture-plans/progress-bar.json)
- [Motion rules](../foundations/runtime.md), [theme](../foundations/theme.md) and
  [visual parity proof](../verification/visual-parity.md)

The frozen source's render_bar helper is evidence for baseline geometry and
tones. Its current Rust names are not future compatibility requirements.

## Target public API

    ProgressBar::new(id: Id, value: ProgressValue) -> ProgressBar<'a>

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    label(&'a str)
    status(ProgressStatus)       // Active, Done, Error, Paused
    animation(AnimationSample)
    patch(StylePatch)

ProgressValue accepts a finite determinate fraction or an explicit
indeterminate sample. Nonfinite input is rejected before rendering. No
durable component state or typed action is required.

## Ownership and phases

| Concern | Owner |
| --- | --- |
| Value, status, label and lifetime | Caller, borrowed/value props |
| Animation phase and motion policy | Runtime/caller AnimationSample |
| Geometry | Shared measure and draw calculation |
| Theme tones and part patches | Termrock theme |
| Cancel/retry/pause | Separate Button/application action |

There is no update method. draw consumes the supplied sample without
advancing it. measure and draw share label, track, suffix and clipping rules.

## Visual contract

Parts are container, label, fill, rest, percentage and suffix. Determinate
fill uses round(track width × ratio), a rounded integer percentage and a fixed
two-cell suffix column. Preserve suffixes: blank for active, check for done,
exclamation for error and double bar for paused.

The baseline label is omitted unless available width exceeds label width plus
eight cells. A remaining track shorter than six cells renders percentage only.
Active bars use secondary text tone; done is success, error is error and pause
is muted. Green is reserved for completion.

Indeterminate rendering is characterized over the complete source phase cycle
for each tested width. It must not be reduced to one arbitrary still. Unicode
track glyphs, grapheme-safe labels, clipping, color fallback and no-color
behavior follow [text](../foundations/text.md) and [theme](../foundations/theme.md).

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, label boundary, track boundary, narrow then wide |
| Focus/hover | Not applicable; display-only |
| Activation | Not applicable; adjacent controls own actions |
| Value/status | 0, fractional rounding boundaries, 50, 100, active, done, error, paused, indeterminate |
| Motion | all unique phases, phase wrap, before/at/after boundary, paused and reduced motion, same-time repaint |
| Color/capability | semantic tone fallback across truecolor, 256, 16, none and nocolor |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W37-01 | 0%, fractional rounding boundaries, 50%, 100% and completed value |
| W37-02 | Active/done/error/paused with aligned suffix column |
| W37-03 | Label width+8 and width+9; track length 5 versus 6 |
| W37-04 | Indeterminate full phase cycle at narrow, normal and wide sizes |
| W37-05 | Pause/resume/reduced motion do not advance from draw count |

Record exact cells/styles, dimensions, capability, cursor (none), focus and
capture (none), animation sample and status. A missing or invalid oracle
capture is blocked evidence, never a passing result.

## Foundations and negative tests

Depends on [layout](../foundations/layout.md), [theme](../foundations/theme.md),
[text](../foundations/text.md), [runtime](../foundations/runtime.md),
[authoring](../foundations/author.md) and [conformance](../foundations/conformance.md).

Required negative tests:

- NaN and infinity are rejected before any cell is painted;
- draw cannot advance animation or mutate caller state;
- measured width/height and painted track boundaries agree;
- narrow areas cannot underflow or write outside area;
- status tones cannot recolor unrelated surfaces;
- a progress bar cannot emit operation controls or domain actions.
