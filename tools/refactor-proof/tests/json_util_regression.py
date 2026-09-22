#!/usr/bin/env python3
"""Cross-runtime strict JSON and canonicalization regressions."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

PROOF_ROOT = Path(__file__).resolve().parents[1]
if str(PROOF_ROOT) not in sys.path:
    sys.path.insert(0, str(PROOF_ROOT))

from runner.json_util import canonical, load_bytes, sha256_canonical  # noqa: E402


class StrictJsonTests(unittest.TestCase):
    def test_duplicate_keys_are_rejected_recursively(self) -> None:
        for payload in (
            b'{"outer":{"key":1,"key":2}}',
            b'{"outer":[{"key":1,"key":2}]}',
        ):
            with self.subTest(payload=payload):
                with self.assertRaises(ValueError):
                    load_bytes(payload)

    def test_non_finite_and_trailing_values_are_rejected(self) -> None:
        for payload in (b"NaN", b"Infinity", b"-Infinity", b"1 2", b"{}[]"):
            with self.subTest(payload=payload):
                with self.assertRaises(ValueError):
                    load_bytes(payload)

    def test_canonical_unicode_bytes_and_hash_match_rust_contract(self) -> None:
        value = {"z": "café 😀", "a": {"雪": "é"}}
        self.assertEqual(canonical(value), '{"a":{"雪":"é"},"z":"café 😀"}'.encode("utf-8"))
        self.assertEqual(
            sha256_canonical(value),
            "afa3e0356e44ebedd3b3c759b32bc6135b3553e816bcc1c408d3f8bc47e52bf3",
        )

    def test_canonical_rejects_non_finite_python_values(self) -> None:
        for value in (float("nan"), float("inf"), float("-inf")):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    canonical({"value": value})


if __name__ == "__main__":
    unittest.main()
