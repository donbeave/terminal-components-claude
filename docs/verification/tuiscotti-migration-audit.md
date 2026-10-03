# Tuiscotti Baseline Migration & Cutover Audit

## 1. Executive Summary

This audit records the complete migration of the legacy snapshot store (`snapshots/`) to the screen-first Tuiscotti baseline (`baselines/tuiscotti-v1/`) on branch `termrock-refactor`.

- **Total Admitted Captures**: 7550 (302 screens across 4 applications × 5 sizes × 5 colors).
- **Total Admitted Artifacts**: 75,500 (6 primary formats + 4 companion artifacts per capture).
- **Text Parity**: 7550/7550 (100.00% exact character match).
- **ANSI Parity**: 7550/7550 (100.00% byte-exact normalized SGR matches).
- **PNG Mean Similarity Score**: 0.9998 (min: 0.9990, max: 1.0000).
  - Exact 1.000: 0
  - 0.990 – 0.999: 7550
  - 0.950 – 0.989: 0
  - Below 0.950: 0
- **Conformance Status**: 222 component cases reconciled to `planned`; 302 application oracle cases reconciled to `approved`.

## 2. Provenance and Immutable Reference Verification

| Reference Axis | Value | Contract Verification |
|---|---|---|
| Working Branch | `termrock-refactor` | Preserved start and final commits; zero production (`src/**`) changes |
| `REFERENCE_APP_SHA` | `7bd6a331721737514a2477c894d922cb262ef07b` | Unchanged application oracle |
| `visual-baseline` Tag | `1ee5ebdcb91fd87adb9a5b28e43d4c7f421706c5` | Immutable visual tag |
| `visual-baseline` Commit | `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b` | Peeled commit matches legacy tree |
| Tuiscotti Source SHA | `a47c9aaefb34e4c00026f99d8a8dd7ee5916b274` | Locked qualified release |
| Velnor Actions | `velnor-actions 0.1.0` (`c57c700459bbe1549fe7eedcb7d8689585c38986`) | Validated release generator |

## 3. Font Rasterizer & Differential Analysis

The legacy `snapshots/` PNG artifacts were rasterized by `tui-snap` with platform-dependent system fonts. Tuiscotti replaces this with vendored, deterministic font faces (`DejaVuSansMono` and `DejaVuSansMono-Bold` with CJK fallback), guaranteeing byte-level cross-platform determinism across macOS and Linux CI.

Because glyph shapes and antialiasing differ slightly between font renderers, PNG comparison between legacy snapshots and Tuiscotti captures yields a characteristic similarity distribution centered at 0.9998, with 100% cell and text agreement.

## 4. Screen-First Hierarchy Inventory

The 302 legacy roots have been mapped deterministically into screen-first canonical paths:
`baselines/tuiscotti-v1/<application>/<screen>/<substep>/<cols>x<rows>/<color>.<ext>`

| Application | Canonical Screens | Captures | Total Files |
|---|---|---|---|
| `showcase` | buttons, chips, chrome, datagrid, diff, editable-tables, editor, forms, inputs, lists, overview, panels, pickers, progress, scrolling, settings, sidebars, tables, taskrunner, terminal, textareas, trees | 3,375 | 33,750 |
| `jackin` | accounts, capsule, cockpit, editor, inspect, manager, modals, prelude, settings, usage | 1,475 | 14,750 |
| `holla` | activities, browser, cleanup, disk, docker, files, finder, plan, review, snapshot | 1,825 | 18,250 |
| `tablepro` | connections, history, query, safety, structure, table, workbench | 875 | 8,750 |
| **Total** | **302 roots** | **7,550** | **75,500** |

## 5. Artifact Set Specification

Each admitted scenario contains exactly 10 artifacts:
1. `<name>.frame.json`: Canonical semantic terminal buffer with cell glyphs, colors, modifiers, cursor, and provenance.
2. `<name>.ansi`: Normalized SGR terminal stream.
3. `<name>.txt`: Plain Unicode text representation.
4. `<name>.png`: Opaque RGB pixel rendering with vendored fonts.
5. `<name>.html`: Static offline HTML embedding base64-encoded PNG.
6. `<name>.ascii`: 7-bit diagnostic projection.
7. `<name>.ascii.loss.json`: Loss accounting for all substituted non-ASCII characters.
8. `<name>.png.fidelity.json`: Missing-glyph and fallback accounting.
9. `<name>.observations.json`: Terminal cursor, non-empty cells, and layout metadata.
10. `<name>.manifest.json`: Atomic candidate seal containing SHA-256 digests for all 9 data and companion artifacts.
