# Production task container paths (TASK-002+)

**Scope:** Tasks **002–069** and **073** (and hybrid **071–072** for bootstrap checks). Complements [`task-001-verify-container.md`](task-001-verify-container.md) and [`path-contract.md`](path-contract.md).

---

## Why these paths exist

Production `verify.toml` files invoke the accepted harness worker:

```text
/proof/bin/tc-proof <operation> --context /run/tc-proof/contexts/CHK-NNN.json
```

The host (`tc-proof-host`) must install `/proof/bin/tc-proof` after TASK-001 receipt and materialize frozen contexts under `/run/tc-proof/` before `taskfmt verify` runs.

**taskfmt does not create these paths.** `prepare` and `freeze` do.

---

## Required container layout (TASK-002+)

| Container path | Host source | Required for |
| --- | --- | --- |
| `/task/` | Catalog package `completion/NNN` | taskfmt lint, package checks |
| `/work/` | Frozen candidate checkout | scope, subprocess CWD |
| `/progress/progress.md` | Host-frozen progress | progress gate |
| `/proof/bin/tc-proof` | Accepted harness install | CHK-001–007 (tc-proof operations) |
| `/run/tc-proof/context-index.json` | Host `prepare`/`freeze` | context binding |
| `/run/tc-proof/contexts/CHK-NNN.json` | One per check in verify.toml | each tc-proof invocation |

Bootstrap tasks **001** and **070** do not use `/proof/bin/` or `/run/tc-proof/` in verify.toml. Tasks **071** and **072** use both bootstrap and production paths (hybrid).

---

## Host sequence (not taskfmt run/promote)

```sh
tc-proof-host prepare --campaign "$CAMPAIGN" \
  --task terminal-components/completion/NNN \
  --parent "$INTEGRATION_PARENT" \
  --run "$RUN"

# executor completes /work edits and progress …

tc-proof-host freeze --run "$RUN" --candidate "$WORKTREE"
tc-proof-host verify --run "$RUN"
```

Inside `verify`, the host runs pinned `taskfmt verify` with container paths visible. Individual checks spawn `/proof/bin/tc-proof` per frozen context.

---

## Common mistakes

| Mistake | Result |
| --- | --- |
| Running `taskfmt verify` with only `--task-dir` and `--root` | CHK fail: `/proof/bin/tc-proof` not found |
| Using `/work/tools/refactor-proof/bin/tc-proof` in production tasks | Wrong — production uses `/proof/bin/tc-proof` after install receipt |
| Expecting `target/debug/tc-proof` to satisfy production verify.toml | Wrong — only bootstrap tasks use `/work/tools/.../bin/` |
| Confusing `$TC_RUN` with `/run/tc-proof/` | Host maps run dir → container `/run/tc-proof/` at verify time |

---

## Hybrid advisory verify (TASK-071/072)

Hybrid packages keep bootstrap drivers for CHK-004+ but add production CHK-001:

```text
/proof/bin/tc-proof preflight --context /run/tc-proof/contexts/CHK-001.json
```

Agents cannot run `sudo` or `apfs.util`. Use [`scripts/hybrid-verify-sandbox.sh`](../../scripts/hybrid-verify-sandbox.sh):

```sh
export TC_PLANNING_REPO=/path/to/terminal-components-claude
export TC_WORKTREE=$TC_PLANNING_REPO/.worktrees/campaign

# No sudo — stages bind tree + CHK-001.json + synthetic.conf.fragment (includes run)
./scripts/hybrid-verify-sandbox.sh prepare-hybrid --task 071   # or 072

# Operator once (outside agent sessions) — merges fragment and refreshes firmlinks:
sudo sh -c 'grep -q tc-task-001-bind /etc/synthetic.conf 2>/dev/null || cat /private/tmp/tc-task-001-bind/synthetic.conf.fragment >> /etc/synthetic.conf; /System/Library/Filesystems/apfs.fs/Contents/Resources/apfs.util -t'

# Agent — verify-only mount check
./scripts/hybrid-verify-sandbox.sh mount
./scripts/hybrid-verify-sandbox.sh layout-smoke

# Advisory taskfmt (exports TC_PROOF_* for CHK-001 preflight only)
export TC_RUN=/tmp/tc-hybrid-071-run
mkdir -p "$TC_RUN/logs"
./scripts/hybrid-verify-sandbox.sh verify --progress "$TC_RUN/progress.md" --log-dir "$TC_RUN/logs"
```

### Without operator `/run` firmlink

| Approach | `/run/tc-proof` at verify time | Full hybrid verify |
| --- | --- | --- |
| **Operator `apfs.util -t`** | Yes — `/run/tc-proof/contexts/CHK-001.json` | Yes on macOS host |
| **Docker `docker-smoke`** | Yes inside container only | No — no `sandbox-exec`, not taskfmt |
| **`tc-proof-host prepare/freeze`** | Host `$RUN/contexts/` mapped by operator | Yes — campaign authority; requires `campaign.json` task entry for 071/072 |

`docker-smoke` (no sudo):

```sh
./scripts/hybrid-verify-sandbox.sh docker-smoke
```

`tc-proof-host prepare` for TASK-071/072 needs `.campaign/host/campaign.json` extended with `check_context_templates` for the task (not present in pre-arm campaign). Production host verify remains operator-owned; advisory sandbox is for implementer path/layout checks only.

---

## Validation

- Planning: `validate-plan.py` `catalog_argv_smoke()` — 479 `/proof/bin/tc-proof` argv references across packages (does not invoke binary).
- Runtime: host must prove contexts exist and match `verify.toml` check IDs before verify.
