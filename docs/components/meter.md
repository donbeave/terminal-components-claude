# Meter

**Inventory:** W39 · feedback · P5 · baseline component\
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

Meter presents caller-supplied capacity or consumption information in a
compact line or filled-block visual. The caller decides whether a value means
used or remaining, its freshness and domain status.

### Non-goals

- Inferring missing capacity as zero.
- Treating capacity as operation completion.
- Fetching quota, currency, reset times or provider data.
- Synthesizing success/green from a full value.

## Oracle and provenance

- [Frozen progress source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/progress.rs)
- [Frozen usage composition](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/bin/jackin_preview/screens/usage.rs)
- Snapshot discovery: Jackin usage and accounts.
- [Meter capture plan](../reference/capture-plans/meter.json)

## Target public API

    Meter::new(id: Id, value: Option<Percent>) -> Meter<'a>

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    readout(&'a str)
    visual(MeterVisual)       // Line or Block
    tone(MeterTone)
    animation(AnimationSample)
    patch(StylePatch)

Value and tone are controlled props; Meter has no durable state or typed
action. Refreshing may compose Spinner using the explicit animation sample.

## Ownership and phases

| Concern | Owner |
| --- | --- |
| Optional value, readout and domain meaning | Caller |
| Threshold or explicit warning/error classification | Caller via MeterTone |
| Refresh phase and motion policy | Runtime/caller AnimationSample |
| Line/block geometry and painting | Meter |
| Quota/network/provider lifetime | Application |

There is no update method. draw does not infer, fetch or mutate status.
measure and draw share rounding, readout placement, suffix width and clipping.

## Visual contract

Two modes are required: line and filled block. Preserve embedded readout
placement, clipping, fill rounding and fixed suffix width. Default consumption
thresholds are <=59 low, <=84 medium and >84 high. Domain-explicit
Warning, Exhausted, Stale, Refreshing, Error and Unknown override generic
thresholds.

The compact line recipe is a `━` filled run followed by the remaining `─`
track, then a two-cell status/readout suffix (for example,
`━━━━────  38%`). The block recipe fills the used share with its level tone,
places the value inside in dark bold text, and leaves the remainder one plane
up. Both are variants of this one component. Low is 0–59%, medium 60–84%, and
high 85–100%; the caller's explicit domain status takes priority over that
threshold table. Unknown has no run/bar and shows faint `—`; refreshing uses a
spinner in place of the value. Capacity never uses completion green.

Capacity is not completion: no green success fill. Unknown draws no invented
track; it uses an explicit unavailable presentation. Error and refresh remain
distinct. Readout text is already safe caller data.

Parts are container, fill, rest, value and suffix. Unicode readouts, grapheme
width, narrow geometry, semantic tones and capability fallback follow
[text](../foundations/text.md), [layout](../foundations/layout.md) and
[theme](../foundations/theme.md).

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, readout boundary, narrow then wide |
| Focus/hover | Not applicable; display-only |
| Activation | Not applicable |
| Value | 0, 59, 60, 84, 85, 100 and unknown |
| Tone | normal, warning, exhausted, stale, refreshing, error, unknown |
| Motion | full refresh spinner phases, paused/reduced motion and same-sample repaint |
| Meaning | used versus remaining must be explicitly supplied |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W39-01 | 0, 59, 60, 84, 85, 100 and unknown |
| W39-02 | Line and Block layouts at rounding/readout boundaries |
| W39-03 | Warning/exhausted/stale/refreshing/error/unknown for one value |
| W39-04 | Refresh spinner full cycle; unknown has no bar |
| W39-05 | Remaining versus used meaning is explicit caller data |

Record exact cells/styles, value meaning, tone, sample, cursor/focus/capture
(none), dimensions and capability. Capture each semantic tone independently;
do not accept threshold inference as a substitute for explicit domain states.

## Foundations and negative tests

Depends on [runtime](../foundations/runtime.md), [layout](../foundations/layout.md),
[theme](../foundations/theme.md), [text](../foundations/text.md),
[authoring](../foundations/author.md) and [conformance](../foundations/conformance.md).

Required negative tests:

- None cannot render as zero or invent a track;
- full capacity cannot become completion green;
- caller readout cannot be reinterpreted or formatted as a domain value;
- draw cannot advance refresh motion or call services;
- line/block measurement and clipping cannot disagree;
- phase changes cannot alter declared layout width;
- zero/tiny geometry cannot underflow or write outside area.
