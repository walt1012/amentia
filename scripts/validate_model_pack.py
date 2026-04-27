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
  "display_name": ("displayName", str),
  "file_name": ("fileName", str),
  "homepage": ("homepage", str),
  "download_url": ("downloadURL", str),
  "sha256": ("sha256", str),
  "size_bytes": ("sizeBytes", int),
  "context_size": ("contextSize", int),
  "max_output_tokens": ("maxOutputTokens", int),
  "license": ("license", str),
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

  swift_catalog = load_default_swift_catalog_entry()
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


def load_default_swift_catalog_entry() -> dict[str, object]:
  if not SWIFT_CATALOG_PATH.is_file():
    raise SystemExit(f"Swift model catalog missing: {SWIFT_CATALOG_PATH}")

  text = SWIFT_CATALOG_PATH.read_text(encoding="utf-8")
  block_match = re.search(
    r"LocalModelCatalogItem\(\s*id:\s*defaultFirstUseModelID,(.*?)\n\s*\),",
    text,
    flags=re.DOTALL,
  )
  if not block_match:
    raise SystemExit("default Swift model catalog entry was not found")

  block = block_match.group(1)
  catalog: dict[str, object] = {}
  for _manifest_key, (swift_key, value_type) in CATALOG_FIELD_MAP.items():
    if value_type is int:
      value_match = re.search(rf"{swift_key}:\s*([0-9_]+)", block)
      if not value_match:
        raise SystemExit(f"default Swift model catalog missing {swift_key}")
      catalog[swift_key] = int(value_match.group(1).replace("_", ""))
    else:
      value_match = re.search(rf'{swift_key}:\s*"([^"]+)"', block)
      if not value_match:
        raise SystemExit(f"default Swift model catalog missing {swift_key}")
      catalog[swift_key] = value_match.group(1)

  return catalog


if __name__ == "__main__":
  raise SystemExit(main())
