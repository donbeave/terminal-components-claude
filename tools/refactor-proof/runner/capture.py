#!/usr/bin/env python3
"""Real subprocess capture with explicit provenance and lifecycle."""

from __future__ import annotations

import hashlib
import json
import os
import signal
import subprocess
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Mapping, Sequence

try:
    from .launcher import (
        EXECUTABLE_ENV,
        BoundPaths,
        LauncherError,
        OracleIdentity,
        ORACLE_COMMIT,
        prepare_run,
        verify_oracle_import,
    )
except ImportError:  # pragma: no cover - direct CLI execution
    from launcher import (  # type: ignore[no-redef]
        EXECUTABLE_ENV,
        BoundPaths,
        LauncherError,
        OracleIdentity,
        ORACLE_COMMIT,
        prepare_run,
        verify_oracle_import,
    )


class CaptureError(LauncherError):
    """A capture could not establish trustworthy execution evidence."""


@dataclass(frozen=True)
class CaptureRequest:
    paths: BoundPaths
    executable: str
    arguments: tuple[str, ...] = ()
    environment: Mapping[str, str] = field(default_factory=dict)
    timeout: float = 600.0

    def __post_init__(self) -> None:
        if not isinstance(self.executable, str) or not self.executable:
            raise CaptureError("executable path is missing")
        if "\x00" in self.executable:
            raise CaptureError("executable path contains NUL")
        if not self.executable.startswith("/"):
            raise CaptureError("executable path must be absolute")
        if ".." in Path(self.executable).parts:
            raise CaptureError("executable path must not contain parent traversal")
        if self.timeout <= 0:
            raise CaptureError("capture timeout must be positive")
        if any(not isinstance(value, str) or "\x00" in value for value in self.arguments):
            raise CaptureError("capture arguments are invalid")
        if any(
            not isinstance(key, str)
            or not isinstance(value, str)
            or not key
            or "\x00" in key
            or "\x00" in value
            for key, value in dict(self.environment).items()
        ):
            raise CaptureError("capture environment is invalid")

    @property
    def argv(self) -> list[str]:
        # Keep this literal.  HTML oracle provenance compares argv[0] byte-for-byte.
        return [self.executable, *self.arguments]


@dataclass(frozen=True)
class CaptureResult:
    returncode: int
    timed_out: bool
    status: str
    provenance: Path
    stdout: Path
    stderr: Path
    argv: tuple[str, ...]

    def as_dict(self) -> dict[str, object]:
        return {
            "returncode": self.returncode,
            "timed_out": self.timed_out,
            "status": self.status,
            "provenance": str(self.provenance),
            "stdout": str(self.stdout),
            "stderr": str(self.stderr),
            "argv": list(self.argv),
        }


def _sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _executable_identity(literal: str) -> tuple[Path, str]:
    path = Path(literal)
    try:
        metadata = path.lstat()
    except OSError as error:
        raise CaptureError(f"executable is unavailable: {literal}") from error
    if path.is_symlink() or not path.is_file():
        raise CaptureError(f"executable is not a regular file: {literal}")
    if metadata.st_nlink != 1 or not os.access(path, os.X_OK):
        raise CaptureError(f"executable is not single-link and executable: {literal}")
    try:
        return path, _sha256_file(path)
    except OSError as error:
        raise CaptureError(f"cannot hash executable: {literal}") from error


def _write_new(path: Path, payload: bytes, *, mode: int = 0o644) -> None:
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    try:
        descriptor = os.open(path, flags, mode)
    except OSError as error:
        raise CaptureError(f"cannot create capture artifact: {path}") from error
    try:
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = -1
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
    finally:
        if descriptor != -1:
            os.close(descriptor)


def _timestamp() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _terminate(process: subprocess.Popen[bytes]) -> None:
    """Terminate the process group and reap every child."""

    try:
        os.killpg(process.pid, signal.SIGTERM)
    except (OSError, AttributeError):
        try:
            process.terminate()
        except OSError:
            pass
    try:
        process.wait(timeout=1.0)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except (OSError, AttributeError):
            try:
                process.kill()
            except OSError:
                pass
        process.wait()


def _environment(request: CaptureRequest, oracle: OracleIdentity) -> dict[str, str]:
    environment = os.environ.copy()
    bindings = {
        **request.paths.environment(),
        EXECUTABLE_ENV: request.executable,
        "TC_PROOF_ORACLE_COMMIT": oracle.commit,
    }
    supplied = dict(request.environment)
    for key, value in bindings.items():
        if key in supplied and supplied[key] != value:
            raise CaptureError(f"capture environment rebinding: {key}")
        environment[key] = value
    environment.update(supplied)
    environment.update(bindings)
    return environment


def run_capture(request: CaptureRequest) -> CaptureResult:
    """Execute exactly one real child and publish its observed exit status."""

    oracle = verify_oracle_import(request.paths.oracle)
    executable, executable_sha256 = _executable_identity(request.executable)
    prepare_run(request.paths)
    environment = _environment(request, oracle)
    started_at = _timestamp()
    started = time.monotonic()
    timed_out = False
    try:
        process = subprocess.Popen(
            request.argv,
            cwd=str(request.paths.source),
            env=environment,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            shell=False,
            start_new_session=True,
        )
    except OSError as error:
        raise CaptureError(f"real executable could not start: {error}") from error
    try:
        try:
            stdout, stderr = process.communicate(timeout=request.timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            _terminate(process)
            stdout, stderr = process.communicate()
    except BaseException:
        if process.poll() is None:
            _terminate(process)
        raise
    finished = time.monotonic()
    returncode = process.returncode
    if returncode is None:
        raise CaptureError("child exit status was not observed")
    after_sha256 = _sha256_file(executable)
    provenance = {
        "schema": "tc-proof-capture-receipt/v1",
        "source": str(request.paths.source),
        "oracle": oracle.as_dict(),
        "output": str(request.paths.output),
        "run": str(request.paths.run),
        "executable": {
            "path": request.executable,
            "sha256": executable_sha256,
            "sha256_after": after_sha256,
        },
        "argv": request.argv,
        "argv_0": request.argv[0],
        "command": request.argv,
        "exit": returncode,
        "timed_out": timed_out,
        "status": "passed" if returncode == 0 and not timed_out else "failed",
        "started_at": started_at,
        "duration_seconds": finished - started,
        "stdout_sha256": _sha256_bytes(stdout),
        "stderr_sha256": _sha256_bytes(stderr),
    }
    if after_sha256 != executable_sha256:
        provenance["status"] = "rejected"
        provenance["error"] = "executable changed during capture"
    encoded = json.dumps(
        provenance,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    stdout_path = request.paths.output / "stdout"
    stderr_path = request.paths.output / "stderr"
    run_stdout = request.paths.run / "stdout"
    run_stderr = request.paths.run / "stderr"
    _write_new(stdout_path, stdout)
    _write_new(stderr_path, stderr)
    if run_stdout != stdout_path:
        _write_new(run_stdout, stdout)
    if run_stderr != stderr_path:
        _write_new(run_stderr, stderr)
    _write_new(request.paths.run / "exit.status", f"{returncode}\n".encode())
    provenance_path = request.paths.run / "capture.json"
    _write_new(provenance_path, encoded)
    if after_sha256 != executable_sha256:
        raise CaptureError("executable changed during capture")
    return CaptureResult(
        returncode=returncode,
        timed_out=timed_out,
        status=str(provenance["status"]),
        provenance=provenance_path,
        stdout=stdout_path,
        stderr=stderr_path,
        argv=tuple(request.argv),
    )


def main(argv: Sequence[str] | None = None) -> int:
    import argparse

    parser = argparse.ArgumentParser(prog="tc-proof-capture")
    for name in ("source", "oracle", "output", "run", "executable"):
        parser.add_argument(f"--{name}", required=True)
    parser.add_argument("--timeout", type=float, default=600.0)
    parser.add_argument("--env", action="append", default=[])
    parser.add_argument("arguments", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    try:
        from .launcher import bind_paths
    except ImportError:  # pragma: no cover - direct CLI execution
        from launcher import bind_paths  # type: ignore[no-redef]
    try:
        values: dict[str, str] = {}
        for raw in args.env:
            key, separator, value = raw.partition("=")
            if not separator or not key:
                raise CaptureError(f"invalid environment assignment: {raw}")
            values[key] = value
        arguments = list(args.arguments)
        if arguments and arguments[0] == "--":
            arguments.pop(0)
        paths = bind_paths(
            source=args.source,
            oracle=args.oracle,
            output=args.output,
            run=args.run,
        )
        result = run_capture(
            CaptureRequest(
                paths=paths,
                executable=args.executable,
                arguments=tuple(arguments),
                environment=values,
                timeout=args.timeout,
            )
        )
        print(json.dumps(result.as_dict(), sort_keys=True, separators=(",", ":")))
        return result.returncode if result.returncode >= 0 else 128 + -result.returncode
    except LauncherError as error:
        print(f"tc-proof-capture: {error}", file=os.sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
