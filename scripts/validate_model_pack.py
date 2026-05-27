#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
import subprocess
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = REPO_ROOT / "models" / "builtin" / "lfm2.5-350m" / "model-pack.json"
README_PATH = REPO_ROOT / "models" / "builtin" / "lfm2.5-350m" / "README.md"
SWIFT_SOURCE_ROOT = (
  REPO_ROOT
  / "apps"
  / "pith-macos"
  / "Sources"
  / "PithApp"
)
SWIFT_LOCAL_MODELS_DIR = SWIFT_SOURCE_ROOT / "LocalModels"
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
REMOTE_METADATA_TIMEOUT_SECONDS = 30
MAX_CURATED_CATALOG_ENTRIES = 5
REJECTED_CATALOG_MODEL_IDS = {
  "qwen2.5-0.5b",
  "qwen2.5-0.5b-instruct",
  "qwen3-0.6b",
  "smollm2-360m",
}


def main() -> int:
  args = parse_args()
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
  validate_curated_catalog_shape(swift_catalog_entries)
  validate_swift_catalog_entries(swift_catalog_entries)
  if args.remote:
    validate_remote_catalog_entries(swift_catalog_entries)
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
  if args.remote:
    print("remote model catalog metadata is valid")
  return 0


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(
    description="Validate local model manifests and Swift catalog metadata."
  )
  parser.add_argument(
    "--remote",
    action="store_true",
    help=(
      "Check Hugging Face HEAD/API metadata for size, checksum, and license. "
      "This is intended for release audits, not mandatory CI."
    ),
  )
  return parser.parse_args()


def load_swift_catalog_entries() -> list[dict[str, object]]:
  text = load_swift_local_model_sources()
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


def validate_curated_catalog_shape(entries: list[dict[str, object]]) -> None:
  if not entries:
    raise SystemExit("Swift model catalog must contain at least the default model")
  if len(entries) > MAX_CURATED_CATALOG_ENTRIES:
    raise SystemExit(
      f"Swift model catalog must stay curated: {len(entries)} > {MAX_CURATED_CATALOG_ENTRIES}"
    )

  model_ids = [str(entry["id"]) for entry in entries]
  duplicate_ids = sorted(
    model_id for model_id in set(model_ids)
    if model_ids.count(model_id) > 1
  )
  if duplicate_ids:
    raise SystemExit(
      f"Swift model catalog has duplicate ids: {', '.join(duplicate_ids)}"
    )
  if model_ids[0] != "lfm2.5-350m":
    raise SystemExit("Swift model catalog must keep lfm2.5-350m as the first-use default")

  rejected_ids = sorted(REJECTED_CATALOG_MODEL_IDS.intersection(model_ids))
  if rejected_ids:
    raise SystemExit(
      "Swift model catalog contains rejected stale entries: "
      + ", ".join(rejected_ids)
    )


def load_swift_local_model_sources() -> str:
  if not SWIFT_LOCAL_MODELS_DIR.is_dir():
    raise SystemExit(f"Swift local model source root missing: {SWIFT_LOCAL_MODELS_DIR}")

  matches = sorted(SWIFT_LOCAL_MODELS_DIR.glob("*.swift"))
  if not matches:
    raise SystemExit(f"Swift local model sources missing under: {SWIFT_LOCAL_MODELS_DIR}")

  return "\n".join(path.read_text(encoding="utf-8") for path in matches)


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


def validate_remote_catalog_entries(entries: list[dict[str, object]]) -> None:
  for entry in entries:
    model_id = str(entry["id"])
    headers = remote_download_headers(str(entry["downloadURL"]), model_id)
    validate_remote_download_metadata(entry, headers)
    validate_remote_license_metadata(entry)


def remote_download_headers(
  download_url: str,
  model_id: str,
) -> urllib.response.addinfourl:
  request = urllib.request.Request(
    download_url,
    method="HEAD",
    headers={"User-Agent": "pith-model-audit"},
  )
  opener = urllib.request.build_opener(NoRedirectHandler)
  try:
    return opener.open(request, timeout=REMOTE_METADATA_TIMEOUT_SECONDS).headers
  except urllib.error.HTTPError as error:
    if error.code in {301, 302, 303, 307, 308}:
      return error.headers
    raise SystemExit(
      f"remote model catalog {model_id} download metadata failed: HTTP {error.code}"
    ) from error
  except urllib.error.URLError as error:
    raise SystemExit(
      f"remote model catalog {model_id} download metadata failed: {error.reason}"
    ) from error


class NoRedirectHandler(urllib.request.HTTPRedirectHandler):
  def redirect_request(self, req, fp, code, msg, headers, newurl):
    return None


def validate_remote_download_metadata(
  entry: dict[str, object],
  headers: urllib.response.addinfourl,
) -> None:
  model_id = str(entry["id"])
  linked_size = headers.get("X-Linked-Size")
  linked_etag = headers.get("X-Linked-ETag")

  if not linked_size:
    raise SystemExit(f"remote model catalog {model_id} missing X-Linked-Size")
  if not linked_etag:
    raise SystemExit(f"remote model catalog {model_id} missing X-Linked-ETag")

  try:
    remote_size = int(linked_size)
  except ValueError as error:
    raise SystemExit(
      f"remote model catalog {model_id} X-Linked-Size is invalid: {linked_size}"
    ) from error
  if remote_size != int(entry["sizeBytes"]):
    raise SystemExit(
      f"remote model catalog {model_id} size mismatch: "
      f"{remote_size} != {entry['sizeBytes']}"
    )

  remote_sha256 = linked_etag.strip().strip('"').lower()
  if remote_sha256 != str(entry["sha256"]):
    raise SystemExit(
      f"remote model catalog {model_id} checksum mismatch: "
      f"{remote_sha256} != {entry['sha256']}"
    )


def validate_remote_license_metadata(entry: dict[str, object]) -> None:
  model_id = str(entry["id"])
  repo_id = hugging_face_repo_id(str(entry["homepage"]))
  api_url = f"https://huggingface.co/api/models/{repo_id}"
  data = remote_json(api_url, model_id)
  card_data = data.get("cardData") or {}
  tags = data.get("tags") or []
  catalog_license = str(entry["license"]).lower()
  remote_license_values = {
    str(card_data.get("license", "")).lower(),
    str(card_data.get("license_name", "")).lower(),
  }
  remote_license_values.update(
    tag.removeprefix("license:").lower()
    for tag in tags
    if isinstance(tag, str) and tag.startswith("license:")
  )

  if catalog_license not in remote_license_values:
    raise SystemExit(
      f"remote model catalog {model_id} license mismatch: "
      f"{catalog_license} not in {sorted(remote_license_values)}"
    )


def hugging_face_repo_id(homepage: str) -> str:
  parsed = urllib.parse.urlparse(homepage)
  path_parts = [part for part in parsed.path.split("/") if part]
  if parsed.netloc != "huggingface.co" or len(path_parts) < 2:
    raise SystemExit(f"unsupported Hugging Face homepage: {homepage}")
  return "/".join(path_parts[:2])


def remote_json(api_url: str, model_id: str) -> dict[str, object]:
  request = urllib.request.Request(
    api_url,
    headers={"User-Agent": "pith-model-audit"},
  )
  try:
    with urllib.request.urlopen(
      request,
      timeout=REMOTE_METADATA_TIMEOUT_SECONDS,
    ) as response:
      return json.loads(response.read().decode("utf-8"))
  except urllib.error.HTTPError as error:
    raise SystemExit(
      f"remote model catalog {model_id} API metadata failed: HTTP {error.code}"
    ) from error
  except urllib.error.URLError as error:
    raise SystemExit(
      f"remote model catalog {model_id} API metadata failed: {error.reason}"
    ) from error


if __name__ == "__main__":
  raise SystemExit(main())
