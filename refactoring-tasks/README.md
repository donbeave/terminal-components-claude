# Termrock refactoring task catalog

This catalog is the executable future implementation plan for the in-place Termrock refactor.
The work starts from the committed descendant of the frozen `visual-baseline` oracle on
`termrock-refactor`; each task package receives its actual starting tree from the caller. No task
creates a clean-room repository or a second project.

The four existing applications remain parity consumers. They are not product work. Every package
protects `snapshots/**` and `tests/visual_baseline/**`; all packages except the single P6 consumer
migration package also protect `src/bin/**`. TASK-015 alone may edit its exact application
presentation paths and the one TablePro `preview_sql` adapter path listed in its verifier. Its
acceptance contract requires exact frozen output and interaction parity; the SQL adapter must match
the baseline's sealed statement vectors.

## Format and execution

Packages use [task-format revision `6348230a7197ddbe3db164c53c3e8ee1ba256f06`](https://github.com/donbeave/task-format/blob/6348230a7197ddbe3db164c53c3e8ee1ba256f06/reference/FORMAT.md)
and `taskfmt 0.2.0`, with `task/v5` README contracts and `verify/v2` verifier contracts. The current format has no `task.toml`,
`task-meta/v1`, project command, or predecessor field. Lint all packages with:

```sh
for task in refactoring-tasks/P*/TASK-*; do
  taskfmt lint "$task" || exit
done
```

The verifier commands are future implementation checks. They use `rtk cargo nextest`; no Rust
implementation or test run is part of this documentation-only change. Each package has one final
gate check. The final gate intentionally runs the workspace suite because shared runtime and
rendering changes cross component boundaries. Only `TASK-015` may update application call sites to
consume Termrock components, plus the TablePro `preview_sql` input adapter required to consume
caller-owned Grid state; it cannot change fixtures, scenarios, product behavior, or output. The
adapter boundary and parity rule are owned by
[`migration.md`](../docs/implementation/migration.md).

## Dependency DAG

Dependencies are catalog relationships, not fabricated Git SHAs. A task can start only from the
actual committed descendant accepted for its dependencies.

| Phase | Package | Outcome | Depends on |
| --- | --- | --- | --- |
| P0 | [TASK-001](P0/TASK-001/README.md) | Qualify the oracle, conformance package, pinned toolchain, and independent comparator | — |
| P1 | [TASK-002](P1/TASK-002/README.md) | Identity, events, responses, and constrained public API | TASK-001 |
| P1 | [TASK-003](P1/TASK-003/README.md) | Runtime focus, hit testing, capture, time, and layers | TASK-001, TASK-002 |
| P1 | [TASK-004](P1/TASK-004/README.md) | Measurement, layout, theme, text, and authoring foundations | TASK-002, TASK-003 |
| P2 | [TASK-005](P2/TASK-005/README.md) | Basic controls and semantic chrome | TASK-004 |
| P2 | [TASK-006](P2/TASK-006/README.md) | Keyed collections and reconciliation | TASK-004 |
| P3 | [TASK-007](P3/TASK-007/README.md) | Shared text editing core, fields, and forms | TASK-004, TASK-005 |
| P3 | [TASK-008](P3/TASK-008/README.md) | Choices, controlled values, secrets, and validation | TASK-006, TASK-007 |
| P4 | [TASK-009](P4/TASK-009/README.md) | Shared overlay and menu engine | TASK-003, TASK-005 |
| P4 | [TASK-010](P4/TASK-010/README.md) | Picker, command palette, chains, and completion | TASK-006, TASK-009 |
| P5 | [TASK-011](P5/TASK-011/README.md) | Panel, split, scroll, and text viewport infrastructure | TASK-004, TASK-006 |
| P5 | [TASK-012](P5/TASK-012/README.md) | Grid, code editing, and diff presentation | TASK-007, TASK-011 |
| P5 | [TASK-013](P5/TASK-013/README.md) | Progress, spinner, meter, status, hints, and help | TASK-005, TASK-006, TASK-011 |
| P6 | [TASK-014](P6/TASK-014/README.md) | Prepared-cell TerminalView and terminal edge contracts | TASK-011, TASK-012, TASK-013 |
| P6 | [TASK-015](P6/TASK-015/README.md) | Narrow consumer migration and composed conformance | TASK-008, TASK-010, TASK-014 |
| P7 | [TASK-016](P7/TASK-016/README.md) | Public API and architecture closure audit | TASK-015 |
| P7 | [TASK-017](P7/TASK-017/README.md) | Complete visual/interaction parity and negative gates | TASK-016 |
| P7 | [TASK-018](P7/TASK-018/README.md) | Performance evidence and independent release review | TASK-017 |

## Canonical contract ownership

Task packages identify the work boundary and proof. They do not redefine component behavior. Read
the linked canonical contracts before implementation:

- `docs/architecture/overview.md` owns the target architecture and ownership boundaries.
- `docs/api/public-api.md` owns `update`, `draw`, `measure`, props, state, actions, and IDs.
- `docs/foundations/` owns shared identity, events, runtime, layout, theme, text, collections,
  secrets, authoring, and conformance mechanisms.
- `docs/components/` owns the complete public component/API inventory and component behavior.
- `docs/design/` owns visual and interaction parity.
- `docs/verification/` owns oracle provenance, exact comparison, state coverage, and mutation gates.
- `docs/implementation/plan.md` owns phase sequencing and in-place migration strategy.
- `docs/implementation/quality-gates.md` owns the pinned toolchain, package/dependency boundaries, implementation-quality blockers, performance evidence, and independent-review protocol.

Application migration is limited to eventual adoption of these reusable mechanisms while preserving
the exact frozen Showcase, TablePro, Jackin Preview, and Holla behavior. TASK-015 is the sole
migration package; its exact application presentation files and TablePro preview adapter are the
only application source scope. No package authorizes services, providers, Docker, account
authorization, Git operations, database backends, shell/PTY runtime, or product redesign.
