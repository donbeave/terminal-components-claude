# Field

**Component ID:** W07\
**Group:** Forms\
**Phase:** P3\
**Contract status:** canonical Termrock target; implementation is future work on `termrock-refactor`.

## Purpose and scope

`Field` is shared form chrome. It owns label, required marker, help/error line, and child allocation. It never owns a second editor, field value, validation state, or independent focus stop. Child components own their own identity, state, input, cursor, and responses.

### Non-goals

- No input, textarea, select, editor, or validation engine inside Field.
- No extra focus/hit region around a child.
- No duplicate field painter in composite input constructors.
- No product form schema or persistence.

## Oracle and provenance

The immutable baseline is [`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`](https://github.com/donbeave/terminal-components-claude/commit/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b). Current shared behavior is distributed across [`src/widgets/input.rs`](../../src/widgets/input.rs), [`src/widgets/textarea.rs`](../../src/widgets/textarea.rs), [`src/widgets/select.rs`](../../src/widgets/select.rs), and [`src/widgets/field_common.rs`](../../src/widgets/field_common.rs). Their source blobs are `f440b74c7d305e987ec31bf04d34afd993844bb0`, `5bfa5148a8b335c28ed52a5af305fa9df39917e5`, `3b6f43f8f642ccce512ac53754d114dc978bd149`, and `bd41cdd85e9c2605c356f7468dc0de3e3c50e54b`. The W07 plan is [`capture-plans/field.json`](../reference/capture-plans/field.json), planned and uncaptured.

## Public API and ownership

```rust
Field::new(label: &'a str) -> Field<'a>

field.draw(
    &mut ui,
    area,
    |ui, child_area| child.draw(ui, child_area),
) -> R
field.measure(&measure_cx, child_size, constraints) -> Size
```

Builders: `help(&'a str)`, `error(Option<&'a ValidationMessage>)`, `required(bool)`, `plain_label(bool)`, `child_id(Id)`, and `patch(StylePatch)`.

Field has no durable state and no typed action. The child response passes through unchanged. The child owns focus, capture, editing, hit target, and state; `child_id` supplies semantic attribution without creating a second stop.

## Update, draw, and measure

- Field has no independent update phase. The parent updates the child using the shared runtime and forwards the child response.
- `draw` allocates the baseline label/input/help-error rows, then invokes the real child painter once. Error replaces the help line without moving the child unexpectedly.
- `measure` computes label and help/error geometry around the child size and constraints. It clips safely at zero/tiny heights and preserves the tagged single-line contract.

## Identity and reconciliation

Field is a wrapper, not an interactive owner. The child's stable `Id` remains the sole focus/hit identity. Hidden/removed child fields release child runtime records. Reordering a form preserves child IDs and cannot move focus or capture to a neighboring field. A labelled input produces one Tab stop.

## Visual contract

Parts are `label`, `required`, `help`, and `error`. Preserve baseline label offset, required/optional width rule, single-line allocation (label + child row + help/error row), error replacement, semantic tones, focus gutter/cursor from the child, and clipping. [`theme`](../foundations/theme.md) owns style precedence. Field does not expose slots for geometry, child focus, or whole-surface replacement.

## Interaction and state rules

| Area | Required behavior |
|---|---|
| Focus/hover | Not applicable at Field surface. It may reflect child focus but creates no focus stop or hover owner. |
| Pointer | Not applicable at Field surface. Pointer events go to the child hit region. |
| Activation | Not applicable. Child actions pass through. |
| Disabled/read-only | Child eligibility and visuals remain authoritative. Field label/help/error reflect caller props without intercepting child events. |
| Editing | Child-specific; Field does not start, commit, or cancel edits. |
| Resize | Reallocate label and child rows deterministically; preserve child geometry and cursor. |
| Unicode | Measure label/help/error and child area by display cells; suffixes disappear at the baseline width boundary rather than partial clipping. |
| Capability/color | Semantic label, help, required, error, and surface styles resolve through the theme in all color modes. |
| Motion | No Field-owned timing or animation. |

## Applicable state matrix

| Axis | Cases |
|---|---|
| Geometry | normal, zero area, tiny area, nonzero origin, exact fit, one cell short, long Unicode, narrow then wide |
| Focus/hover | not applicable to Field; child focus/hover is covered by child contract |
| Activation | not applicable to Field; child gesture cases apply |
| Child composition | help vs error, required vs optional, child editing/error, one Tab stop |

Run geometry and capability cases at 72×20, 80×24, 100×30, 120×40, and 160×50. Do not fabricate interaction states for this display-only wrapper.

## Required capture cases

| Case | Exact requirement |
|---|---|
| W07-01 | Label/help versus label/error at identical allocated height. |
| W07-02 | Required and optional suffix at its width boundary. |
| W07-03 | Child editing with error preserves the child focus gutter and cursor. |
| W07-04 | One Tab stop for a labelled input, not two. |

Record child focus/capture/layer owner and child action/state in each case. Field has no independent action count.

## Foundation and component dependencies

- [`identity`](../foundations/identity.md), [`layout`](../foundations/layout.md) — child identity and row measurement.
- [`theme`](../foundations/theme.md) — label, required, help, error, and surface semantics.
- [`author`](../foundations/author.md) — one child painter and constrained parts.
- [`conformance`](../foundations/conformance.md) — wrapper/child parity and negative proof.
- [`TextInput`](text-input.md), [`TextArea`](text-area.md), and [`Select`](select.md) — child contracts; none may duplicate Field chrome.

## Negative tests and acceptance

- Field cannot create a second focus/hit stop or mutate child state.
- Child is painted exactly once; no decorative replacement hides the child.
- Error and help occupy the same row allocation at the baseline height.
- Required/optional suffix is either wholly present or omitted at the width boundary.
- Child cursor/focus remains visible through Field error rendering.
- Candidate output cannot become the expected wrapper oracle.

Accept after child composition, geometry, exact snapshots, single-focus, Unicode, and independent review pass.
