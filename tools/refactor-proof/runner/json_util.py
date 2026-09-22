"""Strict JSON helpers for tc-proof runner."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
from typing import Any, NoReturn


def canonical(value: Any) -> bytes:
    return json.dumps(
        value,
        allow_nan=False,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_canonical(value: Any) -> str:
    return sha256_bytes(canonical(value))


def load_bytes(data: bytes) -> Any:
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    def reject_constant(value: str) -> NoReturn:
        raise ValueError(f"invalid JSON constant: {value}")

    return json.loads(
        data.decode("utf-8"),
        object_pairs_hook=unique,
        parse_constant=reject_constant,
    )


def load_path(path) -> Any:
    return load_bytes(path.read_bytes())


def save_path(path, value: Any) -> None:
    payload = canonical(value)
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    if hasattr(os, "O_NOFOLLOW"):
        flags |= os.O_NOFOLLOW
    descriptor = os.open(str(Path(path)), flags, 0o444)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = -1
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
    finally:
        if descriptor != -1:
            os.close(descriptor)
