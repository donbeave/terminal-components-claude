# HintBar

**Inventory:** W41 · chrome · P2 · baseline component  
**Baseline:** 4a79c0a2d40fca46fc406b77157ce3b3f12ec16b

## Purpose and scope

HintBar paints the one shell-owned bottom hint row from effective action and
binding metadata. The same binding view drives input, Menu and HelpOverlay.
DerivedHintBar is an adapter/factory that selects metadata; it is not a
second painter.

### Non-goals

- Parsing user-visible strings back into event bindings.
- Creating controls or duplicate actions from descriptive hints.
- Allowing each child/modal to paint another footer row.
- Reimplementing KeyHint formatting.

## Oracle and provenance

- [Frozen hintbar source](https://github.com/donbeave/terminal-components-claude/blob/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b/src/widgets/hintbar.rs)
- Snapshot discovery: Showcase chrome and Jackin compositions.
- [HintBar capture plan](../reference/capture-plans/hint-bar.json)
- [KeyHint contract](./key-hint.md), [runtime layers](../foundations/layers.md) and
  [visual parity proof](../verification/visual-parity.md)

## Target public API

    HintBar::new(id: Id, hints: &'a [Hint<'a>]) -> HintBar<'a>
    HintBar::from_bindings(id: Id, bindings: &'a BindingView) -> HintBar<'a>

    draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect
    measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size

Builders:

    alignment(Alignment)
    patch(StylePatch)

Effective binding metadata is borrowed; there is no durable state or typed
action by default. Hints describe actions already handled by the active owner.

## Ownership and rendering

| Concern | Owner |
| --- | --- |
| Effective binding view and labels | Caller/runtime layer resolver |
| Highest-priority owner selection | Layers/runtime |
| Chord formatting | KeyHint |
| Bottom-row placement | Application shell |
| Painting and fit/drop order | HintBar |

The active owner wins in this order: modal/editor/prefix context, then
temporary mode, then active screen, then global fallback. Preserve one global
bottom row. In narrow widths, drop lowest-priority complete hint groups and
never split a key glyph group.

## Visual and interaction contract

Parts are container, chord, label, separator and editing-badge. Preserve
shortcut notation, label spacing, final separator, status/badge placement and
the baseline narrow-width marker. Descriptive hints are noninteractive.

Keyboard, mouse and focus do not target the bar itself. Editing badges reflect
the active owner metadata. Unicode labels and chord display widths follow
[text](../foundations/text.md); theme and color fallback follow
[theme](../foundations/theme.md). There is no component-owned motion.

## State matrix

| Axis | Required states |
| --- | --- |
| Geometry | normal, zero/tiny area, nonzero origin, exact fit, 1-cell short, Unicode label, narrow then wide |
| Focus/hover | Not applicable; decorative metadata has no focus stop |
| Activation | Not applicable; hints do not duplicate controls |
| Owner | navigation, editing, nested modal and prefix contexts |
| Composition | one footer in composite fixtures; no duplicate child bars |
| Color/capability | semantic fallback across capture capabilities |

## Required parity cases

| Case | Required observation |
| --- | --- |
| W41-01 | Normal navigation, editing and nested modal owner selection |
| W41-02 | Remapped chord is shown consistently with actual handling |
| W41-03 | Narrow priority drops do not split key glyph groups |
| W41-04 | Editing badge and final separator spacing |
| W41-05 | Composite fixture has no duplicate bottom hint rows |

Record exact cells/styles, active owner, hint order, dimensions/capability,
cursor (none), focus/capture (none) and action count (zero).

## Foundations and negative tests

Depends on [layers](../foundations/layers.md), [input/actions](../foundations/input-actions.md),
[layout](../foundations/layout.md), [theme](../foundations/theme.md),
[text](../foundations/text.md), [authoring](../foundations/author.md) and
[conformance](../foundations/conformance.md).

Required negative tests:

- DerivedHintBar cannot paint independently of HintBar;
- a lower-priority layer cannot override the active owner;
- descriptive hints cannot create actions;
- narrow fit cannot split a chord or create a second footer;
- drawing cannot mutate binding metadata;
- remapped display cannot diverge from the effective keymap.

