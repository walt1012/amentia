#!/usr/bin/env python3
"""Shared helpers for structured acceptance evidence contracts."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path


PLACEHOLDER_VALUES = {"todo", "tbd", "n/a", "na", "none", "placeholder"}


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


def reject_placeholder_text(value: object, key: str, *, label: str) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"{label} {key} must be a non-empty string")
  normalized = value.strip().lower()
  if normalized in PLACEHOLDER_VALUES or normalized.startswith("todo") or "fill" in normalized:
    raise RuntimeError(f"{label} {key} must not be placeholder text")


def require_sha256_hex(value: object, *, label: str) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"{label} must be a 64-character SHA-256 hex digest")
  normalized = value.strip().lower()
  if len(normalized) != 64 or any(character not in "0123456789abcdef" for character in normalized):
    raise RuntimeError(f"{label} must be a 64-character SHA-256 hex digest")


def require_utc_timestamp(
  value: object,
  *,
  label: str,
  example: str,
) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"{label} must be a UTC timestamp")
  try:
    datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
  except ValueError as error:
    raise RuntimeError(f"{label} must use UTC ISO format like {example}") from error


def require_empty_list(data: dict[str, object], key: str, *, label: str) -> None:
  actual = data.get(key)
  if actual != []:
    raise RuntimeError(f"{label} {key} must be an empty list")


def require_words(
  value: object,
  key: str,
  words: tuple[str, ...],
  *,
  label: str,
) -> None:
  if not isinstance(value, str):
    raise RuntimeError(f"{label} {key} must be a non-empty string")
  normalized = value.lower()
  missing = [word for word in words if word not in normalized]
  if missing:
    missing_label = ", ".join(missing)
    raise RuntimeError(f"{label} {key} must mention: {missing_label}")
