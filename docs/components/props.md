# Props

**Inventory:** W34 · data and text · P3 · baseline component  
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

Props paints a static label/value sheet. The caller supplies already-safe
display values, tones, wrapping policy and any protected-value presentation.
It preserves the baseline two-column alignment and continuation rows.

Props is display-only. It has no focus stop, pointer target, cursor,
selection, copy action or domain introspection. PropsList owns the
interactive variant and shares this row painter.

### Non-goals

- Inspecting a domain struct or deriving labels from reflection.
- Calling Debug or Display on secret values.
- Copying, activating, scrolling or changing caller-owned rows.
- A second row layout engine inside a parent component.

## Oracle and provenance

- [Frozen widget source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/props.rs)
- Snapshot discovery: Jackin manager and accounts views at the frozen commit.
- [Props capture plan](../reference/capture-plans/props.json)
- [Visual contract](../design/visual-contract.md) and [visual parity proof](../verification/visual-parity.md)

The current src/widgets/props.rs is an oracle only. Its current Rust
identifiers are implementation names and do not define the future public API.

## Target public API

    Props::new(id: Id, rows: &'a [PropsRow<'a>]) -> Props<'a>

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    label_width(LabelWidth)
    patch(StylePatch)
    patch_part(Part, StylePatch)

PropsRow contains a stable row key, borrowed label, typed PropsValue and
display tone. PropsValue must distinguish ordinary text, styled text, an
explicit empty value and a protected/redacted value. A protected value carries
only its safe display form.

There is no update method because the surface has no durable interaction.
If a caller needs an action or navigation, it uses PropsList.

## Ownership and rendering

| Concern | Owner |
| --- | --- |
| Rows, values, tones and wrap policy | Caller, borrowed for the frame |
| Label width and row measurement | Props::measure and the shared layout foundation |
| Theme resolution and part patches | Termrock theme |
| Focus, hit testing and pointer capture | Runtime; unused by static Props |
| Painting | Props::draw, with no semantic mutation |

measure and draw use the same label-width and wrapping calculation. The label
column is shared across rows; wrapped values create continuation rows with no
repeated label. Clipping is constrained to area, including zero and
nonzero-origin rectangles.

The default label column is the widest label plus two cells, measured in
display width. Keep values aligned to that column; wrapped continuation lines
omit the label rather than repeating or indenting a second label column.

## Visual contract

Parts with supported style patches are container, label, value, separator and
status. Preserve the frozen baseline's label alignment, continuation spacing,
value emphasis, empty-value treatment, masking glyphs, surface color and
clipping. Part patches affect the cells they name and cannot replace geometry
or focus ownership.

The component must render correctly for Unicode labels and values, grapheme
boundaries, long wrapped values, narrow widths and all declared terminal color
capabilities (truecolor, 256, 16, no color and nocolor). Theme fallback
follows [theme](../foundations/theme.md); text width and wrapping follow
[text](../foundations/text.md).

## State and behavior matrix

| Axis | Required behavior |
| --- | --- |
| Geometry | normal, zero area, 1-cell area, nonzero origin, exact fit, 1-cell short, long Unicode content, narrow then wide |
| Focus/hover | Not applicable; no focus stop or hover state |
| Activation | Not applicable; no pointer or keyboard action |
| Editing/selection | Not applicable; values are caller-controlled |
| Resize | Re-measure and wrap from the new constraints without mutating rows |
| Motion | Not applicable |
| Color/capability | Preserve semantic tones and baseline fallback in every capture capability |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W34-01 | Long label and wrapped value at narrow width |
| W34-02 | Mixed styled, plain, masked and empty values |
| W34-03 | Nonzero origin and clipped continuation line |
| W34-04 | Static Props creates no focus or pointer activation |

Each capture records dimensions, exact cells and styles, cursor visibility
(none), focus owner (none), capture owner (none), action count (zero) and all
source values. Use the dimensions and capabilities in the capture plan.

## Foundations and negative tests

Depends on [identity](../foundations/identity.md), [layout](../foundations/layout.md),
[theme](../foundations/theme.md), [text](../foundations/text.md) and
[authoring](../foundations/author.md).

Required negative tests:

- secret references cannot leak raw values through formatting or debug paths;
- draw cannot mutate rows or semantic state;
- measured height equals painted height for wrapping and clipping;
- no cell outside area is written;
- static Props never registers focus, hit or capture ownership;
- unsupported part patches cannot replace geometry or surface ownership.
