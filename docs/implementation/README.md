# Termrock implementation plan

This directory owns the future execution sequence for the in-place Termrock
refactor. It does not define a second component API, a second visual contract,
or product roadmaps for the four reference applications.

## Read in this order

1. [`plan.md`](plan.md) — the P0–P7 implementation DAG, phase gates, and work
   packet rules.
2. [`migration.md`](migration.md) — how the current `junie-tui` implementation
   becomes the Termrock library while preserving the frozen applications.
3. [`../architecture/overview.md`](../architecture/overview.md) — the target
   architecture and ownership boundaries.
4. [`../api/public-api.md`](../api/public-api.md) — the target caller-facing
   API.
5. [`../foundations/README.md`](../foundations/README.md) and
   [`../components/README.md`](../components/README.md) — the shared contracts
   and complete component inventory.
6. [`../verification/README.md`](../verification/README.md) — oracle,
   conformance, and parity gates.

The task catalog under `refactoring-tasks/` turns this plan into executable
work packages. Tasks link to these documents and to the canonical foundation,
API, component, and verification contracts; they must not copy those
contracts into task prose.

## Destination and current truth

The implementation destination is this repository on the `termrock-refactor`
branch, created from the annotated `visual-baseline` tag at
`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`. The branch is the continuation of
the existing implementation, not a clean-room project.

The current package and source names remain true until a future implementation
step changes them:

- Cargo package: `junie-tui`;
- Rust library name: `junie_tui`;
- current internals may contain `WidgetId`, `Outcome`, `RenderCtx`, and other
  legacy names.

Those names are current implementation facts, not the target public identity.
The future library identity is Termrock. A package, crate, or module rename is
an implementation-phase change with its own compile and consumer gates. This
documentation phase does not perform that rename.

The earlier source pack described a separate `termrock-new` destination. That
assumption is historical and superseded. No phase in this plan creates,
copies, or migrates into another repository.

## Scope

Every phase is limited to the reusable library, its generic conformance
consumer, and the proof needed to preserve the baseline. The four binaries
remain preserved consumers:

- `showcase` — component laboratory;
- `tablepro` — complex grid/editor/workbench composition;
- `jackin-preview` — host-management and terminal-pane composition;
- `holla` — finder, preview, action, plan, output, and context composition.

Application code may be adapted in a later implementation phase only when
needed to consume a refactored reusable component. Such a change must preserve
the frozen output, observable keyboard/pointer behavior, scenarios, and
snapshots. No application product feature, service, backend, route, or visual
redesign belongs in this plan.

## Canonical ownership

| Question | Canonical owner |
|---|---|
| Mission, immutable boundaries, and definition of success | [`GOAL.md`](../../GOAL.md) |
| Target architecture and concern ownership | [`../architecture/`](../architecture/) |
| Public signatures, types, authoring, and parts | [`../api/`](../api/) |
| Shared runtime/foundation contracts | [`../foundations/`](../foundations/) |
| Component behavior and state matrices | [`../components/`](../components/) |
| Visual and interaction contracts | [`../design/`](../design/) |
| Oracle provenance and exact parity gates | [`../verification/`](../verification/) |
| Preserved application roles and scenarios | [`../applications/`](../applications/) |
| Executable dependency graph and task metadata | [`../../refactoring-tasks/`](../../refactoring-tasks/) |

This directory records sequencing, dependencies, migration mechanics, and
phase acceptance. If a phase needs a behavior rule, it links to the owner
above instead of defining a competing rule.

Machine-readable coverage remains beside the canonical contracts under
`docs/reference/`: component inventory, foundation inventory, legacy-family
disposition, source/tool locks, and capture-plan metadata. These manifests
describe requirements and provenance; they do not contain approved candidate
output.

## Stop line

The plan is complete when the same repository exposes the Termrock public
library contract, every applicable baseline case is compared against the
trusted oracle, generic consumers compile using public exports, and the four
applications still match the frozen visual and interaction reference. Product
integration remains a separately approved effort after this library plan.
