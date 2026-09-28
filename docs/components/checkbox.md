# Checkbox

**Component ID:** W03  
**Group:** Controls  
**Phase:** P3  
**Contract status:** canonical Termrock target; implementation is future work on `termrock-refactor`.

## Purpose and scope

`Checkbox` presents one controlled boolean choice. The caller owns the label and checked value. Termrock reports the requested next value and never mutates a domain model or retains an uncontrolled copy.

### Non-goals

- No form model, persistence, validation schema, or application-specific side effects.
- No focus state stored in the component.
- No generic button or switch recipe; marker and focus gutter preserve the baseline choice-control geometry.

## Oracle and provenance

The oracle is frozen at [`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`](https://github.com/donbeave/terminal-components-claude/commit/4a79c0a2d40fca46fc406b77157ce3b3f12ec16b). Current source is [`src/widgets/choice.rs`](../../src/widgets/choice.rs), legacy `Checkbox`, source blob `9405acf87c152c0766ae9bbcd71c01c9d5db4e80`. Showcase settings and choice pages provide baseline consumers.

The W03 capture contract is [`capture-plans/checkbox.json`](../reference/capture-plans/checkbox.json). It is planned and has no approved artifacts. Bind source output before candidate testing.

## Public API and ownership

```rust
Checkbox::new(id: Id, label: &'a str, checked: bool) -> Checkbox<'a>

checkbox.update(&mut cx) -> Response<ValueChanged<bool>>
checkbox.draw(&mut ui, area) -> Rect
checkbox.measure(&measure_cx, constraints) -> Size
```

Builders: `disabled(bool)`, `status(ControlStatus)`, `patch(StylePatch)`, and `patch_part(Part, StylePatch)`.

The checked value is borrowed controlled input. Durable component state is none. Runtime owns focus, hover, capture, and press timing. The typed action is `ValueChanged<bool> { value, origin: ActivationOrigin }`.

## Update, draw, and measure

- `update` handles Enter, Space, and completed pointer clicks. It requests the inverse controlled value once per eligible gesture. If the caller declines to apply the action, the next draw still shows the caller's value.
- `draw` paints the focus gutter, marker, and label inside the clipped row. Focus never changes `checked`. No semantic state changes occur during draw.
- `measure` accounts for the gutter, compact/full marker, label, and clipping by display cells.

## Identity and reconciliation

The stable `Id` owns focus/hit/capture records. Adjacent controls receive disjoint clipped rectangles. Removing or disabling a checkbox cancels capture and hover. Reordering controls preserves identity and cannot transfer an action. No index-based state is retained.

## Visual contract

Parts are `container`, `gutter`, `marker`, and `label`. Preserve baseline marker glyphs and spacing: full rows use `[✓]`/`[ ]`; narrow rows use compact `✓`/`□` as applicable. The focus gutter occupies a different cell from the checked marker. Disabled colors, hover lift, semantic surfaces, and clipping come from [`theme`](../foundations/theme.md). Part patches cannot replace geometry, focus/capture, or the whole surface.

## Interaction and state rules

| Area | Required behavior |
|---|---|
| Keyboard | Enter/Space requests the inverse value once. Focus alone never changes it. |
| Pointer | Marker, label, and last valid clipped cell are eligible. Down plus release inside requests one change; release outside, removed, or adjacent cells do not. |
| Focus/hover | Runtime owns focus and hover. Keyboard suppresses stale hover; pointer motion restores it. Focus and checked marker are independent. |
| Disabled/read-only | Disabled checked and unchecked controls ignore activation, clear hover/press state, and emit no value change. Read-only callers pass disabled/eligibility policy; no hidden mutation occurs. |
| Editing | Not applicable. |
| Resize/Unicode | Clip label and marker to allocation; measure combining marks and wide glyphs by cells. |
| Capability/color | Semantic accent, muted, disabled, and surface styles must resolve in all supported color modes. |
| Motion | Use shared activation feedback only where the baseline applies; no private animation. |

## Applicable state matrix

| Axis | Cases |
|---|---|
| Geometry | normal, zero area, tiny area, nonzero origin, exact fit, one cell short, long Unicode, narrow then wide |
| Focus/hover | unfocused, focused, hovered, focus+hover, keyboard-suppressed hover, pointer motion restores hover |
| Activation | pointer down, held press, release inside, release outside, removed target, disabled target, keyboard activation, feedback active/expired when applicable |
| Value | checked/unchecked, controlled caller accepts action, controlled caller rejects action |

Run applicable cases at all five capture dimensions and truecolor, 256, 16, none, and nocolor capabilities.

## Required capture cases

| Case | Exact requirement |
|---|---|
| W03-01 | Checked and unchecked crossed with independent focus and hover. |
| W03-02 | Disabled checked and disabled unchecked ignore all activation. |
| W03-03 | Click marker, label, last valid cell, and immediately outside. |
| W03-04 | Caller rejects the change; checkbox does not drift into an uncontrolled value. |

Record exact cells, cursor, focus/capture/layer owner, action count/target, and controlled value before/after.

## Foundation dependencies

- [`identity`](../foundations/identity.md), [`input-actions`](../foundations/input-actions.md) — stable identity and typed value-change origin.
- [`runtime`](../foundations/runtime.md) — focus, hover, hit testing, pointer capture, and timing.
- [`layout`](../foundations/layout.md) — clipped row and display-cell measurement.
- [`theme`](../foundations/theme.md) — choice surfaces, markers, and capability mapping.
- [`author`](../foundations/author.md) — part patches without parallel renderers.
- [`conformance`](../foundations/conformance.md) — source/output proof and negative tests.

## Negative tests and acceptance

- Focus, hover, or draw cannot mutate the checked value.
- A caller rejection leaves the next render equal to the caller input.
- Disabled controls cannot capture, press, or emit an action.
- Adjacent rows never receive a neighboring checkbox's click.
- Unicode clipping cannot write outside the rectangle.
- Candidate code cannot accept its own output as expected output.

Accept after the external API example, source-state tests, exact applicable snapshots, boundary hit tests, and independent review pass.
