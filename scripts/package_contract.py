#!/usr/bin/env python3
"""Shared package contract for Pith macOS release automation."""

from __future__ import annotations

import os
from pathlib import Path


APP_NAME = "Pith"
PACKAGE_MANIFEST_SCHEMA_VERSION = 1
SUPPORTED_ARCH = "x86_64"
MINIMUM_SYSTEM_VERSION = "12.0"
DEFAULT_MODEL_ID = "lfm2.5-350m"
DEFAULT_MODEL_MANIFEST_RELATIVE_PATH = Path(
  "models/builtin/lfm2.5-350m/model-pack.json"
)
MODEL_DELIVERY_MODE = "in-app-download"
MODEL_WEIGHTS_BUNDLED = False
MODEL_METADATA_BUNDLED = True
DEFAULT_MAX_APP_BUNDLE_BYTES = 250 * 1024 * 1024
DEFAULT_MAX_ZIP_ARTIFACT_BYTES = 150 * 1024 * 1024
PROHIBITED_MODEL_SUFFIXES = {".gguf", ".bin", ".safetensors"}

SANDBOX_CONTRACT = {
  "mode": "workspaceReadWrite",
  "backend": "runtime-detected",
  "fallback": "processOnlyWhenNativeUnavailable",
  "networkDefault": "disabled",
}
DAILY_DRIVER_CONTRACT = {
  "stageSource": "runtime/readiness",
  "nextActionSource": "runtime/readiness",
  "presentation": "app-header-inspector",
  "packagedSmoke": "required",
}


def package_size_budget() -> dict[str, int]:
  return {
    "maxAppBundleBytes": env_positive_int(
      "PITH_MAX_APP_BUNDLE_BYTES",
      DEFAULT_MAX_APP_BUNDLE_BYTES,
    ),
    "maxZipArtifactBytes": env_positive_int(
      "PITH_MAX_ZIP_ARTIFACT_BYTES",
      DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
    ),
  }


def env_positive_int(name: str, default: int) -> int:
  raw_value = os.environ.get(name)
  if raw_value is None:
    return default
  try:
    value = int(raw_value)
  except ValueError as error:
    raise RuntimeError(f"{name} must be a positive integer") from error
  if value <= 0:
    raise RuntimeError(f"{name} must be a positive integer")
  return value


def validate_package_size_budget(value: object, label: str) -> dict[str, int]:
  if not isinstance(value, dict):
    raise RuntimeError(f"{label} sizeBudget is required")
  result: dict[str, int] = {}
  for field in ("maxAppBundleBytes", "maxZipArtifactBytes"):
    field_value = value.get(field)
    if not isinstance(field_value, int) or field_value <= 0:
      raise RuntimeError(f"{label} sizeBudget {field} must be a positive integer")
    result[field] = field_value
  return result


def assert_size_under_budget(size_bytes: int, max_bytes: int, label: str) -> None:
  if size_bytes > max_bytes:
    raise RuntimeError(
      f"{label} is {size_bytes} bytes, above the {max_bytes} byte release budget. "
      "Keep model weights, multi-arch runtimes, and optional connectors out of the package."
    )


def directory_size_bytes(path: Path) -> int:
  total = 0
  for entry in path.rglob("*"):
    if entry.is_file():
      total += entry.stat().st_size
  return total


def bundled_model_weight_files(path: Path) -> list[Path]:
  return sorted(
    entry
    for entry in path.rglob("*")
    if entry.is_file() and entry.suffix.lower() in PROHIBITED_MODEL_SUFFIXES
  )
