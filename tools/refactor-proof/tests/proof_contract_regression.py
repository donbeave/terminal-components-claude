#!/usr/bin/env python3
"""Regression guards for the normative proof-contract ABI."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


CONTRACT = (
    Path(__file__).resolve().parents[3]
    / "docs/refactoring-plan/proof-contract.md"
)
TEXT = CONTRACT.read_text(encoding="utf-8")


def section(title: str) -> str:
    marker = f"## {title}\n"
    start = TEXT.index(marker) + len(marker)
    end = TEXT.find("\n## ", start)
    return TEXT[start:] if end == -1 else TEXT[start:end]


class ProofContractTests(unittest.TestCase):
    def test_capture_and_membership_are_production_bound_and_exact(self) -> None:
        body = " ".join(
            section("Production-bound capture and exact artifact closure").split()
        )
        for phrase in (
            "real production-bound capture path",
            "PTY, update, draw, input, resize, settle, and cleanup path",
            "read-only oracle manifest set exactly",
            "7,550 keys",
            "30,200 artifacts",
            "ANSI, plain text, PNG, and HTML",
            "Missing, extra, duplicate, substituted, or unknown",
        ):
            self.assertIn(phrase, body)

    def test_receipt_and_comparator_abi_binds_all_provenance(self) -> None:
        body = " ".join(section("Receipt, comparator, and execution ABI").split())
        for schema in (
            "tc-proof-capture-receipt/v1",
            "tc-proof-comparator-report/v1",
            "additionalProperties: false",
            "canonical JSON ABI",
            "tc-proof-architecture-profile/v1",
        ):
            self.assertIn(schema, body)
        for field in (
            "source_commit",
            "source_tree",
            "tool_sha256",
            "oracle_sha256",
            "producer_sha256",
            "reviewer_sha256",
            "evidence_sha256",
            "command",
            "argv",
            "argv_sha256",
            "observer",
            "exit",
        ):
            self.assertIn(field, body)

        self.assertRegex(body, r"exit == 0.*only successful execution", re.DOTALL)
        self.assertIn("MUST be a JSON integer, not a Boolean or string", body)
        self.assertIn("lexicographically sorted object keys", body)
        self.assertIn("no trailing newline", body)


if __name__ == "__main__":
    unittest.main()
