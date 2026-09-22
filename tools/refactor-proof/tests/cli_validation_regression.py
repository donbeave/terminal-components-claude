#!/usr/bin/env python3
"""Adversarial parser checks for every tc-proof singleton operation."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
BINARY = Path(os.environ.get("TC_PROOF_CLI", ROOT / "target/debug/tc-proof"))


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(BINARY), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def rejected(args: tuple[str, ...], marker: str) -> None:
    result = run(*args)
    if result.returncode == 0 or marker not in result.stderr:
        raise SystemExit(
            f"malformed invocation was accepted: {args}\n"
            f"exit={result.returncode}\n{result.stderr}"
        )


def main() -> None:
    if not BINARY.is_file() or not os.access(BINARY, os.X_OK):
        raise SystemExit(f"tc-proof binary is unavailable: {BINARY}")

    rejected(
        ("compare", "--context", "/tmp/one.json", "--context", "/tmp/two.json"),
        "duplicate option: --context",
    )
    rejected(
        ("compare", "--context", "/tmp/context.json", "extra"),
        "unexpected positional argument: extra",
    )
    rejected(("compare", "--unknown"), "unknown option: --unknown")

    prepare_singletons = {
        "--task-dir": "/tmp/task",
        "--run-dir": "/tmp/run",
        "--worktree": "/tmp/worktree",
        "--scope-base": "a" * 40,
        "--oracle-tag": "refs/tags/visual-baseline",
        "--oracle-commit": "b" * 40,
        "--tool": "/tmp/tool",
        "--comparator": "/tmp/comparator",
        "--taskfmt": "/tmp/taskfmt",
        "--native-build-receipt": "/tmp/build.json",
        "--taskfmt-source": "/tmp/taskfmt-source",
        "--taskfmt-revision": "c" * 40,
        "--taskfmt-version": "0.2.0",
        "--taskfmt-sha256": "d" * 64,
        "--observer-provider": "/tmp/provider",
        "--run-id": "/tmp/run",
        "--observer-nonce": "nonce",
        "--observer-socket": "/tmp/socket",
    }
    for option, value in prepare_singletons.items():
        rejected(
            ("prepare", option, value, option, value),
            f"duplicate option: {option}",
        )
    rejected(("prepare", "extra"), "unexpected positional argument: extra")
    rejected(("prepare", "--unknown"), "unknown prepare option: --unknown")

    for option, value in {
        "--run-dir": "/tmp/run",
        "--check-id": "CHK-001",
        "--observer-socket": "/tmp/socket",
        "--timeout-ms": "1000",
    }.items():
        rejected(
            ("launch", option, value, option, value, "--", "/usr/bin/true"),
            f"duplicate option: {option}",
        )
    rejected(
        ("launch", "extra", "--", "/usr/bin/true"),
        "unexpected positional argument: extra",
    )
    rejected(("launch", "--unknown", "--", "/usr/bin/true"), "unknown launch option: --unknown")

    rejected(
        ("validate", "--run-dir", "/tmp/one", "--run-dir", "/tmp/two"),
        "duplicate option: --run-dir",
    )
    rejected(("validate", "extra"), "unexpected positional argument: extra")
    rejected(("validate", "--unknown"), "unknown validate option: --unknown")

    print("tc-proof CLI validation regressions: PASS")


if __name__ == "__main__":
    main()
