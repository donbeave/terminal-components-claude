# Execution Model and Engineering Rules

## Canonical Two-Stage Execution Model

1. **Stage A: Preparation on `termrock-refactor`**
   - Focus: snapshot, interaction, verification, and CI preparation.
   - Scope: `src/**` is strictly read-only; no refactoring of production behavior.
   - Deliverables: executable requirements registry, live/reference captures across all formats, independent admission, CI workflows, and acceptance evidence.

2. **Stage B: Termrock Refactor on `termrock-implementation`**
   - Execution: the Rust Termrock library refactor is executed exclusively on `termrock-implementation`.
   - Branch origin: `termrock-implementation` is created only from the exact accepted Stage A commit after passing its acceptance gate.
   - Scope: canonical P1–P7 implementation sequence, bounded consumer-adoption checkpoints across all four applications (`showcase`, `tablepro`, `jackin-preview`, `holla`), and final parity verification. Never continue production refactoring on `termrock-refactor`.

## Invariant Rules

- **Strict Commit Sign-off Mandate**: Commits must ALWAYS be with `Signed-off-by: Alexey Zhokhov <alexey@zhokhov.com>`. Only use `Signed-off-by: Alexey Zhokhov <alexey@zhokhov.com>` for commits (using `git commit -s`). NEVER commit anything else than `Signed-off-by: Alexey Zhokhov <alexey@zhokhov.com>`. Any commit lacking this exact sign-off or containing any other sign-off is strictly forbidden.
- **Cargo Nextest Invariant**: All Rust test and validation commands must use `cargo nextest`; never use `cargo test`.
- **Visual Baseline Tag Invariant**: Never move, delete, retarget, force-push, or recreate the `visual-baseline` git tag or its GitHub release. It is the frozen pre-refactor visual oracle (`4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`).
- **CLAUDE.md Symlink Invariant**: `CLAUDE.md` must always be a symlink to the corresponding `AGENTS.md` in the same directory. Do not write a separate `CLAUDE.md` body.
