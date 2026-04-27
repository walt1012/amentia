#!/usr/bin/env python3

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = REPO_ROOT / "models" / "builtin" / "lfm2.5-350m" / "model-pack.json"
README_PATH = REPO_ROOT / "models" / "builtin" / "lfm2.5-350m" / "README.md"
SWIFT_CATALOG_PATH = (
  REPO_ROOT
  / "apps"
  / "pith-macos"
  / "Sources"
  / "PithApp"
  / "LocalModelCatalog.swift"
)
REQUIRED_KEYS = {
  "id",
  "display_name",
  "file_name",
  "context_size",
  "model_context_size",
  "max_output_tokens",
  "backend",
  "homepage",
  "download_url",
  "sha256",
  "size_bytes",
  "license",
}
FORBIDDEN_SUFFIXES = (".gguf", ".bin", ".safetensors")
CATALOG_FIELD_MAP = {
  "id": ("id", str),
  "display_name": ("displayName", str),
  "file_name": ("fileName", str),
  "homepage": ("homepage", str),
  "download_url": ("downloadURL", str),
  "sha256": ("sha256", str),
  "size_bytes": ("sizeBytes", int),
  "context_size": ("contextSize", int),
  "model_context_size": ("modelContextSize", int),
  "max_output_tokens": ("maxOutputTokens", int),
  "license": ("license", str),
}
CATALOG_INVARIANT_KEYS = {
  "id",
  "displayName",
  "fileName",
  "homepage",
  "downloadURL",
  "sha256",
  "sizeBytes",
  "contextSize",
  "modelContextSize",
  "maxOutputTokens",
  "license",
}


def main() -> int:
  if not MANIFEST_PATH.is_file():
    raise SystemExit(f"model manifest missing: {MANIFEST_PATH}")
  if not README_PATH.is_file():
    raise SystemExit(f"model pack README missing: {README_PATH}")

  manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
  missing_keys = sorted(REQUIRED_KEYS.difference(manifest.keys()))
  if missing_keys:
    raise SystemExit(f"model manifest missing keys: {', '.join(missing_keys)}")

  if manifest["id"] != "lfm2.5-350m":
    raise SystemExit("model manifest id must be lfm2.5-350m")
  if manifest["backend"] != "llama.cpp":
    raise SystemExit("model manifest backend must be llama.cpp")
  if not str(manifest["file_name"]).lower().endswith(".gguf"):
    raise SystemExit("model manifest file_name must point to a .gguf artifact")
  if manifest["context_size"] > manifest["model_context_size"]:
    raise SystemExit("model manifest context_size must not exceed model_context_size")
  if manifest["max_output_tokens"] > manifest["context_size"]:
    raise SystemExit("model manifest max_output_tokens must not exceed context_size")

  swift_catalog_entries = load_swift_catalog_entries()
  validate_swift_catalog_entries(swift_catalog_entries)
  swift_catalog = next(
    entry for entry in swift_catalog_entries
    if entry["id"] == "lfm2.5-350m"
  )
  for manifest_key, (swift_key, _value_type) in CATALOG_FIELD_MAP.items():
    if manifest[manifest_key] != swift_catalog[swift_key]:
      raise SystemExit(
        f"default model manifest {manifest_key} does not match Swift catalog "
        f"{swift_key}: {manifest[manifest_key]!r} != {swift_catalog[swift_key]!r}"
      )

  tracked_files = subprocess.check_output(
    ["git", "ls-files", "models", "model-packs"],
    cwd=REPO_ROOT,
    text=True,
  ).splitlines()
  forbidden_files = [
    tracked_path for tracked_path in tracked_files
    if tracked_path.lower().endswith(FORBIDDEN_SUFFIXES)
  ]
  if forbidden_files:
    raise SystemExit(
      "tracked model weight files are forbidden in git history: "
      + ", ".join(forbidden_files)
    )

  print("model pack manifest is valid")
  return 0


def load_swift_catalog_entries() -> list[dict[str, object]]:
  if not SWIFT_CATALOG_PATH.is_file():
    raise SystemExit(f"Swift model catalog missing: {SWIFT_CATALOG_PATH}")

  text = SWIFT_CATALOG_PATH.read_text(encoding="utf-8")
  default_id = extract_swift_string_constant(text, "defaultFirstUseModelID")
  blocks = re.findall(
    r"LocalModelCatalogItem\((.*?)\n\s*\)",
    text,
    flags=re.DOTALL,
  )
  if not blocks:
    raise SystemExit("Swift model catalog entries were not found")

  entries = [parse_swift_catalog_entry(block, default_id) for block in blocks]
  if not any(entry["id"] == default_id for entry in entries):
    raise SystemExit("default Swift model catalog entry was not found")

  return entries


def parse_swift_catalog_entry(block: str, default_id: str) -> dict[str, object]:
  catalog: dict[str, object] = {}
  for _manifest_key, (swift_key, value_type) in CATALOG_FIELD_MAP.items():
    catalog[swift_key] = extract_swift_value(block, swift_key, value_type, default_id)

  return catalog


def extract_swift_value(
  block: str,
  swift_key: str,
  value_type: type,
  default_id: str,
) -> object:
  if swift_key == "id":
    if re.search(r"id:\s*defaultFirstUseModelID", block):
      return default_id

  if value_type is int:
    value_match = re.search(rf"{swift_key}:\s*([0-9_]+)", block)
    if not value_match:
      raise SystemExit(f"Swift model catalog missing {swift_key}")
    return int(value_match.group(1).replace("_", ""))

  value_match = re.search(rf'{swift_key}:\s*"([^"]+)"', block)
  if not value_match:
    raise SystemExit(f"Swift model catalog missing {swift_key}")
  return value_match.group(1)


def extract_swift_string_constant(text: str, name: str) -> str:
  value_match = re.search(rf'static let {name}\s*=\s*"([^"]+)"', text)
  if not value_match:
    raise SystemExit(f"Swift model catalog missing {name}")
  return value_match.group(1)


def validate_swift_catalog_entries(entries: list[dict[str, object]]) -> None:
  for entry in entries:
    missing_keys = CATALOG_INVARIANT_KEYS.difference(entry.keys())
    if missing_keys:
      raise SystemExit(
        f"Swift model catalog entry missing keys: {', '.join(sorted(missing_keys))}"
      )

    model_id = entry["id"]
    if not str(entry["fileName"]).lower().endswith(".gguf"):
      raise SystemExit(f"Swift model catalog {model_id} must point to a .gguf artifact")
    if not str(entry["downloadURL"]).startswith("https://huggingface.co/"):
      raise SystemExit(f"Swift model catalog {model_id} must use a Hugging Face URL")
    if not re.fullmatch(r"[0-9a-f]{64}", str(entry["sha256"])):
      raise SystemExit(f"Swift model catalog {model_id} must use a lowercase SHA-256")
    if int(entry["sizeBytes"]) <= 0:
      raise SystemExit(f"Swift model catalog {model_id} must declare a positive size")
    if int(entry["contextSize"]) <= 0:
      raise SystemExit(f"Swift model catalog {model_id} must declare a positive context")
    if int(entry["modelContextSize"]) <= 0:
      raise SystemExit(f"Swift model catalog {model_id} must declare a positive model context")
    if int(entry["maxOutputTokens"]) <= 0:
      raise SystemExit(f"Swift model catalog {model_id} must declare a positive output cap")
    if int(entry["contextSize"]) > int(entry["modelContextSize"]):
      raise SystemExit(
        f"Swift model catalog {model_id} contextSize must not exceed modelContextSize"
      )
    if int(entry["maxOutputTokens"]) > int(entry["contextSize"]):
      raise SystemExit(
        f"Swift model catalog {model_id} maxOutputTokens must not exceed contextSize"
      )


if __name__ == "__main__":
  raise SystemExit(main())
