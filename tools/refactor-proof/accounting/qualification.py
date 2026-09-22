"""Runner-context/v1-v2 qualification schema, index, and group70 operations."""

from __future__ import annotations

import os
import stat
from pathlib import Path
from typing import Any

from ..runner.context import (
    Reject,
    bind_context,
    load_context,
    validate_close_evidence,
    validate_dependencies,
    validate_oracle_namespace,
    validate_required_members,
    validate_source_authority,
    validate_tool,
)
from ..runner.index import validate_index
from ..runner.json_util import canonical, load_path, sha256_bytes, sha256_canonical
from ..runner.observer import ObserverError
from ..runner.result import finish
from ..runner.validate import (
    validate_capture_payload,
    validate_index_close_outputs,
    validate_native_extension,
    validate_oracle_repeat,
)
from .observer import QualificationObserver

QUALIFICATION_V1 = "tc-proof-runner-context/v1"
QUALIFICATION_V2 = "tc-proof-runner-context/v2"
QUALIFICATION_SCHEMAS = {QUALIFICATION_V1, QUALIFICATION_V2}
EXTENSION_QUALIFICATION_SCHEMA = "tc-proof-runner-extension/v1"
ALLOWED_QUALIFICATION_V1_KEYS = {
    "schema",
    "run_id",
    "operation",
    "tree",
    "oracle_commit",
    "oracle_tree",
    "bundle",
    "bundle_sha256",
    "tool",
    "dependencies",
    "adapter",
    "lane",
    "axes",
    "members",
    "inventory",
    "evidence",
    "configuration",
    "qualification",
}
QUALIFICATION_V2_EXTRA_KEYS = {"task_id", "check_id"}
ALLOWED_SYSTEM_SYMLINKS = {
    Path("/etc"): Path("/private/etc"),
    Path("/home"): Path("/System/Volumes/Data/home"),
    Path("/tmp"): Path("/private/tmp"),
    Path("/var"): Path("/private/var"),
}
QUALIFICATION_INDEX_KEYS = {
    "schema",
    "run_id",
    "task_id",
    "tree",
    "trust_sha256",
    "members",
}
QUALIFICATION_INDEX_MEMBER_KEYS = {
    "check_id",
    "context_path",
    "context_sha256",
    "schema",
    "operation",
    "lane",
    "namespace",
    "required_ids",
    "output_id",
}
QUALIFICATION_INDEX_LAYOUT = (
    ("CHK-001", "preflight", None),
    ("CHK-002", "capture", "direct"),
    ("CHK-003", "capture", "pty"),
    ("CHK-004", "close", None),
)
QUALIFICATION_CLOSE_RECORD_KEYS = {
    "check_id",
    "status",
    "context_sha256",
    "output_id",
    "report_sha256",
}
CURRENT_INDEX_KEYS = {
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


def is_qualification_schema(context: dict[str, Any]) -> bool:
    return context.get("schema") in QUALIFICATION_SCHEMAS


def validate_qualification_oracle_namespace(
    namespace: str | None, qualification: object
) -> None:
    """Allow the extension oracle's synthetic CLI label without widening native names."""
    if (
        namespace == "synthetic"
        and isinstance(qualification, dict)
        and qualification.get("schema") == EXTENSION_QUALIFICATION_SCHEMA
        and qualification.get("family") == "native"
    ):
        return
    family = qualification.get("family") if isinstance(qualification, dict) else None
    validate_oracle_namespace(namespace, family)


def qualification_validated_executable(value: object, label: str) -> Path:
    """Allow documented macOS firmlink ancestry; reject other symlink parents."""
    if not isinstance(value, str) or not value:
        raise RuntimeError(f"{label} path is missing")
    path = Path(value)
    if not path.is_absolute():
        raise RuntimeError(f"{label} path must be absolute")

    current = Path(path.anchor)
    for component in path.parts[1:-1]:
        current /= component
        try:
            metadata = current.lstat()
        except OSError as error:
            raise RuntimeError(f"{label} path is unavailable: {current}") from error
        if stat.S_ISLNK(metadata.st_mode):
            allowed = ALLOWED_SYSTEM_SYMLINKS.get(current)
            if allowed is None or current.resolve() != allowed:
                raise RuntimeError(f"{label} path contains a symlink: {current}")
            continue
        if not stat.S_ISDIR(metadata.st_mode):
            raise RuntimeError(f"{label} parent path is not a directory: {current}")

    try:
        metadata = path.lstat()
    except OSError as error:
        raise RuntimeError(f"{label} path is unavailable: {path}") from error
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise RuntimeError(f"{label} is not a regular single-link file")
    if not os.access(path, os.X_OK):
        raise RuntimeError(f"{label} is not executable")
    return path


def _qualification_hex(value: object, length: int) -> bool:
    return (
        isinstance(value, str)
        and len(value) == length
        and all(character in "0123456789abcdef" for character in value)
    )


def _qualification_check_parents(path: Path, category: str) -> None:
    if not path.is_absolute():
        raise Reject(category)
    current = Path(path.anchor)
    for component in path.parts[1:-1]:
        current /= component
        try:
            metadata = current.lstat()
        except OSError:
            raise Reject(category) from None
        if stat.S_ISLNK(metadata.st_mode):
            allowed = ALLOWED_SYSTEM_SYMLINKS.get(current)
            if allowed is None or current.resolve() != allowed:
                raise Reject(category)
        elif not stat.S_ISDIR(metadata.st_mode):
            raise Reject(category)


def _qualification_regular_file(path: Path, category: str) -> None:
    _qualification_check_parents(path, category)
    try:
        metadata = path.lstat()
    except OSError:
        raise Reject(category) from None
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise Reject(category)


def _qualification_regular_directory(path: Path, category: str) -> None:
    _qualification_check_parents(path, category)
    try:
        metadata = path.lstat()
    except OSError:
        raise Reject(category) from None
    if stat.S_ISLNK(metadata.st_mode) or not stat.S_ISDIR(metadata.st_mode):
        raise Reject(category)


def validate_qualification_schema(context: dict[str, Any]) -> None:
    schema = context.get("schema")
    keys = set(context)
    optional = {"qualification"}
    if schema == QUALIFICATION_V1:
        required = ALLOWED_QUALIFICATION_V1_KEYS - optional
    elif schema == QUALIFICATION_V2:
        required = (ALLOWED_QUALIFICATION_V1_KEYS | QUALIFICATION_V2_EXTRA_KEYS) - optional
    else:
        raise Reject("INTEGRITY")
    if keys - required - optional:
        raise Reject("INTEGRITY")
    if required - keys:
        raise Reject("INTEGRITY")
    if schema == QUALIFICATION_V2:
        if context.get("task_id") != os.environ.get("TC_PROOF_TASK_ID"):
            raise Reject("CONTEXT_INDEX")
        if context.get("check_id") != os.environ.get("TC_PROOF_CHECK_ID"):
            raise Reject("CONTEXT_INDEX")


def qualification_load_index() -> tuple[dict[str, Any], str]:
    index_path = os.environ.get("TC_PROOF_CONTEXT_INDEX")
    index_hash = os.environ.get("TC_PROOF_CONTEXT_INDEX_SHA256")
    if not index_path or not _qualification_hex(index_hash, 64):
        raise Reject("CONTEXT_INDEX")
    path = Path(index_path)
    if path.name != "context-index.json":
        raise Reject("CONTEXT_INDEX")
    _qualification_regular_file(path, "CONTEXT_INDEX")
    try:
        raw = path.read_bytes()
        index = load_path(path)
    except (OSError, TypeError, ValueError):
        raise Reject("CONTEXT_INDEX") from None
    if sha256_bytes(raw) != index_hash:
        raise Reject("CONTEXT_INDEX")
    if not isinstance(index, dict):
        raise Reject("CONTEXT_INDEX")
    return index, index_hash


def _validate_group070_index(
    context: dict[str, Any],
    context_hash: str,
    index: dict[str, Any],
    index_path: Path,
) -> None:
    if set(index) != QUALIFICATION_INDEX_KEYS:
        raise Reject("CONTEXT_INDEX")
    if index["schema"] != "tc-proof-context-index/v1":
        raise Reject("CONTEXT_INDEX")
    if (
        not isinstance(index.get("run_id"), str)
        or not index["run_id"]
        or not isinstance(index.get("task_id"), str)
        or not index["task_id"]
        or not _qualification_hex(index.get("tree"), 40)
        or not _qualification_hex(index.get("trust_sha256"), 64)
    ):
        raise Reject("CONTEXT_INDEX")
    if (
        index["run_id"] != context.get("run_id")
        or index["run_id"] != os.environ.get("TC_PROOF_RUN_ID")
        or index["task_id"] != os.environ.get("TC_PROOF_TASK_ID")
        or index["tree"] != context.get("tree")
        or os.environ.get("TC_PROOF_CONTEXT_SHA256") != context_hash
    ):
        raise Reject("CONTEXT_INDEX")
    members = index["members"]
    if not isinstance(members, list) or len(members) != len(QUALIFICATION_INDEX_LAYOUT):
        raise Reject("CONTEXT_INDEX")
    if any(not isinstance(member, dict) for member in members):
        raise Reject("CONTEXT_INDEX")
    contexts_dir = index_path.parent / "contexts"
    _qualification_regular_directory(contexts_dir, "CONTEXT_INDEX")
    expected_ids = {check_id for check_id, _, _ in QUALIFICATION_INDEX_LAYOUT}
    expected_files = {check_id + ".json" for check_id in expected_ids}
    try:
        actual_files = {path.name for path in contexts_dir.iterdir()}
    except OSError:
        raise Reject("CONTEXT_INDEX") from None
    if actual_files != expected_files:
        raise Reject("CONTEXT_INDEX")
    check_id = os.environ.get("TC_PROOF_CHECK_ID")
    if check_id not in expected_ids:
        raise Reject("CONTEXT_INDEX")
    result_value = os.environ.get("TC_PROOF_RESULT")
    if not isinstance(result_value, str) or not result_value:
        raise Reject("CONTEXT_INDEX")
    result_path = Path(result_value)
    if result_path.name != f"{check_id}.result.json":
        raise Reject("CONTEXT_INDEX")
    output_dir = result_path.parent
    _qualification_regular_directory(output_dir, "CONTEXT_INDEX")
    selected_index = next(
        index
        for index, (expected_id, _, _) in enumerate(QUALIFICATION_INDEX_LAYOUT)
        if expected_id == check_id
    )
    expected_outputs = {
        expected_id + ".result.json"
        for expected_id, _, _ in QUALIFICATION_INDEX_LAYOUT[:selected_index]
    }
    try:
        actual_outputs = {path.name for path in output_dir.iterdir()}
    except OSError:
        raise Reject("CONTEXT_INDEX") from None
    if actual_outputs != expected_outputs:
        raise Reject("CONTEXT_INDEX")
    for output_name in expected_outputs:
        _qualification_regular_file(output_dir / output_name, "CONTEXT_INDEX")

    optional = {"qualification"}
    required_child = (ALLOWED_QUALIFICATION_V1_KEYS | QUALIFICATION_V2_EXTRA_KEYS) - optional
    selected = None
    selected_child = None
    for member, (expected_id, expected_operation, expected_lane) in zip(
        members, QUALIFICATION_INDEX_LAYOUT
    ):
        if set(member) != QUALIFICATION_INDEX_MEMBER_KEYS:
            raise Reject("CONTEXT_INDEX")
        if (
            member.get("check_id") != expected_id
            or member.get("schema") != QUALIFICATION_V2
            or member.get("operation") != expected_operation
            or member.get("lane") != expected_lane
            or member.get("namespace") is not None
            or member.get("output_id") != f"{expected_id}.result.json"
            or not isinstance(member.get("required_ids"), list)
            or not _qualification_hex(member.get("context_sha256"), 64)
            or not isinstance(member.get("context_path"), str)
        ):
            raise Reject("CONTEXT_INDEX")
        child_path = Path(member["context_path"])
        expected_path = contexts_dir / f"{expected_id}.json"
        if child_path != expected_path:
            raise Reject("CONTEXT_INDEX")
        _qualification_regular_file(child_path, "CONTEXT_INDEX")
        try:
            child_raw = child_path.read_bytes()
        except OSError:
            raise Reject("CONTEXT_INDEX") from None
        if sha256_bytes(child_raw) != member["context_sha256"]:
            raise Reject("CONTEXT_INDEX")
        try:
            child = load_path(child_path)
        except (OSError, TypeError, ValueError):
            raise Reject("CONTEXT_INDEX") from None
        if not isinstance(child, dict):
            raise Reject("CONTEXT_INDEX")
        child_keys = set(child)
        if child_keys - required_child - optional or required_child - child_keys:
            raise Reject("CONTEXT_INDEX")
        if (
            child.get("schema") != QUALIFICATION_V2
            or child.get("schema") != member.get("schema")
        ):
            raise Reject("CONTEXT_INDEX")
        bindings = {
            "run_id": index["run_id"],
            "tree": index["tree"],
            "task_id": index["task_id"],
            "check_id": member["check_id"],
            "operation": member["operation"],
        }
        if any(child.get(key) != value for key, value in bindings.items()):
            raise Reject("CONTEXT_INDEX")
        if member["operation"] == "capture" and child.get("lane") != member["lane"]:
            raise Reject("CONTEXT_INDEX")
        try:
            required_members = validate_required_members(child)
            expected_required_ids = [
                name
                for name in required_members
                if member["operation"] == "capture"
                and len(name.split("/")) == 4
                and name.split("/")[1] == member["lane"]
            ]
        except (KeyError, Reject, TypeError):
            raise Reject("CONTEXT_INDEX") from None
        if member["required_ids"] != expected_required_ids:
            raise Reject("CONTEXT_INDEX")
        if member["check_id"] == check_id:
            selected = member
            selected_child = child
    if selected is None:
        raise Reject("CONTEXT_INDEX")
    if context_hash != selected["context_sha256"] or context != selected_child:
        raise Reject("CONTEXT_INDEX")


def qualification_validate_index(context: dict[str, Any], context_hash: str) -> None:
    index, _ = qualification_load_index()
    index_keys = set(index)
    if index_keys == CURRENT_INDEX_KEYS:
        try:
            validate_index(context, context_hash)
        except Reject:
            raise
        except (KeyError, OSError, TypeError, ValueError):
            raise Reject("CONTEXT_INDEX") from None
        return
    if index_keys != QUALIFICATION_INDEX_KEYS:
        raise Reject("CONTEXT_INDEX")
    _validate_group070_index(
        context,
        context_hash,
        index,
        Path(os.environ["TC_PROOF_CONTEXT_INDEX"]),
    )


def qualification_validate_close_records(event: dict[str, Any]) -> None:
    records = event.get("records")
    if not isinstance(records, list) or len(records) != 3:
        raise Reject("CLOSURE")
    if [row.get("check_id") if isinstance(row, dict) else None for row in records] != [
        "CHK-001",
        "CHK-002",
        "CHK-003",
    ]:
        raise Reject("CLOSURE")
    for row in records:
        if (
            not isinstance(row, dict)
            or set(row) != QUALIFICATION_CLOSE_RECORD_KEYS
            or row.get("status") != "passed"
            or not _qualification_hex(row.get("context_sha256"), 64)
            or not _qualification_hex(row.get("report_sha256"), 64)
        ):
            raise Reject("CLOSURE")


def qualification_validate_index_close_outputs(
    event: dict[str, Any],
    index: dict[str, Any],
    output_dir: Path,
) -> None:
    records = event.get("records")
    if not isinstance(records, list):
        raise Reject("CLOSURE")
    qualification_validate_close_records(event)
    output_dir = Path(output_dir)
    _qualification_regular_directory(output_dir, "CLOSURE")
    if (
        set(index) != QUALIFICATION_INDEX_KEYS
        or not isinstance(index.get("members"), list)
        or len(index["members"]) != len(QUALIFICATION_INDEX_LAYOUT)
        or any(
            not isinstance(member, dict)
            or set(member) != QUALIFICATION_INDEX_MEMBER_KEYS
            for member in index["members"]
        )
    ):
        raise Reject("CLOSURE")
    for member, (expected_id, expected_operation, expected_lane) in zip(
        index["members"], QUALIFICATION_INDEX_LAYOUT
    ):
        if (
            member.get("check_id") != expected_id
            or member.get("schema") != QUALIFICATION_V2
            or member.get("operation") != expected_operation
            or member.get("lane") != expected_lane
            or member.get("namespace") is not None
            or member.get("output_id") != f"{expected_id}.result.json"
            or not isinstance(member.get("required_ids"), list)
            or not _qualification_hex(member.get("context_sha256"), 64)
        ):
            raise Reject("CLOSURE")
    members = {member["check_id"]: member for member in index["members"]}
    if set(members) != {check_id for check_id, _, _ in QUALIFICATION_INDEX_LAYOUT}:
        raise Reject("CLOSURE")
    expected_names = {
        members[check_id]["output_id"] for check_id in ("CHK-001", "CHK-002", "CHK-003")
    }
    try:
        actual_names = {path.name for path in output_dir.iterdir()}
    except OSError:
        raise Reject("CLOSURE") from None
    if actual_names != expected_names:
        raise Reject("CLOSURE")
    for row in records:
        if not isinstance(row, dict) or set(row) != QUALIFICATION_CLOSE_RECORD_KEYS:
            raise Reject("CLOSURE")
        member = members.get(row.get("check_id"))
        if member is None or row.get("output_id") != member.get("output_id"):
            raise Reject("CLOSURE")
        output = output_dir / member["output_id"]
        _qualification_regular_file(output, "CLOSURE")
        try:
            report = load_path(output)
        except (OSError, TypeError, ValueError):
            raise Reject("CLOSURE") from None
        if not isinstance(report, dict):
            raise Reject("CLOSURE")
        if sha256_canonical(report) != row.get("report_sha256"):
            raise Reject("CLOSURE")
        if row.get("context_sha256") != member["context_sha256"]:
            raise Reject("CLOSURE")
        if row.get("output_id") != member["output_id"]:
            raise Reject("CLOSURE")


def _reported_hash(context_hash: str) -> str:
    return os.environ.get("TC_PROOF_CONTEXT_SHA256", context_hash)


def _guard_qualification(context: dict[str, Any], context_hash: str) -> None:
    if os.environ.get("TC_PROOF_CONTEXT_INDEX"):
        qualification_validate_index(context, context_hash)
        return
    bind_context(context, context_hash)


def _load_or_reject(context_path: Path, default_operation: str) -> tuple[dict[str, Any], str, str] | Reject:
    try:
        context, _, context_hash = load_context(context_path)
    except Reject as error:
        return error
    return context, context_hash, context.get("operation", default_operation)


def _collect_qualification_observations(
    client: QualificationObserver,
    *,
    operation: str,
    source_commit: str,
    tree: str,
    count: int,
) -> list[dict[str, Any]]:
    events = []
    for _ in range(count):
        events.append(client.request(operation, source_commit, tree))
    return events


def _validate_execution_events(
    events: list[dict[str, Any]],
    context: dict[str, Any],
    expected_members: list[str],
    operation: str,
) -> None:
    lane = context["lane"]
    config = context["configuration"]
    for event in events:
        validate_capture_payload(
            event,
            lane=lane,
            expected_members=expected_members,
            config=config,
            operation=operation,
        )


def run_qualification_preflight(context_path: Path) -> int:
    loaded = _load_or_reject(context_path, "preflight")
    if isinstance(loaded, Reject):
        return finish("rejected", loaded.category, {}, [], "preflight", _reported_hash("0" * 64))
    context, context_hash, operation = loaded
    reported = _reported_hash(context_hash)
    try:
        _guard_qualification(context, context_hash)
        validate_qualification_schema(context)
        validate_tool(context)
        validate_dependencies(context)
        if context.get("candidate_success"):
            raise Reject("CLOSURE")
        return finish("passed", None, {"validated": True}, [], operation, reported)
    except Reject as error:
        return finish("rejected", error.category, {}, [], operation, reported)


def run_qualification_required(context_path: Path) -> int:
    loaded = _load_or_reject(context_path, "required")
    if isinstance(loaded, Reject):
        return finish("rejected", loaded.category, {}, [], "required", _reported_hash("0" * 64))
    context, context_hash, operation = loaded
    reported = _reported_hash(context_hash)
    try:
        _guard_qualification(context, context_hash)
        validate_qualification_schema(context)
        members = validate_required_members(context)
        return finish("passed", None, {"members": members}, [], operation, reported)
    except Reject as error:
        return finish("rejected", error.category, {}, [], operation, reported)


def run_qualification_oracle(context_path: Path, namespace: str | None) -> int:
    loaded = _load_or_reject(context_path, "oracle")
    if isinstance(loaded, Reject):
        return finish("rejected", loaded.category, {}, [], "oracle", _reported_hash("0" * 64))
    context, context_hash, operation = loaded
    reported = _reported_hash(context_hash)
    events: list[dict[str, Any]] = []
    digests: list[str] = []
    try:
        _guard_qualification(context, context_hash)
        qualification = context.get("qualification")
        validate_qualification_oracle_namespace(namespace, qualification)
        validate_source_authority(context, operation)
        validate_qualification_schema(context)
        expected_members = validate_required_members(context)
        client = QualificationObserver.from_env()
        if qualification and qualification.get("family") == "native":
            events = _collect_qualification_observations(
                client,
                operation=operation,
                source_commit=os.environ["TC_PROOF_ORACLE_COMMIT"],
                tree=os.environ["TC_PROOF_SOURCE_TREE"],
                count=1,
            )
            digests = [client.digest(event) for event in events]
            validate_native_extension(events[0]["payload"], qualification, context["oracle_commit"])
            return finish("passed", None, {"observations": events}, digests, operation, reported)
        events = _collect_qualification_observations(
            client,
            operation=operation,
            source_commit=os.environ["TC_PROOF_ORACLE_COMMIT"],
            tree=os.environ["TC_PROOF_SOURCE_TREE"],
            count=2,
        )
        digests = [client.digest(event) for event in events]
        validate_oracle_repeat(events)
        _validate_execution_events(events, context, expected_members, operation)
        return finish("passed", None, {"observations": events}, digests, operation, reported)
    except Reject as error:
        return finish("rejected", error.category, {}, digests, operation, reported)
    except ObserverError:
        return finish("rejected", "EXECUTION", {}, digests, operation, reported)


def run_qualification_capture(context_path: Path, lane: str | None) -> int:
    loaded = _load_or_reject(context_path, "capture")
    if isinstance(loaded, Reject):
        return finish("rejected", loaded.category, {}, [], "capture", _reported_hash("0" * 64))
    context, context_hash, operation = loaded
    reported = _reported_hash(context_hash)
    events: list[dict[str, Any]] = []
    digests: list[str] = []
    try:
        _guard_qualification(context, context_hash)
        if lane != context["lane"]:
            raise Reject("PROTOCOL")
        validate_source_authority(context, operation)
        validate_qualification_schema(context)
        expected_members = validate_required_members(context)
        client = QualificationObserver.from_env()
        events = _collect_qualification_observations(
            client,
            operation=operation,
            source_commit=os.environ["TC_PROOF_ORACLE_COMMIT"],
            tree=os.environ["TC_PROOF_SOURCE_TREE"],
            count=1,
        )
        digests = [client.digest(event) for event in events]
        _validate_execution_events(events, context, expected_members, operation)
        return finish("passed", None, {"observations": events}, digests, operation, reported)
    except Reject as error:
        return finish("rejected", error.category, {}, digests, operation, reported)
    except ObserverError:
        return finish("rejected", "EXECUTION", {}, digests, operation, reported)


def run_qualification_close(context_path: Path) -> int:
    loaded = _load_or_reject(context_path, "close")
    if isinstance(loaded, Reject):
        return finish("rejected", loaded.category, {}, [], "close", _reported_hash("0" * 64))
    context, context_hash, operation = loaded
    reported = _reported_hash(context_hash)
    events: list[dict[str, Any]] = []
    digests: list[str] = []
    try:
        if context.get("candidate_success"):
            raise Reject("CLOSURE")
        _guard_qualification(context, context_hash)
        validate_qualification_schema(context)
        if os.environ.get("TC_PROOF_CONTEXT_INDEX"):
            index, _ = qualification_load_index()
            client = QualificationObserver.from_env()
            events = _collect_qualification_observations(
                client,
                operation=operation,
                source_commit=os.environ["TC_PROOF_ORACLE_COMMIT"],
                tree=os.environ["TC_PROOF_SOURCE_TREE"],
                count=1,
            )
            digests = [client.digest(event) for event in events]
            output_dir = Path(os.environ["TC_PROOF_RESULT"]).parent
            if set(index) == CURRENT_INDEX_KEYS:
                validate_index_close_outputs(
                    events[0], index, output_dir, context.get("check_id")
                )
            else:
                qualification_validate_close_records(events[0])
                qualification_validate_index_close_outputs(events[0], index, output_dir)
            return finish("passed", None, {"observations": events}, digests, operation, reported)
        validate_close_evidence(context)
        client = QualificationObserver.from_env()
        events = _collect_qualification_observations(
            client,
            operation=operation,
            source_commit=os.environ["TC_PROOF_ORACLE_COMMIT"],
            tree=os.environ["TC_PROOF_SOURCE_TREE"],
            count=1,
        )
        digests = [client.digest(event) for event in events]
        return finish("passed", None, {"observations": events}, digests, operation, reported)
    except Reject as error:
        return finish("rejected", error.category, {}, digests, operation, reported)
    except ObserverError:
        return finish("rejected", "CLOSURE", {}, digests, operation, reported)


def qualification_dispatch(args: Any) -> int:
    account_tests = globals().get("run_account_tests")
    if account_tests is None:
        from .dispatch import run_account_tests as account_tests

    dispatch = {
        "preflight": lambda: run_qualification_preflight(args.context),
        "required": lambda: run_qualification_required(args.context),
        "oracle": lambda: run_qualification_oracle(args.context, args.namespace),
        "capture": lambda: run_qualification_capture(args.context, args.lane),
        "account-tests": lambda: account_tests(args.context),
        "close": lambda: run_qualification_close(args.context),
    }
    handler = dispatch.get(args.operation)
    if handler is None:
        import sys

        print(f"tc-proof: unknown command: {args.operation}", file=sys.stderr)
        return 2

    previous_save = globals().get("save_path")
    if callable(previous_save):
        def _qualification_save_path(path: Any, value: Any) -> None:
            Path(path).write_bytes(canonical(value))

        globals()["save_path"] = _qualification_save_path
    try:
        return handler()
    finally:
        if callable(previous_save):
            globals()["save_path"] = previous_save
