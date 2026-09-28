# KeyHint

**Inventory:** W42 · chrome · P2 · baseline component  
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

KeyHint renders a typed Chord and its label using baseline modifier glyphs,
multi-key notation and exact display width. It is a decorative leaf used by
menus, help, hints and other chrome.

### Non-goals

- Parsing display strings into event bindings.
- Handling input or owning a focus/hit target.
- Creating a second shortcut formatter in a parent component.
- Splitting graphemes or key glyph groups during clipping.

## Oracle and provenance

- [Frozen keyhint source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/keyhint.rs)
- Snapshot discovery: Showcase chrome.
- [KeyHint capture plan](../reference/capture-plans/key-hint.json)
- [Input/action contract](../foundations/input-actions.md) and
  [visual parity proof](../verification/visual-parity.md)

## Target public API

    KeyHint::new(chord: &'a Chord, label: &'a str) -> KeyHint<'a>

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builder:

    patch(StylePatch)

KeyHint has no durable state, update method or typed action. The Chord and
label are borrowed. The same Chord formatter is used in every parent context.

## Ownership and rendering

| Concern | Owner |
| --- | --- |
| Typed chord and label | Caller/binding view |
| Chord formatting and width | KeyHint |
| Focus, hover, hit and capture | None; parent metadata remains decorative |
| Theme and part styles | Termrock theme |
| Clipping and geometry | Shared layout/text foundations |

Parts are chord, modifier, key and label. Exact-fit and one-cell-short
measurement must agree with painting. Unicode labels, combining marks,
grapheme-safe clipping and capability fallback follow [text](../foundations/text.md)
and [theme](../foundations/theme.md).

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, Unicode label, narrow then wide |
| Focus/hover | Not applicable; no independent focus stop |
| Activation | Not applicable; decorative |
| Binding | single key, modified key, multi-key prefix, disabled/descriptive parent notation |
| Composition | identical rendering in Menu, HelpOverlay and HintBar |
| Color/capability | parent/theme fallback in every capture capability |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W42-01 | Single key, modified key, multi-key prefix and Unicode label |
| W42-02 | Exact fit and one-cell-short measure/paint consistency |
| W42-03 | Disabled/descriptive binding notation supplied by parent |
| W42-04 | One Chord renders identically in menu, help and hint contexts |

Record exact cells/styles, chord input, label input, dimensions/capability,
cursor (none), focus/capture (none) and action count (zero).

## Foundations and negative tests

Depends on [identity](../foundations/identity.md), [input/actions](../foundations/input-actions.md),
[layout](../foundations/layout.md), [theme](../foundations/theme.md),
[text](../foundations/text.md), [authoring](../foundations/author.md) and
[conformance](../foundations/conformance.md).

Required negative tests:

- display text cannot be parsed as a replacement binding;
- clipping cannot split graphemes or key glyph groups;
- width returned by measure cannot differ from draw width;
- KeyHint cannot register focus, hit or pointer capture;
- a Chord cannot render differently solely because its parent is Menu or Help.

