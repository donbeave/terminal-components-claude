#!/usr/bin/env python3
"""Focused regressions for Python trust bindings and comparator closure."""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

PROOF_ROOT = Path(__file__).resolve().parents[1]
if str(PROOF_ROOT) not in sys.path:
    sys.path.insert(0, str(PROOF_ROOT))

from runner.context import Reject, validate_dependencies, validate_oracle_namespace  # noqa: E402
from runner.validate import validate_comparison_report, validate_oracle_repeat  # noqa: E402


def _event(
    *,
    request_id: int,
    check_id: str = "CHK-003",
    payload: dict[str, object] | None = None,
) -> dict[str, object]:
    return {
        "schema": "tc-proof-observation/v1",
        "nonce": "oracle-repeat-nonce",
        "run_id": "run-oracle-repeat",
        "task_id": "TASK-071",
        "check_id": check_id,
        "request_id": request_id,
        "operation": "oracle",
        "source_commit": "a" * 40,
        "tree": "b" * 40,
        "exit": 0,
        "stdout": "",
        "stderr": "",
        "files": {},
        "payload": payload if payload is not None else {"value": 1},
        "records": [{"status": "observed"}],
    }


class OracleBindingTests(unittest.TestCase):
    def test_repeat_binds_identity_not_only_payload(self) -> None:
        context = {
            "run_id": "run-oracle-repeat",
            "task_id": "TASK-071",
            "check_id": "CHK-003",
            "operation": "oracle",
            "tree": "b" * 40,
            "oracle_commit": "a" * 40,
        }
        validate_oracle_repeat([_event(request_id=0), _event(request_id=1)], context=context)
        with self.assertRaises(Reject) as raised:
            validate_oracle_repeat(
                [_event(request_id=0), _event(request_id=1, check_id="CHK-004")],
                context=context,
            )
        self.assertEqual(raised.exception.category, "REPEAT")

    def test_namespace_binds_declared_check(self) -> None:
        context = {
            "qualification": {
                "family": "native",
                "check": {"namespace": "holla"},
            }
        }
        validate_oracle_namespace(context, "holla")
        with self.assertRaises(Reject):
            validate_oracle_namespace(context, "showcase")


class DependencyReceiptTests(unittest.TestCase):
    def test_receipt_binds_file_bytes_and_candidate_ancestry(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        commit = subprocess.check_output(
            ["git", "-C", str(repository), "rev-parse", "HEAD"],
            text=True,
        ).strip()
        with tempfile.TemporaryDirectory(prefix="tc-proof-dependency-") as directory:
            receipt_path = Path(directory) / "receipt.json"
            receipt_path.write_bytes(b"accepted dependency receipt\n")
            dependency = {
                "task_id": "TASK-070",
                "path": str(receipt_path),
                "sha256": hashlib.sha256(receipt_path.read_bytes()).hexdigest(),
                "accepted": True,
                "integrated": True,
                "integration_commit": commit,
            }
            context = {
                "worktree_commit": commit,
                "dependencies": [dependency],
                "qualification": {"common": {"worktree": str(repository)}},
            }
            validate_dependencies(context)
            forged = dict(dependency)
            forged["sha256"] = "0" * 64
            with self.assertRaises(Reject):
                validate_dependencies({**context, "dependencies": [forged]})


class ComparatorClosureTests(unittest.TestCase):
    def _fixture(self) -> tuple[dict[str, object], str, Path, dict[str, object], bytes]:
        root = Path(tempfile.mkdtemp(prefix="tc-proof-compare-report-")).resolve()
        report_path = root / "outputs" / "CHK-004.compare.json"
        context_hash = "c" * 64
        nested = {
            "schema": "tc-proof-compare-context/v1",
            "run_id": str(root),
            "task_id": "TASK-071",
            "check_id": "CHK-004",
            "required_ids": ["scenario-001"],
            "required_count": 1,
            "report_path": str(report_path),
        }
        context = {
            "operation": "compare",
            "run_id": str(root),
            "task_id": "TASK-071",
            "check_id": "CHK-004",
            "qualification": {
                "comparator": {
                    "report_path": str(report_path),
                    "context": nested,
                }
            },
        }
        report = {
            "schema": "tc-proof-comparison/v1",
            "run_id": str(root),
            "task_id": "TASK-071",
            "context_sha256": context_hash,
            "required_count": 1,
            "checked_count": 1,
            "passed_count": 1,
            "results": [{"id": "scenario-001", "status": "passed"}],
            "failures": [],
        }
        raw = json.dumps(report, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        return context, context_hash, report_path, report, raw

    def test_success_report_is_bound_and_nonempty(self) -> None:
        context, context_hash, report_path, report, raw = self._fixture()
        try:
            self.assertEqual(
                validate_comparison_report(
                    report,
                    raw,
                    context,
                    context_hash,
                    report_path,
                    require_success=True,
                    category="CLOSURE",
                ),
                hashlib.sha256(raw).hexdigest(),
            )
        finally:
            shutil.rmtree(context["run_id"])

    def test_empty_report_is_rejected(self) -> None:
        context, context_hash, report_path, report, raw = self._fixture()
        report["results"] = []
        report["checked_count"] = 0
        report["passed_count"] = 0
        try:
            with self.assertRaises(Reject) as raised:
                validate_comparison_report(
                    report,
                    raw,
                    context,
                    context_hash,
                    report_path,
                    require_success=True,
                    category="CLOSURE",
                )
            self.assertEqual(raised.exception.category, "CLOSURE")
        finally:
            shutil.rmtree(context["run_id"])


if __name__ == "__main__":
    unittest.main()
