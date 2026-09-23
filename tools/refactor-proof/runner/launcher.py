#!/usr/bin/env python3
"""Explicit-path host launcher for disposable proof captures.

This module owns path and oracle provenance.  It never invents a checkout,
oracle, output, or run directory: callers must bind all four explicitly.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import tarfile
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Sequence


ORACLE_COMMIT = "4a79c0a2d40fca46fc406b77157ce3b3f12ec16b"
ORACLE_TREE = "0b1f13431fdfd6060cf9f45a114afa5a99cc6c26"
ORACLE_SNAPSHOTS_TREE = "3f0261c32849e26feda24d87697de4a7ce6b8375"
ORACLE_TAG_OBJECT = "1ee5ebdcb91fd87adb9a5b28e43d4c7f421706c5"

SOURCE_ENV = "TC_PROOF_SOURCE_PATH"
ORACLE_ENV = "TC_PROOF_ORACLE_PATH"
OUTPUT_ENV = "TC_PROOF_OUTPUT_PATH"
RUN_ENV = "TC_PROOF_RUN_PATH"
EXECUTABLE_ENV = "TC_PROOF_EXECUTABLE"


class LauncherError(RuntimeError):
    """A path, provenance, or launch contract violation."""


@dataclass(frozen=True)
class BoundPaths:
    """The four external roots owned by one capture invocation."""

    source: Path
    oracle: Path
    output: Path
    run: Path

    def environment(self) -> dict[str, str]:
        return {
            SOURCE_ENV: str(self.source),
            ORACLE_ENV: str(self.oracle),
            OUTPUT_ENV: str(self.output),
            RUN_ENV: str(self.run),
        }


@dataclass(frozen=True)
class OracleIdentity:
    path: Path
    commit: str
    tree: str
    snapshots_tree: str
    tag_object: str
    manifest_sha256: str

    def as_dict(self) -> dict[str, str]:
        return {
            "path": str(self.path),
            "commit": self.commit,
            "tree": self.tree,
            "snapshots_tree": self.snapshots_tree,
            "tag_object": self.tag_object,
            "manifest_sha256": self.manifest_sha256,
        }


def _path(raw: str | os.PathLike[str], label: str) -> Path:
    try:
        value = os.fspath(raw)
    except TypeError as error:
        raise LauncherError(f"{label} is not a path") from error
    if not isinstance(value, str) or not value:
        raise LauncherError(f"{label} is empty")
    path = Path(value)
    if not path.is_absolute():
        raise LauncherError(f"{label} must be absolute: {value}")
    if ".." in path.parts:
        raise LauncherError(f"{label} must not contain parent traversal: {value}")
    if path == Path(path.anchor):
        raise LauncherError(f"{label} must not be the filesystem root: {value}")
    return path


def _lstat(path: Path, label: str) -> os.stat_result:
    try:
        metadata = path.lstat()
    except OSError as error:
        raise LauncherError(f"{label} is unavailable: {path}") from error
    if stat.S_ISLNK(metadata.st_mode):
        raise LauncherError(f"{label} must not be a symlink: {path}")
    return metadata


def _directory(path: Path, label: str, *, create: bool) -> None:
    if path.exists():
        metadata = _lstat(path, label)
        if not stat.S_ISDIR(metadata.st_mode):
            raise LauncherError(f"{label} is not a directory: {path}")
        return
    if not create:
        raise LauncherError(f"{label} is missing: {path}")
    try:
        path.mkdir(parents=True, exist_ok=False)
    except FileExistsError as error:
        raise LauncherError(f"{label} appeared during creation: {path}") from error
    except OSError as error:
        raise LauncherError(f"cannot create {label}: {path}") from error
    _directory(path, label, create=False)


def _overlaps(left: Path, right: Path) -> bool:
    return left == right or left in right.parents or right in left.parents


def bind_paths(
    *,
    source: str | os.PathLike[str],
    oracle: str | os.PathLike[str],
    output: str | os.PathLike[str],
    run: str | os.PathLike[str],
) -> BoundPaths:
    """Bind explicit roots and reject cross-root writes or stale ambiguity."""

    paths = BoundPaths(
        source=_path(source, "source"),
        oracle=_path(oracle, "oracle"),
        output=_path(output, "output"),
        run=_path(run, "run"),
    )
    _directory(paths.source, "source", create=False)
    _directory(paths.oracle, "oracle", create=False)
    if _overlaps(paths.source, paths.oracle):
        raise LauncherError("source and oracle paths overlap")
    if _overlaps(paths.source, paths.output) or _overlaps(paths.source, paths.run):
        raise LauncherError("capture roots overlap the source path")
    if _overlaps(paths.oracle, paths.output) or _overlaps(paths.oracle, paths.run):
        raise LauncherError("capture roots overlap the oracle path")
    if paths.run in paths.output.parents:
        raise LauncherError("run path must not be inside output")
    return paths


def prepare_run(paths: BoundPaths) -> None:
    """Create fresh output/run roots without deleting or reusing old state."""

    _directory(paths.run, "run", create=True)
    if any(paths.run.iterdir()):
        raise LauncherError(f"run directory is not fresh: {paths.run}")
    if paths.output == paths.run:
        return
    _directory(paths.output, "output", create=True)
    if any(paths.output.iterdir()):
        raise LauncherError(f"output directory is not fresh: {paths.output}")


def _read_sidecar(root: Path, name: str, expected: str) -> str:
    path = root / name
    metadata = _lstat(path, f"oracle sidecar {name}")
    if not stat.S_ISREG(metadata.st_mode):
        raise LauncherError(f"oracle sidecar {name} is not a file")
    try:
        value = path.read_text(encoding="utf-8").strip()
    except OSError as error:
        raise LauncherError(f"cannot read oracle sidecar {name}") from error
    if value != expected:
        raise LauncherError(
            f"oracle sidecar {name} mismatch: {value or '<empty>'} != {expected}"
        )
    return value


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def verify_oracle_import(
    oracle: str | os.PathLike[str],
    *,
    expected_commit: str = ORACLE_COMMIT,
    expected_tree: str = ORACLE_TREE,
    expected_snapshots_tree: str = ORACLE_SNAPSHOTS_TREE,
    expected_tag_object: str = ORACLE_TAG_OBJECT,
) -> OracleIdentity:
    """Verify the sealed import identity without writing to it."""

    root = _path(oracle, "oracle")
    _directory(root, "oracle", create=False)
    commit = _read_sidecar(root, "oracle-commit", expected_commit)
    tree = _read_sidecar(root, "snapshot-tree", expected_snapshots_tree)
    tag_object = _read_sidecar(root, "tag-object", expected_tag_object)
    manifest = root / "content-manifest.sha256"
    metadata = _lstat(manifest, "oracle content manifest")
    if not stat.S_ISREG(metadata.st_mode):
        raise LauncherError("oracle content manifest is not a regular file")
    manifest_hash = _sha256(manifest)
    snapshot_root = root / "snapshots"
    _directory(snapshot_root, "oracle snapshots", create=False)
    if _sha256(manifest) != manifest_hash:
        raise LauncherError("oracle content manifest changed during validation")
    return OracleIdentity(root, commit, expected_tree, tree, tag_object, manifest_hash)


def _safe_member(name: str) -> Path:
    candidate = Path(name)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise LauncherError(f"baseline archive contains unsafe path: {name}")
    return candidate


def _readonly_tree(root: Path) -> None:
    for path in sorted(root.rglob("*"), key=lambda item: len(item.parts), reverse=True):
        try:
            metadata = path.lstat()
        except OSError as error:
            raise LauncherError(f"cannot inspect materialized source: {path}") from error
        if stat.S_ISLNK(metadata.st_mode):
            raise LauncherError(f"materialized source contains a symlink: {path}")
        mode = metadata.st_mode & 0o777
        if stat.S_ISDIR(metadata.st_mode):
            os.chmod(path, mode & ~0o222 | 0o555)
        elif stat.S_ISREG(metadata.st_mode):
            os.chmod(path, mode & ~0o222 | 0o444)
    os.chmod(root, root.stat().st_mode & ~0o222 | 0o555)


def materialize_baseline_source(
    *,
    repository: str | os.PathLike[str],
    destination: str | os.PathLike[str],
    commit: str = ORACLE_COMMIT,
    expected_tree: str = ORACLE_TREE,
    sparse_paths: Sequence[str] | None = None,
) -> Path:
    """Materialize an exact, disposable, read-only source tree from Git.

    The repository and commit are read-only inputs.  The destination must not
    already exist and is the only path this function creates.
    """

    repo = _path(repository, "source repository")
    _directory(repo, "source repository", create=False)
    target = _path(destination, "materialized source")
    if target.exists():
        raise LauncherError(f"materialized source already exists: {target}")
    _directory(target.parent, "materialized source parent", create=False)

    try:
        checked = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "--verify", f"{commit}^{{commit}}"],
            capture_output=True,
            text=True,
            check=False,
        )
        if checked.returncode != 0 or checked.stdout.strip() != commit:
            raise LauncherError("baseline source commit is unavailable or not exact")
        tree = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", "--verify", f"{commit}^{{tree}}"],
            capture_output=True,
            text=True,
            check=False,
        )
        if tree.returncode != 0 or tree.stdout.strip() != expected_tree:
            raise LauncherError("baseline source tree does not match the sealed oracle")
        archive = subprocess.Popen(
            ["git", "-C", str(repo), "archive", "--format=tar", commit],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        assert archive.stdout is not None
        assert archive.stderr is not None
        target.mkdir()
        selected = tuple(_safe_member(item).as_posix().rstrip("/") for item in (sparse_paths or ()))
        with tarfile.open(fileobj=archive.stdout, mode="r|") as stream:
            for member in stream:
                relative = _safe_member(member.name)
                name = relative.as_posix()
                if selected and not any(name == item or name.startswith(f"{item}/") for item in selected):
                    continue
                destination_path = target / relative
                if member.isdir():
                    destination_path.mkdir(parents=True, exist_ok=True)
                    continue
                if not member.isfile():
                    raise LauncherError(f"baseline archive contains unsupported entry: {name}")
                destination_path.parent.mkdir(parents=True, exist_ok=True)
                source_stream = stream.extractfile(member)
                if source_stream is None:
                    raise LauncherError(f"cannot read baseline archive entry: {name}")
                with destination_path.open("xb") as output_stream:
                    shutil.copyfileobj(source_stream, output_stream)
                os.chmod(destination_path, member.mode & ~0o222 | 0o444)
        stderr = archive.stderr.read().decode("utf-8", errors="replace")
        exit_code = archive.wait()
        archive.stdout.close()
        archive.stderr.close()
        if exit_code != 0:
            raise LauncherError(f"baseline source archive failed: {stderr.strip()}")
        _readonly_tree(target)
        return target
    except LauncherError:
        raise
    except (OSError, tarfile.TarError, ValueError) as error:
        raise LauncherError(f"baseline source materialization failed: {error}") from error


def _parse_env(values: Sequence[str]) -> Mapping[str, str]:
    result: dict[str, str] = {}
    for raw in values:
        key, separator, value = raw.partition("=")
        if not separator or re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key) is None:
            raise LauncherError(f"invalid environment assignment: {raw}")
        if "\x00" in value:
            raise LauncherError(f"environment value contains NUL: {key}")
        result[key] = value
    return result


def launch(
    *,
    paths: BoundPaths,
    executable: str | os.PathLike[str],
    arguments: Sequence[str] = (),
    environment: Mapping[str, str] | None = None,
    timeout: float = 600.0,
):
    """Run one real capture through the capture adapter."""

    try:
        from .capture import CaptureRequest, run_capture
    except ImportError:  # pragma: no cover - direct CLI execution
        from capture import CaptureRequest, run_capture  # type: ignore[no-redef]

    request = CaptureRequest(
        paths=paths,
        executable=os.fspath(executable),
        arguments=tuple(arguments),
        environment=dict(environment or {}),
        timeout=timeout,
    )
    return run_capture(request)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="tc-proof-launcher")
    subparsers = parser.add_subparsers(dest="command", required=True)
    materialize = subparsers.add_parser("materialize")
    materialize.add_argument("--repository", required=True)
    materialize.add_argument("--destination", required=True)
    materialize.add_argument("--commit", default=ORACLE_COMMIT)
    materialize.add_argument("--tree", default=ORACLE_TREE)
    materialize.add_argument("--sparse", action="append", default=[])
    run = subparsers.add_parser("launch")
    for name in ("source", "oracle", "output", "run", "executable"):
        run.add_argument(f"--{name}", required=True)
    run.add_argument("--timeout", type=float, default=600.0)
    run.add_argument("--env", action="append", default=[])
    run.add_argument("arguments", nargs=argparse.REMAINDER)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "materialize":
            materialize_baseline_source(
                repository=args.repository,
                destination=args.destination,
                commit=args.commit,
                expected_tree=args.tree,
                sparse_paths=args.sparse,
            )
            return 0
        paths = bind_paths(
            source=args.source,
            oracle=args.oracle,
            output=args.output,
            run=args.run,
        )
        verify_oracle_import(paths.oracle)
        arguments = list(args.arguments)
        if arguments and arguments[0] == "--":
            arguments.pop(0)
        result = launch(
            paths=paths,
            executable=args.executable,
            arguments=arguments,
            environment=_parse_env(args.env),
            timeout=args.timeout,
        )
        print(json.dumps(result.as_dict(), sort_keys=True, separators=(",", ":")))
        return result.returncode if result.returncode >= 0 else 128 + -result.returncode
    except LauncherError as error:
        print(f"tc-proof-launcher: {error}", file=os.sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
