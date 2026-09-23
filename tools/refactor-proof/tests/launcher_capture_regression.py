#!/usr/bin/env python3
"""Regression checks for explicit-path real capture adapters."""

from __future__ import annotations

import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

PROOF_ROOT = Path(__file__).resolve().parents[1]
if str(PROOF_ROOT) not in sys.path:
    sys.path.insert(0, str(PROOF_ROOT))

from runner.capture import CaptureRequest, run_capture  # noqa: E402
from runner.launcher import (  # noqa: E402
    ORACLE_COMMIT,
    ORACLE_SNAPSHOTS_TREE,
    ORACLE_TAG_OBJECT,
    ORACLE_TREE,
    LauncherError,
    bind_paths,
    materialize_baseline_source,
    verify_oracle_import,
)


def _oracle(root: Path) -> Path:
    oracle = root / "oracle"
    snapshots = oracle / "snapshots"
    snapshots.mkdir(parents=True)
    (snapshots / "sentinel.txt").write_text("sealed\n", encoding="utf-8")
    (oracle / "oracle-commit").write_text(f"{ORACLE_COMMIT}\n", encoding="utf-8")
    (oracle / "snapshot-tree").write_text(
        f"{ORACLE_SNAPSHOTS_TREE}\n", encoding="utf-8"
    )
    (oracle / "tag-object").write_text(f"{ORACLE_TAG_OBJECT}\n", encoding="utf-8")
    (oracle / "content-manifest.sha256").write_text(
        "sealed-manifest\n", encoding="utf-8"
    )
    return oracle


class LauncherCaptureTests(unittest.TestCase):
    def test_all_capture_roots_are_explicit_and_no_legacy_default_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oracle = _oracle(root)
            source = root / "source"
            source.mkdir()
            with self.assertRaises(LauncherError):
                bind_paths(
                    source="relative-source",
                    oracle=oracle,
                    output=root / "output",
                    run=root / "run",
                )
            paths = bind_paths(
                source=source,
                oracle=oracle,
                output=root / "output",
                run=root / "run",
            )
            self.assertEqual(paths.environment()["TC_PROOF_SOURCE_PATH"], str(source))

    def test_real_exit_and_literal_argv0_are_recorded(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oracle = _oracle(root)
            source = root / "source"
            source.mkdir()
            executable = Path(os.path.realpath(sys.executable))
            paths = bind_paths(
                source=source,
                oracle=oracle,
                output=root / "output",
                run=root / "run",
            )
            result = run_capture(
                CaptureRequest(
                    paths=paths,
                    executable=str(executable),
                    arguments=("-c", "import sys; print('out'); print('err', file=sys.stderr); sys.exit(7)"),
                )
            )
            self.assertEqual(result.returncode, 7)
            self.assertEqual(result.status, "failed")
            receipt = json.loads(result.provenance.read_text(encoding="utf-8"))
            self.assertEqual(receipt["argv"][0], str(executable))
            self.assertEqual(receipt["argv_0"], str(executable))
            self.assertEqual(receipt["exit"], 7)
            self.assertEqual(result.stdout.read_text(encoding="utf-8").strip(), "out")
            self.assertEqual(result.stderr.read_text(encoding="utf-8").strip(), "err")

    def test_oracle_import_and_disposable_readonly_materialization(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oracle = _oracle(root)
            identity = verify_oracle_import(oracle)
            self.assertEqual(identity.commit, ORACLE_COMMIT)

            repository = root / "repo"
            repository.mkdir()
            subprocess.run(["git", "-C", str(repository), "init", "-q"], check=True)
            (repository / "Cargo.toml").write_text("[package]\nname='fixture'\n", encoding="utf-8")
            subprocess.run(["git", "-C", str(repository), "add", "Cargo.toml"], check=True)
            subprocess.run(
                [
                    "git",
                    "-C",
                    str(repository),
                    "-c",
                    "user.email=test@example.invalid",
                    "-c",
                    "user.name=test",
                    "commit",
                    "-q",
                    "-m",
                    "fixture",
                ],
                check=True,
            )
            commit = subprocess.check_output(
                ["git", "-C", str(repository), "rev-parse", "HEAD"], text=True
            ).strip()
            tree = subprocess.check_output(
                ["git", "-C", str(repository), "rev-parse", "HEAD^{tree}"], text=True
            ).strip()
            destination = root / "materialized"
            materialize_baseline_source(
                repository=repository,
                destination=destination,
                commit=commit,
                expected_tree=tree,
            )
            metadata = (destination / "Cargo.toml").stat()
            self.assertEqual(metadata.st_mode & stat.S_IWUSR, 0)
            self.assertEqual((destination / "Cargo.toml").read_text(encoding="utf-8"), "[package]\nname='fixture'\n")

if __name__ == "__main__":
    unittest.main()
