# Subagent proof and verification contract

**Authority:** [`subagent-only-policy.md`](subagent-only-policy.md) and
[`path-contract.md`](path-contract.md).

This contract defines evidence for the refactoring tasks. It does not create a
host daemon, container workflow, receipt service, lifecycle CLI, or integration
authority. The coordinator delegates implementation, verification, and review
to subagents; the coordinator integrates only reviewed task commits.

## Fixed inputs

- Product/source oracle: peeled `visual-baseline` commit
  `4a79c0a2d40fca46fc406b77157ce3b3f12ec16b`.
- Architectural parent: `7b27732a8c3c131760ec3438f641cb3c11343a42`.
- Latest taskfmt source:
  `/Users/donbeave/Projects/taskfmt/task-format`.
- Latest taskfmt revision:
  `afd3b575dbcc7044620bec4b9493a74eca3e5ef2`.
- Latest taskfmt version: `0.2.0`.
- Frozen `visual-baseline` tag/release/store: policy-protected and never
  mutated; provider-level immutability is not assumed.

Taskfmt is used only for one task at a time:

```sh
"$TASKFMT" lint "$TASK_DIR"
"$TASKFMT" verify \
  --root "$WORKTREE" \
  --task-dir "$TASK_DIR" \
  --base "$SCOPE_BASE" \
  --progress "" \
  --log-dir "$RUN_DIR/taskfmt-logs"
```

No `taskfmt init`, `taskfmt status`, `taskfmt-host`, `taskfmt-runtime`,
`taskfmt run`, `taskfmt monitor`, `taskfmt promote`, or other lifecycle command
is permitted. Taskfmt never starts or supervises a container.

## Check workers

`verify.toml` check commands are untrusted task workers. They run on the host
inside the verifier subagent's isolated worktree and external run directory.
They may execute real production code and tests, but they must not write:

- task packages or trusted fixtures;
- the immutable visual oracle or baseline tag;
- another task's worktree or run directory;
- coordinator state, refs, or unrelated files.

Allowed proof operations are task check workers such as:

```text
tools/refactor-proof/bin/tc-proof preflight --context RUN_DIR/contexts/CHK-001.json
tools/refactor-proof/bin/tc-proof required --context RUN_DIR/contexts/CHK-002.json
tools/refactor-proof/bin/tc-proof oracle --context RUN_DIR/contexts/CHK-003.json --namespace APP
tools/refactor-proof/bin/tc-proof capture --context RUN_DIR/contexts/CHK-003.json --lane direct
tools/refactor-proof/bin/tc-proof compare --context RUN_DIR/contexts/CHK-004.json
tools/refactor-proof/bin/tc-proof account-tests --context RUN_DIR/contexts/CHK-005.json
tools/refactor-proof/bin/tc-proof architecture --context RUN_DIR/contexts/CHK-006.json
tools/refactor-proof/bin/tc-proof close --context RUN_DIR/contexts/CHK-007.json
```

These are check workers, not coordinator commands. Context files are
task-scoped, immutable for one verification run, and stored under `RUN_DIR`.
Each context binds task ID, worktree commit, scope base, operation, required
outputs, and expected provenance. Missing, extra, substituted, cross-run, or
mutated contexts fail verification.

Before invoking taskfmt, the verifier subagent must perform native proof
preparation in the isolated worktree: run
`scripts/campaign-build-proof.sh`, materialize the exact context set and
context index under the external `RUN_DIR`, and bind the per-check
preparation results, source/oracle, comparator, and observer capability. The
Rust `tc-proof` launcher then supervises each proof worker. The dispatcher
invokes only standalone taskfmt `lint`/`verify`, passes the explicit
`RUN_DIR/taskfmt-logs` path, captures the exact combined taskfmt output,
including the final `DONE`, in a verifier-owned secure temporary file outside
the run directory, then validates the real log directory and atomically
replaces `$RUN_DIR/taskfmt-logs/verify.log` with that capture. Unsafe or
non-directory log paths fail closed. It invokes `tc-proof validate --run-dir`
afterward and preserves a nonzero taskfmt status. No observer socket is
created inside the run directory or selected through a worker-controlled
default.

The preparation ABI is intentionally shared with
`scripts/campaign_ledger.py`:

The ledger and the external proof document are two different schemas. The
tracked `ledger.preparation` value is the
`campaign-preparation-qualification/v1` wrapper. Its
`proof_preparation` member uses the `preparationProof` definition and contains
only immutable path/hash references. It must not be replaced with the strong
document. The external `$RUN_DIR/proof-preparation.json` file is the
`campaign-proof-preparation/v1` document and is schema-addressable as
`docs/refactoring-plan/campaign-ledger.schema.json#proofPreparation` (the
`proofPreparation` anchor). The wrapper path/hash is checked first; the strong
document and every referenced artifact are then checked by
`validate_proof_preparation` / `validate_preparation_qualification`.

The strong document is sealed after native materialization and never contains
its own future hash. This keeps evidence non-circular: the external file binds
the run inputs, while the tracked wrapper records that file's immutable hash.

Native proof layout is verifier-owned and exact:

```text
$RUN_DIR/
  target/debug/tc-proof
  target/debug/tc-proof.build.json
  contexts/CHK-NNN.json
  results/CHK-NNN.json
  outputs/
  taskfmt-logs/
  observer.json
  context-index.json
  proof-preparation.json
```

`target` is a direct child of `$RUN_DIR`, and the run root contains exactly
these members after preparation. The target is never the candidate worktree's
`target` path or a shared-cache symlink. Every trust file and output reference
must resolve through real path components to a regular single-link file;
symlink and hard-link substitution is rejected. The JSON schema describes the
ABI shape; the native validator additionally checks bytes, hashes, source
commit/tree, oracle identity, worker identity, comparator identity, taskfmt
provenance, observer nonce/capability, trust-manifest digest, and exact run
membership.

- `context-index.json`: `tc-proof-context-index/v1`, with `task_id`,
  `run_id`, `worktree_commit`, `scope_base`, `contexts`, `results`, and
  `observer`.
- `contexts/CHK-NNN.json`: `tc-proof-context/v1`.
- `results/CHK-NNN.json`: `tc-proof-preparation-result/v1`, status `ready`.
- `observer.json`: `tc-proof-observer-capability/v1`, transport
  `inherited-pipe/v1`.

Worker results are separate host-selected artifacts under
`RUN_DIR/outputs/CHK-NNN.result.json` using
`tc-proof-runner-result/v1`; they are not substituted for preparation
results. Runtime closure is split by the bound operation:

- Native members require their exact result, any declared exact
  `CHK-NNN.compare.json` comparator report, and the native `close` result when
  present. Existing result identity, passed status, observer-event, comparator,
  and close checks remain mandatory.
- Members classified as `operation=external` are taskfmt-driven. Their bound
  context and preparation result remain mandatory, and taskfmt's exit status
  plus a final `DONE` log line remain authoritative. An external driver may
  emit no runtime artifact, or may emit its exact bound result/report paths;
  present artifacts still pass the existing schema, identity, and trust-path
  checks. Missing external artifacts are allowed. Any unbound output remains an
  extra-file failure.

A task-specific comparator retains `tc-proof-compare-context/v1` as the nested
`qualification.comparator.context`, while the immutable runner context binds
its identity and exact `CHK-NNN.compare.json` report path under the allowed
runtime output directory. This does not relax native observer binding or allow
external artifacts outside their prepared names and paths.

The observer request/response transport is one inherited-pipe protocol. Each
message binds run, task, check, nonce, request order, operation, source
commit, and source tree. An accepted response must use the exact schema,
report successful execution, and contain non-empty payload and records;
missing, replayed, truncated, empty, forged, alternate-transport, or
incomplete-close evidence fails closed. The native Rust launcher never
fabricates an observation. If an independent observer response provider is
not attached, the launch is rejected rather than accepting a worker-written
passed result. The protocol is an integrity and execution-observation control
for the native same-user threat model; it does not claim arbitrary filesystem isolation from a hostile process running as the
same user. Seatbelt or container isolation is not part of this contract.

## Production-bound capture and exact artifact closure

Candidate and oracle evidence MUST come from the real production-bound capture
path. A candidate capture MUST build and launch the application from the
candidate worktree at the bound source commit and tree, then drive its real
PTY, update, draw, input, resize, settle, and cleanup path. An oracle capture
MUST identify the corresponding production-bound capture recorded by the
protected `visual-baseline` store. Importing an oracle artifact is read-only;
it never permits regenerating, blessing, or replacing the oracle.

Detached widget fixtures, synthetic frames, static/headless renders, copied
oracle bytes, hard-coded output, and a comparator or test that fabricates a
frame are not production-bound captures. A capture whose producer, source
identity, or execution observer cannot be independently verified is rejected.

The acceptance membership is exact, not count-only. The frozen matrix contains
exactly 7,550 keys and exactly 30,200 artifacts: four artifacts for every key
(ANSI, plain text, PNG, and HTML), across every application, fixture, route,
state, interaction checkpoint, captured variant, terminal size
(72x20, 80x24, 100x30, 120x40, and 160x50), and color mode (truecolor,
256-color, 16-color, `none`, and `nocolor`). The candidate membership,
oracle membership, and comparison membership MUST each equal the read-only
oracle manifest set exactly. Missing, extra, duplicate, substituted, or
unknown keys or artifact paths fail closed; a smaller applicable subset is not
an acceptance run. The verifier MUST bind the manifest and artifact-membership
digests into the evidence rather than proving only the two counts.

## Receipt, comparator, and execution ABI

Every accepted capture and comparison MUST be represented by a strict,
versioned JSON object. The accepted schemas are
`tc-proof-capture-receipt/v1` and `tc-proof-comparator-report/v1`; their
objects use `additionalProperties: false`, reject duplicate members, and have
the exact required member sets below. A missing member, extra member, wrong
type, non-canonical encoding, or unbound hash is a failed gate.

The capture-receipt set is:

```text
schema, run_id, task_id, check_id, role, matrix_key, artifact,
source_commit, source_tree, tool_sha256, oracle_commit, oracle_tree,
oracle_sha256, producer_sha256, reviewer_sha256, evidence_sha256,
command, argv, argv_sha256, observer, exit
```

The comparator-report set is:

```text
schema, run_id, task_id, check_id, matrix_key, artifact,
source_commit, source_tree, tool_sha256, oracle_commit, oracle_tree,
oracle_sha256, producer_sha256, reviewer_sha256, evidence_sha256,
command, argv, argv_sha256, observer, comparison, exit
```

`source_commit` and `source_tree` identify the captured production source;
`oracle_commit`, `oracle_tree`, and `oracle_sha256` identify the immutable
expected oracle artifact; `tool_sha256` identifies the exact comparator or
capture tool; `producer_sha256` identifies the executable or script that
produced the bytes; `reviewer_sha256` identifies the independent review
receipt; and `evidence_sha256` identifies the exact evidence bytes. Hashes
MUST be lowercase hexadecimal SHA-256 values except that Git commit/tree
identities use their full object IDs. The reviewer hash MUST not be produced
by the capture producer or comparator itself.

`command` is the bound logical operation. `argv` is the exact ordered argument
vector, including the executable, with no shell interpolation or omitted
environment-selected argument. `argv_sha256` is the SHA-256 of the canonical
JSON encoding of that vector. The recorded executable bytes MUST match
`tool_sha256`; a command string, path, or exit status alone cannot substitute
for the executable and argv binding.

`observer` is also strict and contains exactly `transport`, `nonce_sha256`,
`request_sha256`, and `response_sha256`. It MUST use
`inherited-pipe/v1`. Observer request and response records MUST bind the same
run, task, check, operation, source commit, source tree, nonce, request order,
and numerical exit as the receipt. Missing, replayed, forged, empty, or
cross-run observer evidence fails closed.

All proof documents, receipts, comparator reports, and observer records use
one canonical JSON ABI: UTF-8 bytes, one JSON value, lexicographically sorted
object keys, no insignificant whitespace, no BOM, no trailing newline, no
duplicate object names, and only finite JSON numbers. Hashes cover those exact
serialized bytes. A schema's member set is part of its ABI; changing field
order, spelling, type, or canonicalization is a schema change and requires a
new version.

Every `operation=architecture` context and its accepted evidence MUST contain
a non-empty `architecture_profile` using
`tc-proof-architecture-profile/v1`. The profile MUST be the machine-readable
profile generated from the bound candidate source and MUST carry its source
commit/tree, producer, reviewer, evidence hash, and numerical exit. Omitted,
`null`, empty, prose-only, stale, or candidate-unbound architecture profiles
fail closed. Architecture proof cannot be accepted from a generic result or
from a profile supplied only by a worker-controlled extension.

`exit` is required in every capture, comparator, and observer record and MUST
be a JSON integer, not a Boolean or string, equal to the actual process exit
value. `exit == 0` is the only successful execution; every nonzero exit MUST
remain nonzero through the native launcher, taskfmt driver, receipt, and
review evidence. A textual status, a reviewer verdict, or a successful parent
process cannot turn a failed child exit into a pass.

## Evidence ownership

The implementer subagent produces a scoped commit and advisory test output.
The verifier subagent produces raw `taskfmt lint`/`verify` output plus check
logs. The reviewer subagent independently checks:

1. task scope and writable paths;
2. exact worktree commit and scope base;
3. taskfmt executable revision/version/SHA;
4. check exit codes, final `DONE`, and required outputs;
5. no oracle, trust-root, ref, or unrelated-file mutation.

The reviewer returns `VERIFIED`, `REJECTED`, or `BLOCKED`. A passing taskfmt
result alone never authorizes integration or task completion.

## Visual and behavioral evidence

Visual comparisons must read the frozen oracle; candidates never write or bless
it. Candidate capture output is untrusted and lives only in `RUN_DIR` until
the reviewer accepts the task evidence. Behavioral evidence must cover real
production update/draw/event paths, not detached fixtures or copied oracle
frames. Any missing oracle asset is a blocker, not a reason to weaken a gate.

## Integration

After verifier and reviewer approval, the coordinator integrates the exact
subagent commit against the expected parent with ordinary Git compare-and-swap
checks. Integration is not performed by taskfmt or a proof worker. Use the
repository commit policy: `git commit -s` and
`Co-authored-by: Codex <codex@openai.com>`.
