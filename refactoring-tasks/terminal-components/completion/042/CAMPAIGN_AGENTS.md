# Campaign execution protocol overlay

This file is the campaign-visible executor protocol for terminal-components. It preserves canonical `/task/AGENTS.md` provenance at taskfmt revision `52d9f1eb7721f409bc47beb9fced7997b5c13ede` but **supersedes AGENTS.md steps 6–7**, the executor-authoritative ordered-check sequence, and the prohibition on claiming `DONE` without local `taskfmt verify` when those steps conflict with campaign host handoff.

Full operator/host contract: [campaign-executor-protocol.md](/work/docs/refactoring-plan/campaign-executor-protocol.md).

## Verification authority

- Local compiler/tests are **advisory only**. They must not use production expected bundles, impersonate `/proof/bin`, create accepted receipts, or authorize completion.
- **Prohibited:** `taskfmt verify --progress ""` and any progress-disabled invocation that claims completion or integration authority.
- **Required:** After implementation leaves complete, write the terminal progress event, submit candidate/progress to the operator, and await host `freeze` + `verify` with immutable per-check contexts, pinned configuration, explicit base, and **nonempty frozen progress**.
- Report the host-authentic verdict in the final `VERIFY:` line. A copied, forged, stale, or cross-run host report cannot authorize integration.

## TASK-001 bootstrap exception

TASK-001 cannot require its own future accepted host executable. For that task only, the trusted operator performs initial checkout/freeze/progress handling outside the executor, invokes pinned standalone taskfmt with explicit configuration/base/**nonempty** progress, and runs planner-frozen comparator and host bootstrap drivers. See campaign-executor-protocol.md § Initial bootstrap exception.

## Completion sequence (TASK-002+)

1. Finish scoped edits in `/work/` and append the terminal progress event to `/progress/progress.md`.
2. Place progress and a plain completion request in the operator-visible outbound directory.
3. Operator runs `tc-proof-host freeze` then `tc-proof-host verify` on the frozen tree.
4. Executor reports the returned host verdict faithfully; revise and resubmit on failure. Every revision requires a fresh full frozen gate.

## Final report

When reporting completion, the `VERIFY:` line must cite the **host-authentic** `tc-proof-host verify` result for this run, not a local `--progress ""` invocation.
