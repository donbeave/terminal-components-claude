# Exhaustive visual review bundle

Read-only ratatui/test-harness capture. No baseline was read for writing, rewritten, or blessed.

- Before commit: `cae32f882697cf92f7bdbe18e8292e7d1ff47a60`
- After commit: `18d37bec6e460d66f403efa4b14d700647f19b60`
- Changed test cases: 38
- Matrix cells captured: 304 before + 304 after
- State-reference cells captured: 1152 before + 1152 after
- Old capture matches checked-in baseline: 304/304
- Digest changes: 296/304
- Text-frame changes: 236/304
- Style-only digest changes (ambiguous without cell styles): 60
- Collision rows: 88; unclassified collisions: 0

## Case coverage

| Exact test case | Classification | Cells | Digest changes | Text changes | Style-only |
|---|---|---:|---:|---:|---:|
| `render::components::button::selected` | checked-selection-marker | 8 | 4 | 4 | 0 |
| `render::components::chip_bar::focused` | owned-cursor-state | 8 | 8 | 0 | 8 |
| `render::components::code_editor::disabled` | disabled-editor-state | 8 | 8 | 0 | 8 |
| `render::components::completion::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::completion::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::completion::selected` | selected-row-marker | 8 | 8 | 8 | 0 |
| `render::components::context_menu::selected` | selected-row-marker | 8 | 8 | 8 | 0 |
| `render::components::dialog::disabled` | disabled-actions | 8 | 8 | 0 | 8 |
| `render::components::filter_list::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::filter_list::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::filter_list::selected` | selected-row-marker | 8 | 8 | 8 | 0 |
| `render::components::list::disabled` | disabled-items | 8 | 8 | 0 | 8 |
| `render::components::list::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::list::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::menu_bar::disabled` | disabled-menu-title | 8 | 8 | 0 | 8 |
| `render::components::menu_bar::selected` | selected-menu-title-marker | 8 | 8 | 8 | 0 |
| `render::components::nav_list::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::nav_list::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::picker::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::picker::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::picker::selected` | selected-row-marker | 8 | 8 | 8 | 0 |
| `render::components::picker_chain::selected` | selected-stage-state | 8 | 8 | 0 | 8 |
| `render::components::radio_group::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::radio_group::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::steps::default` | step-status-glyph | 8 | 8 | 8 | 0 |
| `render::components::steps::disabled` | disabled-step-state | 8 | 8 | 8 | 0 |
| `render::components::steps::editing` | step-status-glyph | 8 | 8 | 8 | 0 |
| `render::components::steps::focused` | step-status-glyph | 8 | 8 | 8 | 0 |
| `render::components::steps::hovered` | step-status-glyph | 8 | 8 | 8 | 0 |
| `render::components::steps::pressed` | step-status-glyph | 8 | 8 | 8 | 0 |
| `render::components::steps::selected` | step-status-glyph | 8 | 8 | 8 | 0 |
| `render::components::tabs::focused` | owned-tab-cursor-state | 8 | 8 | 0 | 8 |
| `render::components::too_small::empty` | required-product-copy | 8 | 8 | 8 | 0 |
| `render::components::tree::focused` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::tree::pressed` | owned-cursor-state | 8 | 8 | 8 | 0 |
| `render::components::wizard::disabled` | disabled-step-state | 8 | 4 | 0 | 4 |
| `render::components::wizard::focused` | current-step-state | 8 | 8 | 8 | 0 |
| `render::components::wizard::pressed` | current-step-state | 8 | 8 | 8 | 0 |

## Collision interpretation

See `collisions.tsv`. Rows marked `documented-exemption-candidate` are emitted for review even when the source gate permits them. Rows marked `UNEXPECTED-COLLISION` are blockers.

## Evidence layout

`before/frames/` and `after/frames/` contain one plain textual frame per exact matrix key. `records.tsv` pairs every old/new digest with both frame paths. `baseline-before.tsv` proves the old capture matches the checked-in baseline. `state-comparison.tsv` provides all eight states for every affected component.

## Verdict

PASS for capture evidence: all 38 cases and 304 matrix keys are present; old frames match baseline; no unclassified state collision was found. This is evidence only, not a baseline blessing.
