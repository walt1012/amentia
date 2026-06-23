#!/usr/bin/env python3
"""Shared helpers for structured release acceptance receipts."""

from __future__ import annotations

import json
from pathlib import Path


def load_json_object(path: Path, *, label: str) -> dict[str, object]:
  if not path.is_file():
    raise FileNotFoundError(f"{label} is missing: {path}")
  value = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} must be a JSON object")
  return value


def write_json(path: Path, payload: dict[str, object]) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def require_equal(
  data: dict[str, object],
  key: str,
  expected: str,
  *,
  label: str,
) -> None:
  actual = data.get(key)
  if actual != expected:
    raise RuntimeError(f"{label} {key} must be {expected!r}, got {actual!r}")


def require_true(data: dict[str, object], key: str, *, label: str) -> None:
  if data.get(key) is not True:
    raise RuntimeError(f"{label} {key} must be true")


def require_string(
  data: dict[str, object],
  key: str,
  *,
  label: str,
  length: int | None = None,
  prefix: str | None = None,
) -> str:
  actual = data.get(key)
  if not isinstance(actual, str) or not actual.strip():
    raise RuntimeError(f"{label} {key} must be a non-empty string")
  value = actual.strip()
  if length is not None and len(value) != length:
    raise RuntimeError(f"{label} {key} must be {length} characters")
  if prefix is not None and not value.startswith(prefix):
    raise RuntimeError(f"{label} {key} must start with {prefix}")
  return value


def require_sha256_hex(value: object, *, label: str) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"{label} must be a 64-character SHA-256 hex digest")
  normalized = value.strip().lower()
  if len(normalized) != 64 or any(
    character not in "0123456789abcdef" for character in normalized
  ):
    raise RuntimeError(f"{label} must be a 64-character SHA-256 hex digest")
