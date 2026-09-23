"""Context loading and integrity checks."""

from __future__ import annotations

import itertools
import os
import re
import stat
import subprocess
from pathlib import Path
from typing import Any

from .json_util import load_path, sha256_bytes

V1_SCHEMA = "tc-proof-context/v1"
ALLOWED_V1_KEYS = {
    "schema",
    "run_id",
    "task_id",
    "check_id",
    "worktree_commit",
    "scope_base",
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
    "observer_sequence",
}
ARCHITECTURE_EXTENSION_KEYS = frozenset(
    {"architecture_profile", "branch_host_projection"}
)
NATIVE_ORACLE_NAMESPACES = frozenset({"showcase", "holla", "jackin", "tablepro", "components"})
_ALLOWED_SYSTEM_SYMLINKS = {
    Path("/etc"): Path("/private/etc"),
    Path("/home"): Path("/System/Volumes/Data/home"),
    Path("/tmp"): Path("/private/tmp"),
    Path("/var"): Path("/private/var"),
}
OBSERVER_OPERATIONS = {
    "account-tests",
    "architecture",
    "capture",
    "close",
    "compare",
    "oracle",
    "preflight",
    "required",
    "external",
}


class Reject(Exception):
    def __init__(self, category: str) -> None:
        self.category = category
        super().__init__(category)


def allowed_context_keys(operation: object) -> frozenset[str]:
    """Return the exact v1 context keys owned by one operation."""
    if operation == "architecture":
        return frozenset(ALLOWED_V1_KEYS) | ARCHITECTURE_EXTENSION_KEYS
    return frozenset(ALLOWED_V1_KEYS)


def expanded_members(axes: dict[str, list[Any]]) -> list[str]:
    return [
        "tiny/" + "/".join(str(part) for part in row)
        for row in itertools.product(axes["lanes"], axes["widths"], axes["palettes"])
    ]


def load_context(path: Path) -> tuple[dict[str, Any], bytes, str]:
    raw = path.read_bytes()
    try:
        context = load_path(path)
    except (UnicodeDecodeError, ValueError):
        raise Reject("INTEGRITY") from None
    if not isinstance(context, dict):
        raise Reject("INTEGRITY")
    return context, raw, sha256_bytes(raw)


def bind_context(context: dict[str, Any], raw_hash: str) -> None:
    expected_hash = os.environ.get("TC_PROOF_CONTEXT_SHA256")
    if expected_hash != raw_hash:
        raise Reject("INTEGRITY")
    if context.get("run_id") != os.environ.get("TC_PROOF_RUN_ID"):
        raise Reject("INTEGRITY")


def validate_schema(context: dict[str, Any]) -> None:
    schema = context.get("schema")
    keys = set(context)
    optional = {"qualification"} | (
        ARCHITECTURE_EXTENSION_KEYS
        if context.get("operation") == "architecture"
        else set()
    )
    if schema != V1_SCHEMA:
        raise Reject("INTEGRITY")
    required = ALLOWED_V1_KEYS - optional
    if keys - required - optional:
        raise Reject("INTEGRITY")
    if required - keys:
        raise Reject("INTEGRITY")
    if context.get("task_id") != os.environ.get("TC_PROOF_TASK_ID"):
        raise Reject("CONTEXT_INDEX")
    if context.get("check_id") != os.environ.get("TC_PROOF_CHECK_ID"):
        raise Reject("CONTEXT_INDEX")
    validate_observer_sequence(context, context.get("operation"))


def validate_observer_sequence(context: dict[str, Any], operation: Any) -> list[str]:
    """Validate the host-bound, ordered observer request contract.

    The sequence is part of the hashed context.  It is deliberately required
    rather than inferred from the operation or qualification family: native
    oracle checks declare ``["oracle"]`` and synthetic oracle checks declare
    ``["oracle", "oracle"]``.  No pre-sequence context is accepted.
    """
    sequence = context.get("observer_sequence")
    if (
        not isinstance(sequence, list)
        or not sequence
        or len(sequence) > 8
        or any(
            not isinstance(item, str) or item not in OBSERVER_OPERATIONS
            for item in sequence
        )
        or not isinstance(operation, str)
        or operation not in OBSERVER_OPERATIONS
        or sequence[0] != operation
    ):
        raise Reject("PROTOCOL")
    return list(sequence)


def validate_oracle_sequence(context: dict[str, Any], operation: Any) -> list[str]:
    """Validate the exact native/synthetic oracle cardinality contract."""
    sequence = validate_observer_sequence(context, operation)
    if operation != "oracle" or any(item != "oracle" for item in sequence):
        raise Reject("PROTOCOL")
    qualification = context.get("qualification")
    family = qualification.get("family") if isinstance(qualification, dict) else None
    if len(sequence) == 1:
        if family != "native":
            raise Reject("PROTOCOL")
    elif len(sequence) == 2:
        if family != "synthetic":
            raise Reject("PROTOCOL")
    else:
        raise Reject("PROTOCOL")
    return sequence


def _validate_oracle_namespace_values(namespace: object, family: object) -> None:
    """Validate the allowed namespace set without binding a context."""
    if family == "native":
        if namespace not in NATIVE_ORACLE_NAMESPACES:
            raise Reject("PROTOCOL")
        return
    if namespace != "synthetic":
        raise Reject("PROTOCOL")


def validate_oracle_namespace(
    namespace_or_context: str | dict[str, Any] | None,
    family_or_namespace: object,
) -> None:
    """Validate an oracle namespace and, when available, bind its identity.

    The two-argument legacy form remains available to the accounting runner.
    The current v1 runner passes the complete context as the first argument so
    the command-line namespace must equal the hashed check declaration.
    """
    if isinstance(namespace_or_context, dict):
        context = namespace_or_context
        namespace = family_or_namespace
        qualification = context.get("qualification")
        family = qualification.get("family") if isinstance(qualification, dict) else None
        check = qualification.get("check") if isinstance(qualification, dict) else None
        declared = check.get("namespace") if isinstance(check, dict) else None
        if not isinstance(declared, str) or namespace != declared:
            raise Reject("PROTOCOL")
        _validate_oracle_namespace_values(namespace, family)
        return
    _validate_oracle_namespace_values(namespace_or_context, family_or_namespace)


def _trusted_regular_file(path: Path, category: str) -> None:
    if not path.is_absolute():
        raise Reject(category)
    current = Path(path.anchor)
    for component in path.parts[1:]:
        current /= component
        try:
            metadata = current.lstat()
        except OSError:
            raise Reject(category) from None
        if stat.S_ISLNK(metadata.st_mode):
            allowed_target = _ALLOWED_SYSTEM_SYMLINKS.get(current)
            if allowed_target is None or current.resolve() != allowed_target:
                raise Reject(category)
    try:
        metadata = path.lstat()
    except OSError:
        raise Reject(category) from None
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise Reject(category)


def _git_ancestor(worktree: Path, commit: str) -> bool:
    try:
        result = subprocess.run(
            ["git", "-C", str(worktree), "merge-base", "--is-ancestor", commit, "HEAD"],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except OSError:
        return False
    return result.returncode == 0


def validate_tool(context: dict[str, Any]) -> None:
    tool = context["tool"]
    if not isinstance(tool, dict) or set(tool) != {"path", "sha256"}:
        raise Reject("PREFLIGHT")
    if (
        not isinstance(tool.get("path"), str)
        or not re.fullmatch(r"[0-9a-f]{64}", str(tool.get("sha256")))
    ):
        raise Reject("PREFLIGHT")
    path = Path(tool["path"])
    _trusted_regular_file(path, "PREFLIGHT")
    try:
        actual = sha256_bytes(path.read_bytes())
    except OSError:
        raise Reject("PREFLIGHT") from None
    if actual != tool["sha256"]:
        raise Reject("PREFLIGHT")


def validate_dependencies(context: dict[str, Any]) -> None:
    dependencies = context.get("dependencies")
    if not isinstance(dependencies, list):
        raise Reject("PREFLIGHT")
    seen: set[str] = set()
    for dependency in dependencies:
        if not isinstance(dependency, dict) or set(dependency) != {
            "task_id",
            "path",
            "sha256",
            "accepted",
            "integrated",
            "integration_commit",
        }:
            raise Reject("PREFLIGHT")
        task_id = dependency.get("task_id")
        path_value = dependency.get("path")
        sha256 = dependency.get("sha256")
        integration_commit = dependency.get("integration_commit")
        if (
            not isinstance(task_id, str)
            or not re.fullmatch(r"TASK-[0-9]{3}", task_id)
            or task_id in seen
            or not isinstance(path_value, str)
            or not re.fullmatch(r"[0-9a-f]{64}", str(sha256))
            or dependency.get("accepted") is not True
            or dependency.get("integrated") is not True
            or not re.fullmatch(r"[0-9a-f]{40}", str(integration_commit))
        ):
            raise Reject("PREFLIGHT")
        seen.add(task_id)
        path = Path(path_value)
        _trusted_regular_file(path, "PREFLIGHT")
        try:
            actual = sha256_bytes(path.read_bytes())
        except OSError:
            raise Reject("PREFLIGHT") from None
        if actual != sha256:
            raise Reject("PREFLIGHT")

        qualification = context.get("qualification")
        common = qualification.get("common") if isinstance(qualification, dict) else None
        worktree_value = common.get("worktree") if isinstance(common, dict) else None
        candidate_commit = context.get("worktree_commit")
        if (
            not isinstance(worktree_value, str)
            or not isinstance(candidate_commit, str)
            or not re.fullmatch(r"[0-9a-f]{40}", candidate_commit)
        ):
            raise Reject("PREFLIGHT")
        worktree = Path(worktree_value)
        if not worktree.is_absolute() or not worktree.is_dir() or not _git_ancestor(worktree, integration_commit):
            raise Reject("PREFLIGHT")


def validate_required_members(context: dict[str, Any]) -> list[str]:
    axes = context["axes"]
    if set(axes) != {"lanes", "widths", "palettes"}:
        raise Reject("REQUIRED_SET")
    expected = expanded_members(axes)
    members = context["members"]
    if members != expected:
        raise Reject("REQUIRED_SET")
    if len(set(members)) != len(members):
        raise Reject("REQUIRED_SET")
    return expected


def validate_source_authority(context: dict[str, Any], operation: str) -> None:
    if "source_directory" in context:
        raise Reject("SOURCE")
    adapter = context.get("adapter", {})
    if adapter.get("changes"):
        raise Reject("SOURCE")
    if context.get("oracle_commit") != os.environ.get("TC_PROOF_ORACLE_COMMIT"):
        raise Reject("SOURCE")
    if context.get("tree") != os.environ.get("TC_PROOF_SOURCE_TREE"):
        raise Reject("SOURCE")


def validate_close_evidence(context: dict[str, Any]) -> None:
    if context.get("candidate_success"):
        raise Reject("CLOSURE")
    evidence = context.get("evidence")
    if not isinstance(evidence, list):
        raise Reject("CLOSURE")
    if len(evidence) != 4:
        raise Reject("CLOSURE")
    tree = context["tree"]
    for row in evidence:
        if row.get("status") != "passed":
            raise Reject("CLOSURE")
        if row.get("tree") != tree:
            raise Reject("CLOSURE")
        if row.get("run_id") != context["run_id"]:
            raise Reject("CLOSURE")
