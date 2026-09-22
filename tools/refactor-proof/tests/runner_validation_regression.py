#!/usr/bin/env python3
"""Adversarial operation-schema checks for current runner contexts."""

from __future__ import annotations

import os
import sys
import unittest
from pathlib import Path
from unittest.mock import patch


PROOF_ROOT = Path(__file__).resolve().parents[1]
if str(PROOF_ROOT) not in sys.path:
    sys.path.insert(0, str(PROOF_ROOT))

from runner.context import ALLOWED_V1_KEYS, Reject, V1_SCHEMA, validate_schema  # noqa: E402


TASK_ID = "TASK-071"


def context(operation: object) -> dict[str, object]:
    value: dict[str, object] = {key: None for key in ALLOWED_V1_KEYS}
    value.update(
        {
            "schema": V1_SCHEMA,
            "run_id": "/private/run",
            "task_id": TASK_ID,
            "check_id": "CHK-001",
            "worktree_commit": "a" * 40,
            "scope_base": "b" * 40,
            "operation": operation,
            "observer_sequence": [operation] if isinstance(operation, str) else [],
        }
    )
    return value


class CurrentContextSchemaTests(unittest.TestCase):
    def validate(self, value: dict[str, object]) -> None:
        with patch.dict(
            os.environ,
            {"TC_PROOF_TASK_ID": TASK_ID, "TC_PROOF_CHECK_ID": "CHK-001"},
            clear=False,
        ):
            validate_schema(value)

    def test_architecture_extensions_are_operation_scoped(self) -> None:
        value = context("architecture")
        value["architecture_profile"] = {"schema": "tc-architecture-rust-profile/v1"}
        value["branch_host_projection"] = {"schema": "tc-branch-host-projection/v1"}
        self.validate(value)

    def test_non_architecture_context_cannot_import_architecture_fields(self) -> None:
        for operation in ("preflight", "account-tests", "compare", "external"):
            with self.subTest(operation=operation):
                value = context(operation)
                value["architecture_profile"] = {}
                with self.assertRaises(Reject) as raised:
                    self.validate(value)
                self.assertEqual(raised.exception.category, "INTEGRITY")

    def test_branch_projection_is_not_global_extension(self) -> None:
        value = context("capture")
        value["branch_host_projection"] = {}
        with self.assertRaises(Reject) as raised:
            self.validate(value)
        self.assertEqual(raised.exception.category, "INTEGRITY")

    def test_malformed_operation_is_rejected_without_type_error(self) -> None:
        value = context(["preflight"])
        with self.assertRaises(Reject) as raised:
            self.validate(value)
        self.assertEqual(raised.exception.category, "PROTOCOL")


if __name__ == "__main__":
    unittest.main()
