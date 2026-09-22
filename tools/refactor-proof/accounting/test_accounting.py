#!/usr/bin/env python3
"""Focused tests for shipped accounting functions."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import threading
import types
import unittest
from pathlib import Path


PROOF = Path(__file__).resolve().parents[1]


def _install_packages() -> None:
    if "refactor_proof" in sys.modules:
        return
    package = types.ModuleType("refactor_proof")
    package.__path__ = [str(PROOF)]
    sys.modules["refactor_proof"] = package
    sys.modules["refactor_proof.runner"] = types.ModuleType("refactor_proof.runner")
    sys.modules["refactor_proof.runner"].__path__ = [str(PROOF / "runner")]
    sys.modules["refactor_proof.accounting"] = types.ModuleType("refactor_proof.accounting")
    sys.modules["refactor_proof.accounting"].__path__ = [str(PROOF / "accounting")]


_install_packages()

from refactor_proof.accounting.dispatch import run_account_tests  # noqa: E402
from refactor_proof.accounting.extension import validate_extension_event  # noqa: E402
from refactor_proof.accounting.fixture import (  # noqa: E402
    build_outputs,
    validate_event,
    validate_inventory_integrity,
)
from refactor_proof.accounting.identity import (  # noqa: E402
    canonical_identity,
    original_name,
    resolve_required_names,
)
from refactor_proof.accounting.qualification import (  # noqa: E402
    qualification_validate_close_records,
    qualification_validate_index,
    validate_qualification_oracle_namespace,
    is_qualification_schema,
    validate_qualification_schema,
)
from refactor_proof.runner.context import Reject  # noqa: E402
from refactor_proof.runner.json_util import canonical, sha256_bytes  # noqa: E402


class IdentityTests(unittest.TestCase):
    def test_canonical_identity_is_sorted_json(self) -> None:
        left = {"name": "test_future", "package": "tiny-a", "profile": "primary"}
        right = {"profile": "primary", "package": "tiny-a", "name": "test_future"}
        self.assertEqual(canonical_identity(left), canonical_identity(right))

    def test_relocations_preserve_required_names(self) -> None:
        inventory = {
            "required": ["test_closed", "test_draw", "test_future", "test_update"],
            "relocations": {"test_draw": "test_renamed"},
        }
        self.assertEqual(
            resolve_required_names(inventory),
            ["test_closed", "test_future", "test_renamed", "test_update"],
        )
        self.assertEqual(original_name("test_renamed", inventory["relocations"]), "test_draw")
        self.assertEqual(original_name("test_future", inventory["relocations"]), "test_future")


class FixtureTests(unittest.TestCase):
    def test_closed_and_future_overlap_is_integrity(self) -> None:
        with self.assertRaises(Reject) as raised:
            validate_inventory_integrity({"closed": ["test_closed"], "future": {"test_closed": "owner"}})
        self.assertEqual(raised.exception.category, "INTEGRITY")

    def test_build_outputs_keeps_unresolved_and_closed_contribution(self) -> None:
        events = [{"operation": "account-tests", "payload": {"results": []}}]
        inventory = {
            "future": {"test_future": "terminal-components/completion/999"},
            "future_scenarios": {"tiny-shell": "terminal-components/completion/999"},
            "contributions": [
                {
                    "id": "tiny-shell.route",
                    "previously_closed": True,
                    "required_for_closure": True,
                }
            ],
        }
        self.assertEqual(
            build_outputs(events, inventory),
            {
                "observations": events,
                "unresolved_tests": ["test_future"],
                "closed_contributions": ["tiny-shell.route"],
                "unresolved_scenarios": ["tiny-shell"],
            },
        )

    def test_unknown_failure_is_test_accounting(self) -> None:
        inventory = {
            "required": ["test_closed", "test_draw", "test_future", "test_update"],
            "relocations": {},
            "closed": ["test_closed"],
            "future": {"test_future": "terminal-components/completion/999"},
            "package": "tiny",
            "target": "unit",
            "profile": "primary",
            "contributions": [],
        }
        event = {
            "exit": 0,
            "payload": {
                "discovered": ["test_closed", "test_draw", "test_future", "test_update"],
                "results": [
                    {"id": "test_closed", "status": "passed"},
                    {"id": "test_draw", "status": "passed"},
                    {"id": "test_future", "status": "failed"},
                    {"id": "test_update", "status": "passed"},
                    {"id": "test_unknown", "status": "failed"},
                ],
                "package": "tiny",
                "target": "unit",
                "profile": "primary",
                "scenarios": [{"state": {}, "frame": {}}],
            },
        }
        with self.assertRaises(Reject) as raised:
            validate_event(event, inventory, {})
        self.assertEqual(raised.exception.category, "TEST_ACCOUNTING")


class QualificationSchemaTests(unittest.TestCase):
    def test_native_extension_accepts_synthetic_cli_namespace(self) -> None:
        validate_qualification_oracle_namespace(
            "synthetic",
            {"schema": "tc-proof-runner-extension/v1", "family": "native"},
        )

    def test_non_extension_native_rejects_synthetic_cli_namespace(self) -> None:
        with self.assertRaises(Reject) as raised:
            validate_qualification_oracle_namespace(
                "synthetic", {"schema": "other", "family": "native"}
            )
        self.assertEqual(raised.exception.category, "PROTOCOL")

    def test_runner_context_v1_is_qualification(self) -> None:
        context = {
            "schema": "tc-proof-runner-context/v1",
            "run_id": "r",
            "operation": "account-tests",
            "tree": "0" * 40,
            "oracle_commit": "1" * 40,
            "oracle_tree": "2" * 40,
            "bundle": "/tmp/oracle.bundle",
            "bundle_sha256": "a" * 64,
            "tool": {"path": "/usr/bin/python3", "sha256": "b" * 64},
            "dependencies": [{"accepted": True, "integrated": True}],
            "adapter": {"changes": []},
            "lane": "direct",
            "axes": {"lanes": ["direct"], "widths": [8], "palettes": ["blue"]},
            "members": ["tiny/direct/8/blue"],
            "inventory": {},
            "evidence": [],
            "configuration": {},
        }
        self.assertTrue(is_qualification_schema(context))
        validate_qualification_schema(context)

    def test_native_context_is_not_qualification(self) -> None:
        self.assertFalse(is_qualification_schema({"schema": "tc-proof-context/v1"}))


class QualificationIndexTests(unittest.TestCase):
    _LAYOUT = (
        ("CHK-001", "preflight", None),
        ("CHK-002", "capture", "direct"),
        ("CHK-003", "capture", "pty"),
        ("CHK-004", "close", None),
    )
    _MEMBERS = [
        "tiny/direct/8/blue",
        "tiny/direct/8/yellow",
        "tiny/direct/12/blue",
        "tiny/direct/12/yellow",
        "tiny/pty/8/blue",
        "tiny/pty/8/yellow",
        "tiny/pty/12/blue",
        "tiny/pty/12/yellow",
    ]

    @staticmethod
    def _write(path: Path, value: object) -> str:
        raw = canonical(value)
        path.write_bytes(raw)
        return sha256_bytes(raw)

    def _build_group070(self, root: Path) -> dict[str, object]:
        root = root.resolve()
        contexts = root / "contexts"
        outputs = root / "outputs"
        contexts.mkdir()
        outputs.mkdir()
        run_id = "run-071"
        task_id = "TASK-071"
        tree = "a" * 40
        children: dict[str, dict[str, object]] = {}
        members: list[dict[str, object]] = []
        axes = {
            "lanes": ["direct", "pty"],
            "widths": [8, 12],
            "palettes": ["blue", "yellow"],
        }
        for check_id, operation, lane in self._LAYOUT:
            child = {
                "schema": "tc-proof-runner-context/v2",
                "run_id": run_id,
                "task_id": task_id,
                "check_id": check_id,
                "operation": operation,
                "tree": tree,
                "oracle_commit": "b" * 40,
                "oracle_tree": "c" * 40,
                "bundle": "bundle",
                "bundle_sha256": "d" * 64,
                "tool": {"path": sys.executable, "sha256": "e" * 64},
                "dependencies": [{"accepted": True, "integrated": True}],
                "adapter": {"changes": []},
                "lane": lane or "direct",
                "axes": axes,
                "members": self._MEMBERS,
                "inventory": {},
                "evidence": [],
                "configuration": {},
            }
            path = contexts / f"{check_id}.json"
            context_hash = self._write(path, child)
            children[check_id] = child
            members.append(
                {
                    "check_id": check_id,
                    "context_path": str(path),
                    "context_sha256": context_hash,
                    "schema": "tc-proof-runner-context/v2",
                    "operation": operation,
                    "lane": lane,
                    "namespace": None,
                    "required_ids": [
                        name
                        for name in self._MEMBERS
                        if operation == "capture" and name.split("/")[1] == lane
                    ],
                    "output_id": f"{check_id}.result.json",
                }
            )
        index = {
            "schema": "tc-proof-context-index/v1",
            "run_id": run_id,
            "task_id": task_id,
            "tree": tree,
            "trust_sha256": "f" * 64,
            "members": members,
        }
        index_path = root / "context-index.json"
        index_hash = self._write(index_path, index)
        return {
            "root": root,
            "contexts": contexts,
            "outputs": outputs,
            "index": index,
            "index_path": index_path,
            "index_hash": index_hash,
            "children": children,
            "hashes": {member["check_id"]: member["context_sha256"] for member in members},
        }

    def _build_current(self, root: Path) -> dict[str, object]:
        root = root.resolve()
        for name in ("contexts", "results", "outputs", "taskfmt-logs", "target"):
            (root / name).mkdir()
        (root / "proof-preparation.json").write_bytes(b"{}")
        run_id = str(root)
        task_id = "TASK-071"
        worktree_commit = "a" * 40
        scope_base = "b" * 40
        observer_sequences = {"CHK-001": ["preflight"]}
        context = {
            "schema": "tc-proof-context/v1",
            "task_id": task_id,
            "check_id": "CHK-001",
            "run_id": run_id,
            "worktree_commit": worktree_commit,
            "scope_base": scope_base,
            "operation": "preflight",
            "tree": "c" * 40,
            "oracle_commit": "d" * 40,
            "oracle_tree": "e" * 40,
            "bundle": "bundle",
            "bundle_sha256": "f" * 64,
            "tool": {"path": "/usr/bin/true", "sha256": "0" * 64},
            "dependencies": [],
            "adapter": {"changes": []},
            "lane": "direct",
            "axes": {"lanes": ["direct"], "widths": [8], "palettes": ["blue"]},
            "members": ["tiny/direct/8/blue"],
            "inventory": {},
            "evidence": [],
            "configuration": {},
            "observer_sequence": ["preflight"],
        }
        context_path = root / "contexts" / "CHK-001.json"
        context_hash = self._write(context_path, context)
        result = {
            "schema": "tc-proof-preparation-result/v1",
            "task_id": task_id,
            "check_id": "CHK-001",
            "run_id": run_id,
            "worktree_commit": worktree_commit,
            "scope_base": scope_base,
            "context_sha256": context_hash,
            "status": "ready",
        }
        result_path = root / "results" / "CHK-001.json"
        result_hash = self._write(result_path, result)
        observer = {
            "schema": "tc-proof-observer-capability/v1",
            "task_id": task_id,
            "run_id": run_id,
            "worktree_commit": worktree_commit,
            "scope_base": scope_base,
            "transport": "inherited-pipe/v1",
            "nonce_sha256": "1" * 64,
            "sequences": observer_sequences,
            "provider": {"path": "/usr/bin/true", "sha256": "2" * 64},
        }
        observer_path = root / "observer.json"
        observer_hash = self._write(observer_path, observer)
        index = {
            "schema": "tc-proof-context-index/v1",
            "task_id": task_id,
            "run_id": run_id,
            "worktree_commit": worktree_commit,
            "scope_base": scope_base,
            "contexts": [
                {"check_id": "CHK-001", "path": str(context_path), "sha256": context_hash}
            ],
            "results": [
                {"check_id": "CHK-001", "path": str(result_path), "sha256": result_hash}
            ],
            "observer_sequences": observer_sequences,
            "observer": {"path": str(observer_path), "sha256": observer_hash},
        }
        index_path = root / "context-index.json"
        index_hash = self._write(index_path, index)
        return {
            "root": root,
            "context": context,
            "context_hash": context_hash,
            "index": index,
            "index_path": index_path,
            "index_hash": index_hash,
        }

    def _validate(self, fixture: dict[str, object], context: dict[str, object], context_hash: str, check_id: str) -> None:
        index_path = fixture["index_path"]
        index_hash = fixture["index_hash"]
        root = fixture["root"]
        assert isinstance(index_path, Path)
        assert isinstance(root, Path)
        env = {
            "TC_PROOF_CONTEXT_INDEX": str(index_path),
            "TC_PROOF_CONTEXT_INDEX_SHA256": str(index_hash),
            "TC_PROOF_CHECK_ID": check_id,
            "TC_PROOF_TASK_ID": "TASK-071",
            "TC_PROOF_RUN_ID": str(fixture.get("run_id", "run-071")),
            "TC_PROOF_CONTEXT_SHA256": context_hash,
            "TC_PROOF_RESULT": str(root / "outputs" / f"{check_id}.result.json"),
        }
        saved = {key: os.environ.get(key) for key in env}
        os.environ.update(env)
        try:
            qualification_validate_index(context, context_hash)
        finally:
            for key, value in saved.items():
                if value is None:
                    os.environ.pop(key, None)
                else:
                    os.environ[key] = value

    def test_group070_six_key_index_with_v2_children_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self._build_group070(Path(directory))
            children = fixture["children"]
            hashes = fixture["hashes"]
            assert isinstance(children, dict)
            assert isinstance(hashes, dict)
            fixture["run_id"] = "run-071"
            self._validate(fixture, children["CHK-001"], str(hashes["CHK-001"]), "CHK-001")

    def test_group070_rejects_unexpected_top_level_shape(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self._build_group070(Path(directory))
            index = fixture["index"]
            assert isinstance(index, dict)
            index["unexpected"] = True
            index_path = fixture["index_path"]
            assert isinstance(index_path, Path)
            fixture["index_hash"] = self._write(index_path, index)
            children = fixture["children"]
            hashes = fixture["hashes"]
            assert isinstance(children, dict)
            assert isinstance(hashes, dict)
            with self.assertRaises(Reject) as raised:
                self._validate(fixture, children["CHK-001"], str(hashes["CHK-001"]), "CHK-001")
            self.assertEqual(raised.exception.category, "CONTEXT_INDEX")

    def test_group070_rejects_current_child_schema(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self._build_group070(Path(directory))
            index = fixture["index"]
            children = fixture["children"]
            hashes = fixture["hashes"]
            assert isinstance(index, dict)
            assert isinstance(children, dict)
            assert isinstance(hashes, dict)
            child = dict(children["CHK-001"])
            child["schema"] = "tc-proof-context/v1"
            path = fixture["contexts"] / "CHK-001.json"
            assert isinstance(path, Path)
            child_hash = self._write(path, child)
            index["members"][0]["context_sha256"] = child_hash
            hashes["CHK-001"] = child_hash
            fixture["index_hash"] = self._write(fixture["index_path"], index)
            with self.assertRaises(Reject) as raised:
                self._validate(fixture, child, child_hash, "CHK-001")
            self.assertEqual(raised.exception.category, "CONTEXT_INDEX")

    def test_current_nine_key_index_with_v1_child_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self._build_current(Path(directory))
            context = fixture["context"]
            self._validate(
                fixture,
                context,
                str(fixture["context_hash"]),
                "CHK-001",
            )

    def test_current_and_group070_shapes_do_not_accept_subsets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = self._build_current(Path(directory))
            index = fixture["index"]
            index_path = fixture["index_path"]
            context = fixture["context"]
            assert isinstance(index, dict)
            assert isinstance(index_path, Path)
            index.pop("observer")
            fixture["index_hash"] = self._write(index_path, index)
            with self.assertRaises(Reject) as raised:
                self._validate(
                    fixture,
                    context,
                    str(fixture["context_hash"]),
                    "CHK-001",
                )
            self.assertEqual(raised.exception.category, "CONTEXT_INDEX")

    def test_close_records_reject_extra_fields(self) -> None:
        event = {
            "records": [
                {
                    "check_id": check_id,
                    "status": "passed",
                    "context_sha256": "a" * 64,
                    "output_id": f"{check_id}.result.json",
                    "report_sha256": "b" * 64,
                    "unexpected": True,
                }
                for check_id in ("CHK-001", "CHK-002", "CHK-003")
            ]
        }
        with self.assertRaises(Reject) as raised:
            qualification_validate_close_records(event)
        self.assertEqual(raised.exception.category, "CLOSURE")


class ExtensionTests(unittest.TestCase):
    def test_borrowed_owner_is_rejected(self) -> None:
        identity_a = {
            "package": "tiny-a",
            "target": "unit",
            "profile": "primary",
            "source_commit": "c" * 40,
            "name": "test_future",
        }
        identity_b = dict(identity_a)
        identity_b["package"] = "tiny-b"
        identity_b["target"] = "integration"
        identity_b["profile"] = "msrv"
        contract = {
            "original": {
                "source_sha256": "s" * 64,
                "non_test_sha256": "n" * 64,
                "assertions": {"test_compatible": "x", "test_future": "y"},
            },
            "mode": "production",
            "accepted_inventory": True,
            "accepted_disposition": True,
            "required": [
                {**identity_a, "name": "test_compatible"},
                identity_a,
                {**identity_b, "name": "test_compatible"},
                identity_b,
            ],
            "future": [
                {
                    "identity": identity_a,
                    "owner": "terminal-components/completion/991",
                    "classification": "failed",
                },
                {
                    "identity": identity_b,
                    "owner": "terminal-components/completion/991",
                    "classification": "failed",
                },
            ],
            "closed": [],
        }
        event = {
            "payload": {
                "source": {
                    "assertions": contract["original"]["assertions"],
                    "non_test_sha256": "n" * 64,
                },
                "archive_sha256": "s" * 64,
                "runs": [],
            }
        }
        with self.assertRaises(Reject) as raised:
            validate_extension_event(event, contract, run_id="run", source_commit="c" * 40)
        self.assertEqual(raised.exception.category, "TEST_ACCOUNTING")


class DispatchTests(unittest.TestCase):
    def test_hash_mismatch_rejects_integrity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "context.json"
            context = {
                "schema": "tc-proof-runner-context/v1",
                "run_id": "run",
                "operation": "account-tests",
                "tree": "0" * 40,
                "oracle_commit": "1" * 40,
                "oracle_tree": "2" * 40,
                "bundle": "bundle",
                "bundle_sha256": "a" * 64,
                "tool": {"path": sys.executable, "sha256": "b" * 64},
                "dependencies": [{"accepted": True, "integrated": True}],
                "adapter": {"changes": []},
                "lane": "direct",
                "axes": {"lanes": ["direct", "pty"], "widths": [8, 12], "palettes": ["blue", "yellow"]},
                "members": [],
                "inventory": {"required": [], "closed": [], "future": {}},
                "evidence": [],
                "configuration": {},
            }
            raw = canonical(context)
            path.write_bytes(raw)
            result_path = Path(directory) / "result.json"
            env = {
                "TC_PROOF_CONTEXT_SHA256": "0" * 64,
                "TC_PROOF_RUN_ID": "run",
                "TC_PROOF_ORACLE_COMMIT": "1" * 40,
                "TC_PROOF_SOURCE_TREE": "0" * 40,
                "TC_PROOF_RESULT": str(result_path),
            }
            saved = {key: os.environ.get(key) for key in env}
            os.environ.update(env)
            try:
                code = run_account_tests(path)
            finally:
                for key, value in saved.items():
                    if value is None:
                        os.environ.pop(key, None)
                    else:
                        os.environ[key] = value
            self.assertEqual(code, 1)
            report = json.loads(result_path.read_text())
            self.assertEqual(report["status"], "rejected")
            self.assertEqual(report["category"], "INTEGRITY")
            self.assertEqual(report["observation_digests"], [])

    def test_executed_inventory_failure_is_accounted(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "context.json"
            inventory = {
                "source_commit": "1" * 40,
                "package": "tiny",
                "target": "unit",
                "profile": "primary",
                "required": ["test_closed", "test_draw", "test_future", "test_update"],
                "closed": ["test_closed"],
                "future": {"test_future": "terminal-components/completion/999"},
                "relocations": {},
                "contributions": [
                    {
                        "id": "tiny-shell.route",
                        "previously_closed": True,
                        "required_for_closure": True,
                        "state_key": "route_value",
                        "expected": 8,
                    }
                ],
                "scenario_expectations": {"tiny-shell": {"custom_art": "<>"}},
                "future_scenarios": {"tiny-shell": "terminal-components/completion/999"},
            }
            context = {
                "schema": "tc-proof-runner-context/v1",
                "run_id": "run",
                "operation": "account-tests",
                "tree": "0" * 40,
                "oracle_commit": "1" * 40,
                "oracle_tree": "2" * 40,
                "bundle": "bundle",
                "bundle_sha256": "a" * 64,
                "tool": {"path": sys.executable, "sha256": "b" * 64},
                "dependencies": [{"accepted": True, "integrated": True}],
                "adapter": {"changes": []},
                "lane": "direct",
                "axes": {"lanes": ["direct", "pty"], "widths": [8, 12], "palettes": ["blue", "yellow"]},
                "members": [
                    "tiny/direct/8/blue",
                    "tiny/direct/8/yellow",
                    "tiny/direct/12/blue",
                    "tiny/direct/12/yellow",
                    "tiny/pty/8/blue",
                    "tiny/pty/8/yellow",
                    "tiny/pty/12/blue",
                    "tiny/pty/12/yellow",
                ],
                "inventory": inventory,
                "evidence": [],
                "configuration": {"seed": 7, "palette": "blue"},
            }
            raw = canonical(context)
            path.write_bytes(raw)
            digest = sha256_bytes(raw)
            event = {
                "exit": 0,
                "payload": {
                    "discovered": ["test_closed", "test_draw", "test_future", "test_update"],
                    "results": [
                        {"id": "test_closed", "status": "passed"},
                        {"id": "test_draw", "status": "passed"},
                        {"id": "test_future", "status": "failed"},
                        {"id": "test_update", "status": "passed"},
                    ],
                    "package": "tiny",
                    "target": "unit",
                    "profile": "primary",
                    "scenarios": [
                        {
                            "id": "tiny-shell",
                            "state": {"route_value": 8},
                            "frame": {"custom_art": "*"},
                        }
                    ],
                },
            }
            request_read, request_write = os.pipe()
            response_read, response_write = os.pipe()

            def serve() -> None:
                with os.fdopen(request_read, "rb") as incoming, os.fdopen(response_write, "wb") as outgoing:
                    line = incoming.readline(4097)
                    self.assertTrue(line.endswith(b"\n"))
                    payload = json.loads(line)
                    self.assertEqual(
                        set(payload),
                        {"schema", "nonce", "operation", "source_commit", "tree"},
                    )
                    outgoing.write(canonical(event) + b"\n")
                    outgoing.flush()

            thread = threading.Thread(target=serve)
            thread.start()
            result_path = Path(directory) / "result.json"
            env = {
                "TC_PROOF_CONTEXT_SHA256": digest,
                "TC_PROOF_RUN_ID": "run",
                "TC_PROOF_ORACLE_COMMIT": "1" * 40,
                "TC_PROOF_SOURCE_TREE": "0" * 40,
                "TC_PROOF_RESULT": str(result_path),
                "TC_PROOF_OBSERVER_NONCE": "nonce",
                "TC_PROOF_OBSERVER_REQUEST_FD": str(request_write),
                "TC_PROOF_OBSERVER_RESPONSE_FD": str(response_read),
            }
            saved = {key: os.environ.get(key) for key in env}
            os.environ.update(env)
            try:
                code = run_account_tests(path)
            finally:
                thread.join(timeout=5)
                for key, value in saved.items():
                    if value is None:
                        os.environ.pop(key, None)
                    else:
                        os.environ[key] = value
            self.assertEqual(code, 0)
            report = json.loads(result_path.read_text())
            self.assertEqual(report["status"], "passed")
            self.assertEqual(report["outputs"]["unresolved_tests"], ["test_future"])
            self.assertEqual(report["outputs"]["closed_contributions"], ["tiny-shell.route"])
            self.assertEqual(report["outputs"]["unresolved_scenarios"], ["tiny-shell"])


if __name__ == "__main__":
    unittest.main()
