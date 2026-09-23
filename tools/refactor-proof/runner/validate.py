"""Observation and extension payload validation."""

from __future__ import annotations

import os
import stat
from pathlib import Path
from typing import Any

from .context import ALLOWED_V1_KEYS, Reject, allowed_context_keys, expanded_members
from .json_util import load_path, sha256_bytes, sha256_canonical


def seeded_value(config: dict[str, Any]) -> int:
    return config["seed"] + len(config.get("seed_steps", []))


def validate_capture_payload(
    event: dict[str, Any],
    *,
    lane: str,
    expected_members: list[str],
    config: dict[str, Any],
    operation: str,
) -> None:
    if operation not in {"oracle", "capture", "direct", "pty"}:
        raise Reject("EXECUTION")
    if "exit" not in event or type(event.get("exit")) is not int or event.get("exit") != 0:
        raise Reject("EXECUTION")
    if not isinstance(event.get("payload"), dict):
        raise Reject("EXECUTION")
    body = event["payload"]
    calls = body.get("calls", [])
    if not isinstance(calls, list) or any(not isinstance(call, str) for call in calls):
        raise Reject("EXECUTION")
    draw_count = 10
    if "App.update" not in calls:
        raise Reject("EXECUTION")
    if calls.count("Widget.draw") != draw_count:
        raise Reject("EXECUTION")
    if "Props.enabled" not in calls:
        raise Reject("EXECUTION")
    seeded = seeded_value(config)
    expected_value = seeded + (0 if operation == "architecture" else 1)
    if "value" not in body or body["value"] != expected_value:
        raise Reject("EXECUTION")
    expected_pty = operation == "capture" and lane == "pty"
    if body.get("pty") != expected_pty:
        raise Reject("EXECUTION")
    if operation in {"capture", "direct", "pty"}:
        selected_lane = lane if operation == "capture" else "direct"
        members = [name for name in expected_members if name.split("/")[1] == selected_lane]
        captures = body.get("captures")
        if not isinstance(captures, list) or [row["id"] for row in captures] != members:
            raise Reject("EXECUTION")
        for row in captures:
            if set(row) != {"id", "before", "after"}:
                raise Reject("EXECUTION")
            _, _, width, palette = row["id"].split("/")
            for checkpoint, increment in (("before", 0), ("after", 1)):
                expected = {
                    "width": int(width),
                    "custom_art": "*",
                    "control": {
                        "text": str(seeded + increment),
                        "foreground": palette,
                        "owner": "Widget",
                    },
                }
                if row[checkpoint] != expected:
                    raise Reject("EXECUTION")


def validate_oracle_repeat(
    events: list[dict[str, Any]],
    *,
    context: dict[str, Any] | None = None,
    source_commit: str | None = None,
    source_tree: str | None = None,
) -> None:
    if len(events) != 2:
        raise Reject("EXECUTION")
    first = events[0].get("payload")
    second = events[1].get("payload")
    if (
        type(events[0].get("exit")) is not int
        or type(events[1].get("exit")) is not int
        or events[0].get("exit") != 0
        or events[1].get("exit") != 0
    ):
        raise Reject("EXECUTION")
    if not isinstance(first, dict) or not first or not isinstance(second, dict) or not second:
        raise Reject("EXECUTION")
    if sha256_canonical(first) != sha256_canonical(second):
        raise Reject("REPEAT")
    if context is None:
        return
    if set(events[0]) != _OBSERVATION_KEYS or set(events[1]) != _OBSERVATION_KEYS:
        raise Reject("REPEAT")
    if source_commit is None:
        source_commit = context.get("oracle_commit")
    if source_tree is None:
        source_tree = context.get("tree")
    expected = {
        "run_id": context.get("run_id"),
        "task_id": context.get("task_id"),
        "check_id": context.get("check_id"),
        "operation": "oracle",
        "source_commit": source_commit,
        "tree": source_tree,
    }
    if not isinstance(source_commit, str) or not isinstance(source_tree, str):
        raise Reject("REPEAT")
    nonce = events[0].get("nonce")
    if not isinstance(nonce, str) or not nonce:
        raise Reject("REPEAT")
    for request_id, event in enumerate(events):
        if any(event.get(key) != value for key, value in expected.items()):
            raise Reject("REPEAT")
        if (
            event.get("nonce") != nonce
            or type(event.get("request_id")) is not int
            or event.get("request_id") != request_id
        ):
            raise Reject("REPEAT")
        if not isinstance(event.get("records"), list) or not event["records"]:
            raise Reject("REPEAT")


def validate_native_extension(payload: dict[str, Any], contract: dict[str, Any], source_commit: str) -> None:
    if payload.get("source") != contract["source_sha256"]:
        raise Reject("SOURCE")
    if contract.get("mapping_owner") != source_commit:
        raise Reject("SOURCE")
    if contract.get("mapping") != {"footer_row": "height - 2", "pointer": [2, "height - 2"]}:
        raise Reject("SOURCE")
    runs = payload.get("runs")
    if not runs or runs[0].get("exit") != 0:
        raise Reject("SOURCE")
    observed = runs[0]["stdout"]
    native = observed["native"]
    if [(row["width"], row["height"], row["row"]) for row in native] != [(120, 40, 38), (100, 30, 28)]:
        raise Reject("SOURCE")
    resized = observed["resized"]
    if [[row["width"], row["height"]] for row in resized] != contract["resized_sizes"]:
        raise Reject("SOURCE")
    for row in native + resized:
        if row["row"] != row["height"] - 2:
            raise Reject("SOURCE")
        if row["pointer"] != [2, row["height"] - 2]:
            raise Reject("SOURCE")
        if len(row["rows"]) != row["height"]:
            raise Reject("SOURCE")
        if any(len(line) != row["width"] for line in row["rows"]):
            raise Reject("SOURCE")
        if row.get("expected", "attached") not in row["rows"][row["row"]]:
            raise Reject("SOURCE")


_INDEX_KEYS = {
    "schema",
    "task_id",
    "run_id",
    "worktree_commit",
    "scope_base",
    "contexts",
    "results",
    "observer_sequences",
    "observer",
}
_MEMBER_KEYS = {"check_id", "path", "sha256"}
_PREPARATION_KEYS = {
    "schema",
    "task_id",
    "check_id",
    "run_id",
    "worktree_commit",
    "scope_base",
    "context_sha256",
    "status",
}
_RUNTIME_RESULT_KEYS = {
    "schema",
    "run_id",
    "operation",
    "context_sha256",
    "status",
    "category",
    "observation_digests",
    "outputs",
}
_COMPARISON_KEYS = {
    "schema",
    "run_id",
    "task_id",
    "context_sha256",
    "required_count",
    "checked_count",
    "passed_count",
    "results",
    "failures",
}
_COMPARISON_FAILURE_CODES = {
    "UNSAFE_PATH",
    "UNEXPECTED_APPROVAL",
    "INTEGRITY",
    "REQUIRED_SET",
    "PROVENANCE",
    "FRAME_INVALID",
    "FRAME_MISMATCH",
    "STATE_INVALID",
    "STATE_MISMATCH",
}

_OBSERVATION_KEYS = {
    "schema",
    "nonce",
    "run_id",
    "task_id",
    "check_id",
    "request_id",
    "operation",
    "source_commit",
    "tree",
    "exit",
    "stdout",
    "stderr",
    "files",
    "payload",
    "records",
}

_ALLOWED_SYSTEM_SYMLINKS = {
    Path("/etc"): Path("/private/etc"),
    Path("/home"): Path("/System/Volumes/Data/home"),
    Path("/tmp"): Path("/private/tmp"),
    Path("/var"): Path("/private/var"),
}


def validate_comparison_report(
    report: Any,
    report_raw: bytes,
    context: dict[str, Any],
    context_hash: str,
    report_path: Path,
    *,
    require_success: bool,
    category: str,
) -> str:
    """Validate a native comparator report against its bound runner context."""

    def reject() -> None:
        raise Reject(category)

    if not isinstance(report, dict) or set(report) != _COMPARISON_KEYS:
        reject()
    if not isinstance(report_raw, bytes):
        reject()
    qualification = context.get("qualification")
    comparator = qualification.get("comparator") if isinstance(qualification, dict) else None
    nested = comparator.get("context") if isinstance(comparator, dict) else None
    if (
        context.get("operation") != "compare"
        or not isinstance(comparator, dict)
        or not isinstance(nested, dict)
        or nested.get("schema") != "tc-proof-compare-context/v1"
        or nested.get("run_id") != context.get("run_id")
        or nested.get("task_id") != context.get("task_id")
        or nested.get("check_id") != context.get("check_id")
        or not isinstance(context.get("run_id"), str)
        or not Path(context["run_id"]).is_absolute()
        or not isinstance(context.get("check_id"), str)
        or not _check_id(context["check_id"])
    ):
        reject()
    expected_report = Path(context["run_id"]) / "outputs" / f"{context['check_id']}.compare.json"
    if report_path != expected_report or comparator.get("report_path") != str(expected_report):
        reject()
    if nested.get("report_path") != str(expected_report):
        reject()

    required_ids = nested.get("required_ids")
    required_count = nested.get("required_count")
    if (
        not isinstance(required_ids, list)
        or not required_ids
        or any(not isinstance(value, str) or not value for value in required_ids)
        or len(set(required_ids)) != len(required_ids)
        or type(required_count) is not int
        or required_count != len(required_ids)
    ):
        reject()
    if (
        report.get("schema") != "tc-proof-comparison/v1"
        or report.get("run_id") != context.get("run_id")
        or report.get("task_id") != context.get("task_id")
        or report.get("context_sha256") != context_hash
        or type(report.get("required_count")) is not int
        or report.get("required_count") != required_count
    ):
        reject()
    counts = [report.get(name) for name in ("checked_count", "passed_count")]
    if any(type(value) is not int or value < 0 or value > required_count for value in counts):
        reject()
    checked_count, passed_count = counts
    if passed_count > checked_count:
        reject()

    results = report.get("results")
    failures = report.get("failures")
    if not isinstance(results, list) or not isinstance(failures, list):
        reject()
    failed_ids: set[str] = set()
    if results:
        if checked_count != required_count or [row.get("id") if isinstance(row, dict) else None for row in results] != required_ids:
            reject()
        for result in results:
            if (
                not isinstance(result, dict)
                or set(result) != {"id", "status"}
                or not isinstance(result.get("id"), str)
                or result.get("status") not in {"passed", "failed"}
            ):
                reject()
            if result["status"] == "failed":
                failed_ids.add(result["id"])
        if passed_count != required_count - len(failed_ids):
            reject()
    elif checked_count != 0 or passed_count != 0:
        reject()

    failure_ids: set[str] = set()
    for failure in failures:
        if not isinstance(failure, dict) or set(failure) not in ({"id", "code"}, {"id", "code", "detail"}):
            reject()
        failure_id = failure.get("id")
        if failure_id is not None and (
            not isinstance(failure_id, str) or failure_id not in required_ids or failure_id in failure_ids
        ):
            reject()
        if failure.get("code") not in _COMPARISON_FAILURE_CODES:
            reject()
        if "detail" in failure and not isinstance(failure["detail"], str):
            reject()
        if failure_id is not None:
            failure_ids.add(failure_id)
    if results:
        if failure_ids != failed_ids:
            reject()
    elif len(failures) != 1 or failures[0].get("id") is not None:
        reject()

    if require_success:
        if (
            not results
            or checked_count != required_count
            or passed_count != required_count
            or failures
            or any(result.get("status") != "passed" for result in results)
        ):
            reject()
    elif results and not failed_ids:
        reject()
    return sha256_bytes(report_raw)


def _hex(value: Any, length: int) -> bool:
    return isinstance(value, str) and len(value) == length and all(
        char in "0123456789abcdef" for char in value
    )


def _check_id(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 7 and value.startswith("CHK-") and value[4:].isdigit()


def _real_path_components(path: Path) -> None:
    if not path.is_absolute():
        raise Reject("CLOSURE")
    current = Path(path.anchor)
    for component in path.parts[1:]:
        current /= component
        try:
            metadata = current.lstat()
        except OSError:
            raise Reject("CLOSURE") from None
        if stat.S_ISLNK(metadata.st_mode):
            allowed_target = _ALLOWED_SYSTEM_SYMLINKS.get(current)
            if allowed_target is None or current.resolve() != allowed_target:
                raise Reject("CLOSURE")


def _regular_file(path: Path) -> None:
    _real_path_components(path)
    try:
        metadata = path.lstat()
    except OSError:
        raise Reject("CLOSURE") from None
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise Reject("CLOSURE")


def _real_directory(path: Path) -> None:
    _real_path_components(path)
    try:
        metadata = path.lstat()
    except OSError:
        raise Reject("CLOSURE") from None
    if path.is_symlink() or not stat.S_ISDIR(metadata.st_mode):
        raise Reject("CLOSURE")


def _object(path: Path) -> dict[str, Any]:
    try:
        value = load_path(path)
    except (OSError, TypeError, UnicodeDecodeError, ValueError):
        raise Reject("CLOSURE") from None
    if not isinstance(value, dict):
        raise Reject("CLOSURE")
    return value


def _bound_member(member: Any, expected_id: str, directory: Path) -> tuple[Path, str]:
    if not isinstance(member, dict) or set(member) != _MEMBER_KEYS:
        raise Reject("CLOSURE")
    if member.get("check_id") != expected_id or not _hex(member.get("sha256"), 64):
        raise Reject("CLOSURE")
    raw_path = member.get("path")
    if not isinstance(raw_path, str):
        raise Reject("CLOSURE")
    path = Path(raw_path)
    if not path.is_absolute() or path.parent != directory or path.name != f"{expected_id}.json":
        raise Reject("CLOSURE")
    _regular_file(path)
    try:
        raw = path.read_bytes()
    except OSError:
        raise Reject("CLOSURE") from None
    if sha256_bytes(raw) != member["sha256"]:
        raise Reject("CLOSURE")
    return path, member["sha256"]


def _bound_path_reference(value: Any) -> tuple[Path, str]:
    if not isinstance(value, dict) or set(value) != {"path", "sha256"}:
        raise Reject("CLOSURE")
    raw_path = value.get("path")
    if not isinstance(raw_path, str) or not _hex(value.get("sha256"), 64):
        raise Reject("CLOSURE")
    path = Path(raw_path)
    _regular_file(path)
    try:
        raw = path.read_bytes()
    except OSError:
        raise Reject("CLOSURE") from None
    if sha256_bytes(raw) != value["sha256"]:
        raise Reject("CLOSURE")
    return path, value["sha256"]


def validate_close_records(event: dict[str, Any], expected_check_ids: list[str]) -> None:
    if not isinstance(event, dict):
        raise Reject("CLOSURE")
    records = event.get("records")
    if not isinstance(records, list) or [row.get("check_id") if isinstance(row, dict) else None for row in records] != expected_check_ids:
        raise Reject("CLOSURE")
    for row in records:
        if not isinstance(row, dict) or set(row) != {
            "check_id",
            "context_sha256",
            "result_sha256",
            "status",
        }:
            raise Reject("CLOSURE")
        if row["status"] != "passed" or not _check_id(row["check_id"]):
            raise Reject("CLOSURE")
        if not _hex(row["context_sha256"], 64) or not _hex(row["result_sha256"], 64):
            raise Reject("CLOSURE")


def _validate_current_observation(
    event: Any,
    *,
    index: dict[str, Any],
    context: dict[str, Any],
    capability: dict[str, Any],
    check_id: str,
    request_id: int,
    operation: str,
) -> None:
    """Validate the complete current nine-field observer identity envelope.

    The legacy accounting observer deliberately has a smaller five-field
    request ABI and is validated by ``accounting.qualification``.  This
    helper is called only for current ``tc-proof-context/v1`` records.
    """
    if not isinstance(event, dict) or set(event) != _OBSERVATION_KEYS:
        raise Reject("CLOSURE")
    if event.get("schema") != "tc-proof-observation/v1":
        raise Reject("CLOSURE")
    nonce = os.environ.get("TC_PROOF_OBSERVER_NONCE")
    source_commit = os.environ.get("TC_PROOF_ORACLE_COMMIT")
    source_tree = os.environ.get("TC_PROOF_SOURCE_TREE")
    if not isinstance(nonce, str) or not nonce:
        raise Reject("CLOSURE")
    if not isinstance(source_commit, str) or not isinstance(source_tree, str):
        raise Reject("CLOSURE")
    expected = {
        "nonce": nonce,
        "run_id": index.get("run_id"),
        "task_id": index.get("task_id"),
        "check_id": check_id,
        "request_id": request_id,
        "operation": operation,
        "source_commit": source_commit,
        "tree": source_tree,
    }
    if any(event.get(key) != value for key, value in expected.items()):
        raise Reject("CLOSURE")
    if type(event.get("request_id")) is not int or type(event.get("exit")) is not int:
        raise Reject("CLOSURE")
    if (
        context.get("run_id") != index.get("run_id")
        or context.get("task_id") != index.get("task_id")
        or context.get("check_id") != check_id
        or context.get("oracle_commit") != source_commit
        or context.get("tree") != source_tree
        or capability.get("nonce_sha256") != sha256_bytes(nonce.encode())
    ):
        raise Reject("CLOSURE")
    if event.get("exit") != 0:
        raise Reject("CLOSURE")
    if not isinstance(event.get("stdout"), str) or not isinstance(event.get("stderr"), str):
        raise Reject("CLOSURE")
    files = event.get("files")
    if not isinstance(files, dict) or any(
        not isinstance(name, str) or not isinstance(value, str)
        for name, value in files.items()
    ):
        raise Reject("CLOSURE")
    if (
        not isinstance(event.get("payload"), dict)
        or not event["payload"]
        or not isinstance(event.get("records"), list)
        or not event["records"]
        or any(not isinstance(row, dict) or not row for row in event["records"])
    ):
        raise Reject("CLOSURE")


def validate_index_close_outputs(
    event: dict[str, Any],
    index: dict[str, Any],
    output_dir: Path,
    current_check_id: str | None = None,
) -> None:
    """Validate close against the immutable context-index/v1 ABI.

    Close observes every indexed check except the selected close check. The
    observer records bind each prior result to its immutable context and raw
    runtime-result digest; no worker-provided output identifier is accepted.
    """
    if not isinstance(index, dict) or set(index) != _INDEX_KEYS:
        raise Reject("CLOSURE")
    if index.get("schema") != "tc-proof-context-index/v1":
        raise Reject("CLOSURE")
    if (
        not isinstance(index.get("task_id"), str)
        or not isinstance(index.get("run_id"), str)
        or not _hex(index.get("worktree_commit"), 40)
        or not _hex(index.get("scope_base"), 40)
    ):
        raise Reject("CLOSURE")
    run_dir = Path(index["run_id"])
    if not run_dir.is_absolute() or output_dir != run_dir / "outputs":
        raise Reject("CLOSURE")
    _real_directory(run_dir)
    _real_directory(output_dir)
    contexts_dir = run_dir / "contexts"
    results_dir = run_dir / "results"
    _real_directory(contexts_dir)
    _real_directory(results_dir)

    contexts = index.get("contexts")
    results = index.get("results")
    observer_sequences = index.get("observer_sequences")
    if not isinstance(contexts, list) or not contexts or not isinstance(results, list) or len(contexts) != len(results):
        raise Reject("CLOSURE")
    if not isinstance(observer_sequences, dict):
        raise Reject("CLOSURE")
    context_ids = [row.get("check_id") if isinstance(row, dict) else None for row in contexts]
    result_ids = [row.get("check_id") if isinstance(row, dict) else None for row in results]
    if (
        any(not _check_id(check_id) for check_id in context_ids)
        or context_ids != result_ids
        or len(set(context_ids)) != len(context_ids)
    ):
        raise Reject("CLOSURE")
    if set(observer_sequences) != set(context_ids):
        raise Reject("CLOSURE")
    if current_check_id is None:
        current_check_id = os.environ.get("TC_PROOF_CHECK_ID")
    if not _check_id(current_check_id) or current_check_id not in context_ids:
        raise Reject("CLOSURE")
    expected_ids = [check_id for check_id in context_ids if check_id != current_check_id]
    if not expected_ids:
        raise Reject("CLOSURE")
    validate_close_records(event, expected_ids)

    context_entries: dict[str, tuple[Path, str, dict[str, Any]]] = {}
    required_context = ALLOWED_V1_KEYS
    for check_id, member in zip(context_ids, contexts):
        context_path, context_hash = _bound_member(member, check_id, contexts_dir)
        context = _object(context_path)
        allowed_context = allowed_context_keys(context.get("operation"))
        if (
            set(context) - allowed_context
            or required_context - set(context)
            or context.get("schema") != "tc-proof-context/v1"
            or any(
                context.get(key) != index[key]
                for key in ("task_id", "run_id", "worktree_commit", "scope_base")
            )
            or context.get("check_id") != check_id
            or not isinstance(context.get("operation"), str)
            or not context["operation"]
            or context.get("observer_sequence") != observer_sequences.get(check_id)
        ):
            raise Reject("CLOSURE")
        context_entries[check_id] = (context_path, context_hash, context)

    for check_id, member in zip(context_ids, results):
        _, context_hash, context = context_entries[check_id]
        result_path, _ = _bound_member(member, check_id, results_dir)
        preparation = _object(result_path)
        if (
            set(preparation) != _PREPARATION_KEYS
            or preparation.get("schema") != "tc-proof-preparation-result/v1"
            or preparation.get("task_id") != index["task_id"]
            or preparation.get("check_id") != check_id
            or preparation.get("run_id") != index["run_id"]
            or preparation.get("worktree_commit") != index["worktree_commit"]
            or preparation.get("scope_base") != index["scope_base"]
            or preparation.get("context_sha256") != context_hash
            or preparation.get("status") != "ready"
        ):
            raise Reject("CLOSURE")
    observer = index.get("observer")
    if not isinstance(observer, dict) or set(observer) != {"path", "sha256"} or not _hex(observer.get("sha256"), 64):
        raise Reject("CLOSURE")
    observer_path = observer.get("path")
    if not isinstance(observer_path, str) or Path(observer_path) != run_dir / "observer.json":
        raise Reject("CLOSURE")
    observer_file = Path(observer_path)
    _regular_file(observer_file)
    try:
        observer_raw = observer_file.read_bytes()
    except OSError:
        raise Reject("CLOSURE") from None
    if sha256_bytes(observer_raw) != observer["sha256"]:
        raise Reject("CLOSURE")
    capability = _object(observer_file)
    if set(capability) != {
        "schema",
        "task_id",
        "run_id",
        "worktree_commit",
        "scope_base",
        "transport",
        "nonce_sha256",
        "sequences",
        "provider",
    } or (
        capability.get("schema") != "tc-proof-observer-capability/v1"
        or capability.get("task_id") != index["task_id"]
        or capability.get("run_id") != index["run_id"]
        or capability.get("worktree_commit") != index["worktree_commit"]
        or capability.get("scope_base") != index["scope_base"]
        or capability.get("transport") != "inherited-pipe/v1"
        or not _hex(capability.get("nonce_sha256"), 64)
        or capability.get("sequences") != observer_sequences
    ):
        raise Reject("CLOSURE")
    _bound_path_reference(capability.get("provider"))

    current_context = context_entries[current_check_id][2]
    current_sequence = current_context.get("observer_sequence")
    if current_context.get("schema") == "tc-proof-context/v1":
        if current_sequence != ["close"]:
            raise Reject("CLOSURE")
        _validate_current_observation(
            event,
            index=index,
            context=current_context,
            capability=capability,
            check_id=current_check_id,
            request_id=0,
            operation="close",
        )

    expected_names = {f"{check_id}.result.json" for check_id in expected_ids}
    report_bindings: dict[str, tuple[Path, str, dict[str, Any]]] = {}
    for check_id, (_, context_hash, context) in context_entries.items():
        qualification = context.get("qualification")
        if not isinstance(qualification, dict):
            raise Reject("CLOSURE")
        comparator = qualification.get("comparator")
        if comparator is None:
            if context["operation"] == "compare":
                raise Reject("CLOSURE")
            continue
        if not isinstance(comparator, dict):
            raise Reject("CLOSURE")
        report_path = comparator.get("report_path")
        if report_path is None:
            if context["operation"] == "compare":
                raise Reject("CLOSURE")
            continue
        if context["operation"] != "compare" or not isinstance(report_path, str):
            raise Reject("CLOSURE")
        report = Path(report_path)
        expected_report = run_dir / "outputs" / f"{check_id}.compare.json"
        if report != expected_report or report.parent != output_dir:
            raise Reject("CLOSURE")
        report_name = report.name
        if report_name in report_bindings:
            raise Reject("CLOSURE")
        report_bindings[report_name] = (report, context_hash, context)
    expected_names |= set(report_bindings)
    try:
        actual_names = {path.name for path in output_dir.iterdir()}
    except OSError:
        raise Reject("CLOSURE") from None
    if actual_names != expected_names:
        raise Reject("CLOSURE")

    report_hashes: dict[str, str] = {}
    for report_name, (report_path, context_hash, context) in report_bindings.items():
        _regular_file(report_path)
        try:
            report_raw = report_path.read_bytes()
        except OSError:
            raise Reject("CLOSURE") from None
        report = _object(report_path)
        report_hashes[context["check_id"]] = validate_comparison_report(
            report,
            report_raw,
            context,
            context_hash,
            report_path,
            require_success=True,
            category="CLOSURE",
        )

    context_by_id = {check_id: values for check_id, values in context_entries.items()}
    for row in event["records"]:
        check_id = row["check_id"]
        output = output_dir / f"{check_id}.result.json"
        _regular_file(output)
        try:
            raw = output.read_bytes()
        except OSError:
            raise Reject("CLOSURE") from None
        report = _object(output)
        _, context_hash, context = context_by_id[check_id]
        if (
            set(report) != _RUNTIME_RESULT_KEYS
            or report.get("schema") != "tc-proof-runner-result/v1"
            or report.get("run_id") != index["run_id"]
            or report.get("operation") != context["operation"]
            or report.get("context_sha256") != context_hash
            or report.get("status") != "passed"
            or report.get("category") is not None
            or not isinstance(report.get("observation_digests"), list)
            or not report["observation_digests"]
            or not all(_hex(digest, 64) for digest in report["observation_digests"])
            or not isinstance(report.get("outputs"), dict)
            or row["context_sha256"] != context_hash
            or row["result_sha256"] != sha256_bytes(raw)
        ):
            raise Reject("CLOSURE")

        outputs = report["outputs"]
        if "observations" not in outputs:
            if context["operation"] == "compare":
                report_hash = report_hashes.get(check_id)
                expected_report = run_dir / "outputs" / f"{check_id}.compare.json"
                if (
                    report_hash is None
                    or set(outputs) != {"exit_code", "report"}
                    or type(outputs.get("exit_code")) is not int
                    or outputs.get("exit_code") != 0
                    or outputs.get("report") != str(expected_report)
                    or report["observation_digests"] != [report_hash]
                ):
                    raise Reject("CLOSURE")
            elif context["operation"] != "external":
                raise Reject("CLOSURE")
        else:
            observations = outputs["observations"]
            sequence = context.get("observer_sequence")
            if (
                not isinstance(sequence, list)
                or not sequence
                or not isinstance(observations, list)
                or len(observations) != len(sequence)
            ):
                raise Reject("CLOSURE")
            for request_id, observed_operation in enumerate(sequence):
                _validate_current_observation(
                    observations[request_id],
                    index=index,
                    context=context,
                    capability=capability,
                    check_id=check_id,
                    request_id=request_id,
                    operation=observed_operation,
                )
            if report["observation_digests"] != [
                sha256_canonical(observation) for observation in observations
            ]:
                raise Reject("CLOSURE")
