#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = REPO_ROOT / "models" / "builtin" / "lfm2.5-350m" / "model-pack.json"
README_PATH = REPO_ROOT / "models" / "builtin" / "lfm2.5-350m" / "README.md"
REQUIRED_KEYS = {
  "id",
  "display_name",
  "file_name",
  "context_size",
  "max_output_tokens",
  "backend",
}
FORBIDDEN_SUFFIXES = (".gguf", ".bin", ".safetensors")


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


if __name__ == "__main__":
  raise SystemExit(main())
