#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
import os
import selectors
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from macos_llama_backend import assert_portable_llama_backend


APP_PROCESS_NAME = "Pith"
RUNTIME_PROCESS_NAME = "pith-runtime-bin"
APP_SUPPORT_ENV_KEY = "PITH_APP_SUPPORT_DIR"
WEB_SEARCH_FIXTURE_NAME = "packaged-web-search-fixture.html"
WEB_SEARCH_SNAPSHOT_KIND = "searchResults"
WEB_SEARCH_SNAPSHOT_HASH_LENGTH = 16
NOTION_CONNECTOR_ID = "notion-connector::notion"
NOTION_PLUGIN_ID = "notion-connector"
NOTION_COMMAND_ID = "notion-connector::notion.prepare-page-draft"
NOTION_CREDENTIAL_LABEL = "Packaged Smoke Notion"
NOTION_CREDENTIAL_SECRET = "packaged-smoke-token"
DEFAULT_MODEL_ID = "lfm2.5-350m"
DEFAULT_MODEL_DISPLAY_NAME = "LFM2.5-350M Q4_K_M"
DEFAULT_MODEL_FILE_NAME = "LFM2.5-350M-Q4_K_M.gguf"
DEFAULT_MODEL_MANIFEST_RELATIVE_PATH = Path("models/builtin/lfm2.5-350m/model-pack.json")
DEFAULT_MODEL_DOWNLOAD_URL = (
  "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF/resolve/main/"
  "LFM2.5-350M-Q4_K_M.gguf"
)
DEFAULT_MODEL_SHA256 = "7e6f72643caafc9a68256686638c4d7916f2cec76d1df478d4c3ddcd95a6aed4"
DEFAULT_MODEL_SIZE_BYTES = 229312224
SMOKE_MODEL_ID = "packaged-smoke-local-model"
SMOKE_MODEL_FILE_NAME = "smoke-local-model.gguf"
PROHIBITED_MODEL_SUFFIXES = {".gguf", ".bin", ".safetensors"}
RUNTIME_REQUEST_TIMEOUT_SECONDS = 10.0
LLAMA_BACKEND_LAUNCH_TIMEOUT_SECONDS = 10.0
APP_STARTUP_TIMEOUT_SECONDS = 18.0
APP_STABILITY_SECONDS = 2.0
REQUIRED_BUNDLED_PLUGINS = {
  "notion-connector",
  "review-assistant",
  "shell-recorder",
  "web-search",
  "workspace-notes",
}
REQUIRED_DATABASE_TABLES = {
  "approvals",
  "memory_notes",
  "plugin_connector_credentials",
  "plugin_state",
  "schema_migrations",
  "threads",
  "workspace_state",
}
REQUIRED_SCHEMA_VERSION = 9


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(
    description="Smoke launch a packaged Pith.app on macOS."
  )
  parser.add_argument("app_path", type=Path, help="Path to the packaged Pith.app bundle.")
  parser.add_argument(
    "--duration",
    type=float,
    default=None,
    help="Deprecated alias for --stability-duration.",
  )
  parser.add_argument(
    "--startup-timeout",
    type=float,
    default=APP_STARTUP_TIMEOUT_SECONDS,
    help="Seconds to wait for the packaged app to launch its runtime.",
  )
  parser.add_argument(
    "--stability-duration",
    type=float,
    default=APP_STABILITY_SECONDS,
    help="Seconds the app and launched runtime must remain alive after startup.",
  )
  return parser.parse_args()


def require_file(path: Path) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Required packaged app file is missing: {path}")


def read_json_object(path: Path) -> dict:
  require_file(path)
  value = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(value, dict):
    raise RuntimeError(f"Expected JSON object in packaged file: {path}")
  return value


def process_ids(name: str) -> set[int]:
  result = subprocess.run(
    ["/usr/bin/pgrep", "-x", name],
    check=False,
    capture_output=True,
    text=True,
  )
  if result.returncode not in {0, 1}:
    raise RuntimeError(result.stderr.strip() or f"pgrep failed for {name}")
  return {
    int(line)
    for line in result.stdout.splitlines()
    if line.strip().isdigit()
  }


def terminate_processes(process_name: str, process_ids_to_stop: set[int]) -> None:
  for pid in process_ids_to_stop:
    try:
      os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
      pass
  deadline = time.monotonic() + 5
  while time.monotonic() < deadline:
    if process_ids_to_stop.isdisjoint(process_ids(process_name)):
      return
    time.sleep(0.2)
  for pid in process_ids_to_stop:
    try:
      os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
      pass


def validate_app_bundle(app_path: Path) -> None:
  require_file(app_path / "Contents" / "Info.plist")
  require_file(app_path / "Contents" / "MacOS" / "Pith")
  require_file(app_path / "Contents" / "MacOS" / "pith-runtime-bin")
  require_file(app_path / "Contents" / "Resources" / "tools" / "llama.cpp" / "llama-cli")
  require_file(app_path / "Contents" / "Resources" / "PithPackage.json")
  validate_packaged_model_metadata(app_path)
  assert_portable_llama_backend(app_path / "Contents" / "Resources" / "tools" / "llama.cpp")
  validate_packaged_llama_backend_launch(app_path)


def validate_packaged_model_metadata(app_path: Path) -> None:
  resources_path = bundled_resource_path(app_path)
  package_manifest_path = resources_path / "PithPackage.json"
  package_manifest = read_json_object(package_manifest_path)
  expected_package_values = {
    "defaultModelId": DEFAULT_MODEL_ID,
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
    "modelDelivery": "in-app-download",
    "modelWeightsBundled": False,
    "modelMetadataBundled": True,
  }
  for field, expected_value in expected_package_values.items():
    if package_manifest.get(field) != expected_value:
      raise RuntimeError(
        f"Packaged manifest field {field} must be {expected_value!r}."
      )

  model_manifest_path = resources_path / DEFAULT_MODEL_MANIFEST_RELATIVE_PATH
  validate_default_model_manifest(model_manifest_path)
  bundled_weight_files = sorted(
    path
    for path in resources_path.rglob("*")
    if path.is_file() and path.suffix.lower() in PROHIBITED_MODEL_SUFFIXES
  )
  if bundled_weight_files:
    raise RuntimeError(
      "Packaged app must not bundle model weight files: "
      f"{', '.join(str(path) for path in bundled_weight_files)}"
    )


def validate_default_model_manifest(manifest_path: Path) -> None:
  manifest = read_json_object(manifest_path)
  expected_values = {
    "id": DEFAULT_MODEL_ID,
    "display_name": DEFAULT_MODEL_DISPLAY_NAME,
    "file_name": DEFAULT_MODEL_FILE_NAME,
    "context_size": 4096,
    "model_context_size": 32768,
    "max_output_tokens": 160,
    "backend": "llama.cpp",
    "download_url": DEFAULT_MODEL_DOWNLOAD_URL,
    "sha256": DEFAULT_MODEL_SHA256,
    "size_bytes": DEFAULT_MODEL_SIZE_BYTES,
  }
  for field, expected_value in expected_values.items():
    if manifest.get(field) != expected_value:
      raise RuntimeError(
        f"Packaged default model field {field} must be {expected_value!r}: {manifest_path}"
      )

  homepage = manifest.get("homepage")
  if not isinstance(homepage, str) or not homepage.startswith("https://"):
    raise RuntimeError("Packaged default model homepage must be HTTPS.")
  license_value = manifest.get("license")
  if not isinstance(license_value, str) or not license_value:
    raise RuntimeError("Packaged default model license must be present.")
  if (manifest_path.parent / DEFAULT_MODEL_FILE_NAME).exists():
    raise RuntimeError("Packaged default model weights must not be bundled.")


def validate_packaged_llama_backend_launch(app_path: Path) -> None:
  backend = app_path / "Contents" / "Resources" / "tools" / "llama.cpp" / "llama-cli"
  try:
    completed = subprocess.run(
      [str(backend), "--help"],
      stdout=subprocess.PIPE,
      stderr=subprocess.STDOUT,
      text=True,
      timeout=LLAMA_BACKEND_LAUNCH_TIMEOUT_SECONDS,
    )
  except subprocess.TimeoutExpired as error:
    raise RuntimeError("Packaged llama.cpp backend did not respond to --help.") from error
  if completed.returncode != 0:
    raise RuntimeError(
      "Packaged llama.cpp backend failed to launch. "
      f"Exit {completed.returncode}. Output: {completed.stdout[-1000:]}"
    )


def packaged_runtime_path(app_path: Path) -> Path:
  return app_path / "Contents" / "MacOS" / RUNTIME_PROCESS_NAME


def bundled_resource_path(app_path: Path) -> Path:
  return app_path / "Contents" / "Resources"


def launch_app_process(app_path: Path, support_dir: Path) -> subprocess.Popen[str]:
  environment = os.environ.copy()
  environment[APP_SUPPORT_ENV_KEY] = str(support_dir)
  return subprocess.Popen(
    [str(app_path / "Contents" / "MacOS" / APP_PROCESS_NAME)],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    env=environment,
  )


def launch_runtime_process(
  app_path: Path,
  support_dir: Path,
  extra_environment: dict[str, str] | None = None,
) -> subprocess.Popen[str]:
  resources_path = bundled_resource_path(app_path)
  environment = os.environ.copy()
  environment["PITH_DATA_DIR"] = str(support_dir / "storage")
  environment["PITH_LOCAL_PLUGIN_DIR"] = str(support_dir / "plugins")
  environment["PITH_MODEL_PACK_ROOT"] = str(resources_path)
  environment["PITH_PLUGIN_DIR"] = str(resources_path / "plugins")
  environment["PITH_LLAMACPP_PATH"] = str(
    resources_path / "tools" / "llama.cpp" / "llama-cli"
  )
  if extra_environment:
    environment.update(extra_environment)
  return subprocess.Popen(
    [str(packaged_runtime_path(app_path))],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    env=environment,
  )


def terminate_process(process: subprocess.Popen[str]) -> None:
  if process.poll() is not None:
    return
  process.terminate()
  try:
    process.wait(timeout=5)
  except subprocess.TimeoutExpired:
    process.kill()
    process.wait(timeout=5)


def kill_process(process: subprocess.Popen[str]) -> None:
  if process.poll() is not None:
    return
  process.kill()
  process.wait(timeout=5)


def terminate_process_with_output(process: subprocess.Popen[str]) -> tuple[str, str]:
  terminate_process(process)
  stdout, stderr = process.communicate(timeout=1)
  return stdout or "", stderr or ""


def validate_isolated_support_dir(support_dir: Path) -> None:
  required_directories = [
    support_dir,
    support_dir / "storage",
    support_dir / "storage" / "models",
    support_dir / "plugins",
    support_dir / "model-downloads",
  ]
  for directory in required_directories:
    if not directory.is_dir():
      raise RuntimeError(f"Packaged app did not prepare app-owned directory: {directory}")
    if directory.is_symlink():
      raise RuntimeError(f"Packaged app support directory must not be a symlink: {directory}")

  model_weights = sorted(
    path
    for path in support_dir.rglob("*")
    if path.is_file() and path.suffix.lower() == ".gguf"
  )
  if model_weights:
    raise RuntimeError(
      "Fresh packaged app launch unexpectedly created model weight files: "
      f"{', '.join(str(path) for path in model_weights)}"
    )

  storage_dir = support_dir / "storage"
  validate_runtime_database(storage_dir / "pith.db")


def validate_runtime_database(database_path: Path) -> None:
  require_file(database_path)
  with sqlite3.connect(f"file:{database_path}?mode=ro", uri=True) as connection:
    schema_version = connection.execute("PRAGMA user_version").fetchone()[0]
    if schema_version != REQUIRED_SCHEMA_VERSION:
      raise RuntimeError(
        f"Packaged runtime database schema is {schema_version}, "
        f"expected {REQUIRED_SCHEMA_VERSION}: {database_path}"
      )
    rows = connection.execute(
      "SELECT name FROM sqlite_master WHERE type = 'table'"
    ).fetchall()
  tables = {row[0] for row in rows}
  missing = sorted(REQUIRED_DATABASE_TABLES - tables)
  if missing:
    raise RuntimeError(
      "Packaged runtime database is missing tables "
      f"{', '.join(missing)}: {database_path}"
    )


def send_runtime_request(
  process: subprocess.Popen[str],
  request_id: int,
  method: str,
  params: dict | None = None,
) -> dict:
  if process.stdin is None or process.stdout is None:
    raise RuntimeError("Packaged runtime pipes are unavailable.")

  payload = {
    "id": request_id,
    "method": method,
  }
  if params is not None:
    payload["params"] = params

  process.stdin.write(json.dumps(payload) + "\n")
  process.stdin.flush()

  while True:
    line = read_runtime_line(process, method)
    if not line:
      raise RuntimeError(
        f"Packaged runtime exited before responding to {method}."
        f"{runtime_stderr_detail(process)}"
      )
    message = json.loads(line)
    if message.get("id") == request_id:
      if "error" in message:
        raise RuntimeError(f"Packaged runtime {method} failed: {message['error']}")
      return message


def read_runtime_line(process: subprocess.Popen[str], method: str) -> str:
  if process.stdout is None:
    raise RuntimeError("Packaged runtime stdout is unavailable.")

  with selectors.DefaultSelector() as selector:
    selector.register(process.stdout, selectors.EVENT_READ)
    events = selector.select(timeout=RUNTIME_REQUEST_TIMEOUT_SECONDS)
  if not events:
    raise RuntimeError(
      f"Packaged runtime did not respond to {method} within "
      f"{RUNTIME_REQUEST_TIMEOUT_SECONDS:.0f}s.{runtime_stderr_detail(process)}"
    )

  return process.stdout.readline()


def runtime_stderr_detail(process: subprocess.Popen[str]) -> str:
  if process.poll() is None or process.stderr is None:
    return ""
  stderr = process.stderr.read().strip()
  if not stderr:
    return ""
  return f"\nRuntime stderr:\n{stderr[-2000:]}"


def timeline_item_summary(items: list[dict]) -> str:
  summaries = []
  interesting_keys = {
    "commandId",
    "executionKind",
    "mcpProtocolStatus",
    "pluginCommandStatus",
    "pluginRunnerFailureKind",
    "pluginRunnerRecoveryHint",
    "pluginRunnerStderrPreview",
    "pluginRunnerStdoutPreview",
    "pageFetchPerformed",
    "sourceSnapshotAvailable",
    "sourceSnapshotHash",
    "sourceSnapshotKind",
    "sourceSnapshotResultCount",
    "sourceTitles",
    "sourceUrls",
    "webSearchSourceMode",
  }
  for item in items:
    attributes = item.get("attributes", {})
    interesting_attributes = {
      key: value
      for key, value in attributes.items()
      if key in interesting_keys
    }
    summaries.append(
      {
        "kind": item.get("kind"),
        "title": item.get("title"),
        "content": item.get("content", "")[:240],
        "attributes": interesting_attributes,
      }
    )
  return json.dumps(summaries, sort_keys=True)


def validate_runtime_readiness(
  readiness: dict,
  expected_statuses: dict[str, str | set[str]] | None = None,
  expected_status: str = "setup_required",
  workspace_open: bool = False,
) -> None:
  result = readiness["result"]
  if result["status"] != expected_status:
    raise RuntimeError(
      f"Packaged runtime readiness was {result['status']}, expected {expected_status}."
    )
  checks = {check["id"]: check for check in result["checks"]}
  expected_statuses = expected_statuses or {
    "localModel": "setup_required",
    "workspace": "setup_required",
    "thread": "setup_required",
    "firstRequest": "waiting",
    "context": "ready",
    "executionControls": "ready",
    "plugins": "ready",
    "boundedRuntime": "ready",
  }
  for check_id, expected_status in expected_statuses.items():
    validate_readiness_check_status(checks, check_id, expected_status)
  validate_tooling_readiness(result, checks, workspace_open=workspace_open)


def validate_readiness_check_status(
  checks: dict[str, dict],
  check_id: str,
  expected_status: str | set[str],
) -> None:
  actual_status = checks.get(check_id, {}).get("status")
  allowed_statuses = {expected_status} if isinstance(expected_status, str) else expected_status
  if actual_status not in allowed_statuses:
    expected = ", ".join(sorted(allowed_statuses))
    raise RuntimeError(
      f"Packaged runtime readiness check {check_id} was {actual_status}, "
      f"expected one of {expected}."
    )


def validate_tooling_readiness(
  result: dict,
  checks: dict[str, dict],
  workspace_open: bool,
) -> None:
  validate_readiness_check_status(checks, "executionControls", "ready")
  validate_readiness_check_status(checks, "webSearch", "ready")
  validate_readiness_check_status(
    checks,
    "nativeSandbox",
    {"ready", "limited"} if workspace_open else {"limited", "setup_required"},
  )

  metrics = result["metrics"]
  expected_metrics = {
    "webSearchTimeoutSeconds": "20",
    "webSearchProvider": "DuckDuckGo Lite",
    "webSearchAvailable": "true",
    "webSearchPermissionGranted": "true",
    "sandboxMode": "workspaceReadWrite",
    "sandboxNetworkAllowed": "false",
  }
  for key, expected_value in expected_metrics.items():
    actual_value = metrics.get(key)
    if actual_value != expected_value:
      raise RuntimeError(
        f"Packaged runtime readiness metric {key} was {actual_value}, "
        f"expected {expected_value}."
      )

  if "Web Search" not in metrics.get("webSearchPermissionSources", ""):
    raise RuntimeError("Packaged runtime readiness did not attribute Web Search permission.")
  if metrics.get("webSearchClient") not in {"curl", "fixture"}:
    raise RuntimeError(
      "Packaged runtime readiness reported unexpected web search client: "
      f"{metrics.get('webSearchClient')}"
    )
  if workspace_open:
    if metrics.get("sandboxBackend") not in {"macosSeatbelt", "processOnly"}:
      raise RuntimeError(
        "Packaged runtime readiness reported unexpected sandbox backend after "
        f"workspace open. Backend: {metrics.get('sandboxBackend')}"
      )
    native_sandbox_check = checks.get("nativeSandbox")
    native_sandbox_status = (
      native_sandbox_check.get("status")
      if isinstance(native_sandbox_check, dict)
      else None
    )
    if (
      metrics.get("sandboxBackend") == "macosSeatbelt"
      and metrics.get("sandboxActive") != "true"
    ):
      raise RuntimeError(
        "Packaged runtime readiness must report an active native sandbox after "
        f"workspace open. Active: {metrics.get('sandboxActive')}"
      )
    if (
      metrics.get("sandboxBackend") == "macosSeatbelt"
      and native_sandbox_status != "ready"
    ):
      raise RuntimeError(
        "Packaged runtime readiness must mark native sandbox ready when the "
        f"macosSeatbelt backend is active. Status: {native_sandbox_status}"
      )
    if (
      metrics.get("sandboxBackend") == "processOnly"
      and metrics.get("sandboxActive") != "false"
    ):
      raise RuntimeError(
        "Packaged runtime readiness must report inactive native sandbox when "
        f"falling back to process-only. Active: {metrics.get('sandboxActive')}"
      )
    if (
      metrics.get("sandboxBackend") == "processOnly"
      and native_sandbox_status != "limited"
    ):
      raise RuntimeError(
        "Packaged runtime readiness must mark native sandbox limited when "
        f"falling back to process-only. Status: {native_sandbox_status}"
      )
  else:
    if metrics.get("sandboxBackend") not in {"macosSeatbelt", "processOnly"}:
      raise RuntimeError(
        "Packaged runtime readiness reported unexpected sandbox backend: "
        f"{metrics.get('sandboxBackend')}"
      )
    if metrics.get("sandboxActive") not in {"true", "false"}:
      raise RuntimeError(
        "Packaged runtime readiness reported unexpected sandbox active state: "
        f"{metrics.get('sandboxActive')}"
      )


def validate_packaged_runtime_workspace_bootstrap(
  process: subprocess.Popen[str],
  support_dir: Path,
  local_model_status: str = "setup_required",
  expected_status: str = "setup_required",
) -> None:
  workspace_dir = support_dir / "workspace"
  workspace_dir.mkdir()
  (workspace_dir / "README.md").write_text(
    "# Packaged Runtime Smoke\n\nWorkspace bootstrap check.\n",
    encoding="utf-8",
  )

  workspace_open = send_runtime_request(
    process,
    5,
    "workspace/open",
    {
      "path": str(workspace_dir),
    },
  )
  workspace = workspace_open["result"]["workspace"]
  if workspace["displayName"] != "workspace":
    raise RuntimeError(
      f"Packaged runtime opened workspace as {workspace['displayName']}."
    )

  thread_start = send_runtime_request(
    process,
    6,
    "thread/start",
    {
      "title": "Packaged Runtime Smoke",
    },
  )
  if thread_start["result"]["thread"]["id"] != "thread-1":
    raise RuntimeError("Packaged runtime did not create the first thread.")

  thread_readiness = send_runtime_request(process, 7, "runtime/readiness")
  validate_runtime_readiness(
    thread_readiness,
    {
      "localModel": local_model_status,
      "workspace": "ready",
      "thread": "ready",
      "firstRequest": "ready_to_send",
      "context": "ready",
      "executionControls": "ready",
      "plugins": "ready",
      "boundedRuntime": "ready",
    },
    expected_status=expected_status,
    workspace_open=True,
  )

  thread_list = send_runtime_request(process, 8, "thread/list")
  threads = thread_list["result"]["threads"]
  if len(threads) != 1 or threads[0]["title"] != "Packaged Runtime Smoke":
    raise RuntimeError("Packaged runtime did not persist the smoke thread.")

  validate_packaged_runtime_workspace_search(process)


def validate_packaged_runtime_workspace_search(process: subprocess.Popen[str]) -> None:
  search = send_runtime_request(
    process,
    9,
    "workspace/search",
    {
      "query": "bootstrap",
      "maxResults": 5,
      "clientRequestId": "packaged-smoke-search",
    },
  )
  result = search["result"]
  if result["query"] != "bootstrap":
    raise RuntimeError(
      f"Packaged runtime workspace search query was {result['query']}."
    )

  matches = result["matches"]
  if not any(
    match["relativePath"] == "README.md"
    and match["lineNumber"] == 3
    and "Workspace bootstrap check." in match["line"]
    for match in matches
  ):
    raise RuntimeError(
      "Packaged runtime workspace search did not find the smoke README."
    )


def write_smoke_model_pack(support_dir: Path) -> tuple[Path, Path]:
  model_dir = support_dir / "storage" / "models" / "builtin" / SMOKE_MODEL_ID
  model_dir.mkdir(parents=True)
  model_path = model_dir / SMOKE_MODEL_FILE_NAME
  model_bytes = b"GGUFpackaged smoke local model fixture\n"
  model_path.write_bytes(model_bytes)
  manifest = {
    "id": SMOKE_MODEL_ID,
    "display_name": "Packaged Smoke Local Model",
    "file_name": model_path.name,
    "context_size": 512,
    "model_context_size": 512,
    "max_output_tokens": 32,
    "backend": "llama.cpp",
    "license": "test-fixture",
    "homepage": "https://example.com/pith-smoke-model",
    "download_url": "https://example.com/pith-smoke-model.gguf",
    "sha256": hashlib.sha256(model_bytes).hexdigest(),
    "size_bytes": len(model_bytes),
  }
  manifest_path = model_dir / "model-pack.json"
  manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
  validate_smoke_model_pack(manifest_path, model_path, support_dir)
  return manifest_path, model_path


def validate_smoke_model_pack(manifest_path: Path, model_path: Path, support_dir: Path) -> None:
  expected_model_root = (support_dir / "storage" / "models").resolve()
  if expected_model_root not in model_path.resolve().parents:
    raise RuntimeError("Packaged first-use smoke model must live under app-owned storage.")
  if manifest_path.parent != model_path.parent:
    raise RuntimeError("Packaged first-use smoke manifest must live next to the model file.")
  if model_path.read_bytes()[:4] != b"GGUF":
    raise RuntimeError("Packaged first-use smoke model is missing the GGUF header.")

  manifest = read_json_object(manifest_path)
  expected_values = {
    "id": SMOKE_MODEL_ID,
    "file_name": model_path.name,
    "sha256": sha256_hex(model_path),
    "size_bytes": model_path.stat().st_size,
  }
  for field, expected_value in expected_values.items():
    if manifest.get(field) != expected_value:
      raise RuntimeError(
        f"Packaged first-use smoke model field {field} must be {expected_value!r}."
      )


def sha256_hex(path: Path) -> str:
  return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_ready_smoke_model_health(
  model_health: dict,
  manifest_path: Path,
  model_path: Path,
) -> None:
  result = model_health["result"]
  if result["status"] != "ready":
    raise RuntimeError(
      "Packaged first cowork smoke did not resolve a ready local model: "
      f"{result}"
    )
  if Path(result["manifestPath"]).resolve() != manifest_path.resolve():
    raise RuntimeError("Packaged first cowork smoke used the wrong model manifest.")
  if Path(result["modelPath"]).resolve() != model_path.resolve():
    raise RuntimeError("Packaged first cowork smoke used the wrong model file.")

  metrics = result["metrics"]
  expected_metrics = {
    "fileName": SMOKE_MODEL_FILE_NAME,
    "sha256": sha256_hex(model_path),
    "sizeBytes": str(model_path.stat().st_size),
    "readiness": "ready",
    "packReady": "true",
    "modelPresent": "true",
    "manifestPresent": "true",
  }
  for key, expected_value in expected_metrics.items():
    actual_value = metrics.get(key)
    if actual_value != expected_value:
      raise RuntimeError(
        f"Packaged first-use model metric {key} was {actual_value}, "
        f"expected {expected_value}."
      )


def write_deterministic_llama_backend_fixture(support_dir: Path) -> Path:
  backend_path = support_dir / "deterministic-llama-cli"
  backend_path.write_text(
    "#!/bin/sh\n"
    "printf '%s\\n' 'Packaged smoke local response.'\n",
    encoding="utf-8",
  )
  backend_path.chmod(0o755)
  return backend_path


def write_web_search_fixture(support_dir: Path) -> Path:
  fixture_path = support_dir / WEB_SEARCH_FIXTURE_NAME
  fixture_path.write_text(
    """
      <a rel="nofollow" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpith-packaged-smoke&amp;rut=abc" class='result-link'>Pith packaged web search fixture</a>
      <td class='result-snippet'>Deterministic packaged web search result.</td>
    """,
    encoding="utf-8",
  )
  return fixture_path


def validate_packaged_web_search_turn(process: subprocess.Popen[str]) -> None:
  web_turn = send_runtime_request(
    process,
    24,
    "turn/start",
    {
      "threadId": "thread-1",
      "message": "Search the web for Pith packaged smoke fixture",
    },
  )
  items = web_turn["result"]["items"]
  if not any(item["kind"] == "toolStart" and item["title"] == "web_search" for item in items):
    raise RuntimeError("Packaged first-run smoke did not start web_search.")

  result_item = next(
    (
      item
      for item in items
      if item["kind"] == "toolResult" and item["title"] == "web_search result"
    ),
    None,
  )
  if result_item is None:
    raise RuntimeError("Packaged first-run smoke did not produce a web_search result.")
  if "Pith packaged web search fixture" not in result_item["content"]:
    raise RuntimeError(
      "Packaged first-run smoke did not use the web search fixture result. "
      f"Result content: {result_item['content'][:500]}"
    )
  if not any(item["kind"] == "assistantMessage" for item in items):
    raise RuntimeError("Packaged web search turn did not produce an assistant item.")
  validate_packaged_web_search_snapshot(items)


def validate_packaged_web_search_snapshot(items: list[dict]) -> None:
  assistant_item = next(
    (
      item
      for item in items
      if item["kind"] == "assistantMessage"
      and item.get("attributes", {}).get("handoffKind") == "webSearchSources"
    ),
    None,
  )
  if assistant_item is None:
    raise RuntimeError(
      "Packaged web search turn did not produce a source handoff. "
      f"Items: {timeline_item_summary(items)}"
    )

  attributes = assistant_item.get("attributes", {})
  expected_attributes = {
    "webSearchSourceMode": "searchResultAttribution",
    "pageFetchPerformed": "false",
    "sourceSnapshotAvailable": "true",
    "sourceSnapshotKind": WEB_SEARCH_SNAPSHOT_KIND,
    "sourceSnapshotResultCount": "1",
  }
  for key, expected_value in expected_attributes.items():
    actual_value = attributes.get(key)
    if actual_value != expected_value:
      raise RuntimeError(
        f"Packaged web search source attribute {key} was {actual_value!r}, "
        f"expected {expected_value!r}. Items: {timeline_item_summary(items)}"
      )

  source_snapshot = attributes.get("sourceSnapshot", "")
  required_snapshot_fragments = [
    "Pith packaged web search fixture",
    "https://example.com/pith-packaged-smoke",
    "Deterministic packaged web search result.",
    "fixture",
  ]
  for fragment in required_snapshot_fragments:
    if fragment not in source_snapshot:
      raise RuntimeError(
        "Packaged web search source snapshot missed expected evidence. "
        f"Missing: {fragment!r}. Snapshot: {source_snapshot[:500]}"
      )

  source_hash = attributes.get("sourceSnapshotHash", "")
  if len(source_hash) != WEB_SEARCH_SNAPSHOT_HASH_LENGTH:
    raise RuntimeError(
      "Packaged web search source snapshot hash had unexpected length: "
      f"{source_hash!r}"
    )
  if attributes.get("sourceUrls") != "https://example.com/pith-packaged-smoke":
    raise RuntimeError(
      "Packaged web search source URL was not preserved in the handoff. "
      f"Attributes: {attributes}"
    )
  if attributes.get("sourceTitles") != "Pith packaged web search fixture":
    raise RuntimeError(
      "Packaged web search source title was not preserved in the handoff. "
      f"Attributes: {attributes}"
    )


def validate_packaged_mcp_plugin_command(process: subprocess.Popen[str]) -> None:
  enable = send_runtime_request(
    process,
    29,
    "plugin/setEnabled",
    {
      "pluginId": NOTION_PLUGIN_ID,
      "enabled": True,
    },
  )
  if enable["result"]["plugin"]["id"] != NOTION_PLUGIN_ID:
    raise RuntimeError("Packaged plugin smoke enabled the wrong plugin.")
  if enable["result"]["plugin"]["enabled"] is not True:
    raise RuntimeError("Packaged plugin smoke did not enable the Notion connector.")

  authorize = send_runtime_request(
    process,
    30,
    "plugin/connectorAuthorize",
    {
      "connectorId": NOTION_CONNECTOR_ID,
      "credentialLabel": NOTION_CREDENTIAL_LABEL,
      "credentialSecret": NOTION_CREDENTIAL_SECRET,
    },
  )
  if NOTION_CREDENTIAL_SECRET in json.dumps(authorize):
    raise RuntimeError("Packaged plugin smoke leaked the connector credential secret.")
  connector = authorize["result"]["connector"]
  if connector["connectorId"] != NOTION_CONNECTOR_ID:
    raise RuntimeError("Packaged plugin smoke authorized the wrong connector.")
  if connector["status"] != "ready" or connector["authStatus"] != "authorized":
    raise RuntimeError(
      "Packaged plugin smoke connector did not become ready after authorization."
    )

  command = send_runtime_request(
    process,
    31,
    "plugin/commandRun",
    {
      "threadId": "thread-1",
      "commandId": NOTION_COMMAND_ID,
      "input": "Prepare a packaged smoke page draft from this workspace.",
    },
  )
  pending_approvals = command["result"]["pendingApprovals"]
  if len(pending_approvals) != 1:
    raise RuntimeError(
      "Packaged MCP plugin command should request one connector-backed approval."
    )
  approval = pending_approvals[0]
  if approval["action"] != "run_plugin_command":
    raise RuntimeError("Packaged MCP plugin command requested the wrong approval.")
  if approval["relativePath"] != f"plugin:{NOTION_PLUGIN_ID}":
    raise RuntimeError("Packaged MCP plugin approval is not scoped to the plugin.")
  if not any(
    item["kind"] == "approvalRequested"
    and item.get("attributes", {}).get("commandId") == NOTION_COMMAND_ID
    for item in command["result"]["items"]
  ):
    raise RuntimeError("Packaged MCP plugin approval did not expose the command id.")

  approved = send_runtime_request(
    process,
    32,
    "approval/respond",
    {
      "approvalId": approval["id"],
      "decision": "approved",
    },
  )
  items = approved["result"]["items"]
  if not any(
    item["kind"] == "pluginResult"
    and item["title"] == "Notion Page Draft"
    and "Prepared a credential-scoped local Notion page draft" in item["content"]
    and item.get("attributes", {}).get("targetService") == "notion"
    and item.get("attributes", {}).get("draftMode") == "localDraft"
    and item.get("attributes", {}).get("remoteWrite") == "false"
    for item in items
  ):
    raise RuntimeError(
      "Packaged MCP plugin command did not return the local Notion page draft. "
      f"Items: {timeline_item_summary(items)}"
    )
  if not any(
    item["kind"] == "system"
    and item["title"] == "Plugin Memory Note Saved"
    and item.get("attributes", {}).get("memoryNoteTitle") == "Notion Draft Prepared"
    for item in items
  ):
    raise RuntimeError(
      "Packaged MCP plugin command did not capture runner memory. "
      f"Items: {timeline_item_summary(items)}"
    )

  memory_list = send_runtime_request(process, 33, "memory/list")
  if not any(
    note["title"] == "Notion Draft Prepared"
    and note["source"] == "plugin.notion-connector"
    for note in memory_list["result"]["notes"]
  ):
    raise RuntimeError("Packaged MCP plugin command memory was not persisted.")


def validate_packaged_workspace_write_approval(
  process: subprocess.Popen[str],
  support_dir: Path,
) -> None:
  validate_packaged_workspace_write_denial(process, support_dir)
  approval = request_packaged_workspace_write(
    process,
    27,
    "docs/packaged-output.txt",
    "Created from packaged approval flow",
  )

  approved = send_runtime_request(
    process,
    28,
    "approval/respond",
    {
      "approvalId": approval["id"],
      "decision": "approved",
    },
  )
  if not any(
    item["kind"] == "toolResult"
    and item["title"] == "write_file result"
    and item.get("attributes", {}).get("relativePath") == "docs/packaged-output.txt"
    for item in approved["result"]["items"]
  ):
    raise RuntimeError(
      "Packaged workspace write approval did not execute write_file. "
      f"Items: {timeline_item_summary(approved['result']['items'])}"
    )

  output_path = support_dir / "workspace" / "docs" / "packaged-output.txt"
  if output_path.read_text(encoding="utf-8") != "Created from packaged approval flow":
    raise RuntimeError("Packaged workspace write approval wrote unexpected content.")


def validate_packaged_workspace_write_denial(
  process: subprocess.Popen[str],
  support_dir: Path,
) -> None:
  approval = request_packaged_workspace_write(
    process,
    25,
    "docs/packaged-denied.txt",
    "Denied packaged approval flow",
  )

  denied = send_runtime_request(
    process,
    26,
    "approval/respond",
    {
      "approvalId": approval["id"],
      "decision": "denied",
    },
  )
  if not any(
    item["kind"] == "approvalResolved"
    and item["title"] == "Approval Denied"
    and item.get("attributes", {}).get("decision") == "denied"
    and item.get("attributes", {}).get("relativePath") == "docs/packaged-denied.txt"
    for item in denied["result"]["items"]
  ):
    raise RuntimeError(
      "Packaged workspace write denial did not resolve the approval as denied. "
      f"Items: {timeline_item_summary(denied['result']['items'])}"
    )
  if any(item["title"] == "write_file result" for item in denied["result"]["items"]):
    raise RuntimeError("Packaged workspace write denial still executed write_file.")

  denied_path = support_dir / "workspace" / "docs" / "packaged-denied.txt"
  if denied_path.exists():
    raise RuntimeError("Packaged workspace write denial wrote a file.")


def request_packaged_workspace_write(
  process: subprocess.Popen[str],
  request_id: int,
  relative_path: str,
  content: str,
) -> dict:
  write_turn = send_runtime_request(
    process,
    request_id,
    "turn/start",
    {
      "threadId": "thread-1",
      "message": f"Write {relative_path}: {content}",
    },
  )
  pending_approvals = write_turn["result"]["pendingApprovals"]
  if len(pending_approvals) != 1:
    raise RuntimeError(
      "Packaged workspace write should request one approval. "
      f"Items: {timeline_item_summary(write_turn['result']['items'])}"
    )

  approval = pending_approvals[0]
  if approval["action"] != "write_file":
    raise RuntimeError("Packaged workspace write requested the wrong approval action.")
  if approval["relativePath"] != relative_path:
    raise RuntimeError("Packaged workspace write approval had the wrong path.")
  if not any(
    item["kind"] == "diffArtifact"
    and f"+++ b/{relative_path}" in item["content"]
    for item in write_turn["result"]["items"]
  ):
    raise RuntimeError(
      "Packaged workspace write did not expose a diff before approval. "
      f"Items: {timeline_item_summary(write_turn['result']['items'])}"
    )
  return approval


def validate_packaged_first_cowork_request(app_path: Path) -> None:
  with tempfile.TemporaryDirectory(prefix="pith-packaged-first-cowork-") as support_root:
    support_dir = Path(support_root)
    manifest_path, model_path = write_smoke_model_pack(support_dir)
    backend_path = write_deterministic_llama_backend_fixture(support_dir)
    web_search_fixture_path = write_web_search_fixture(support_dir)
    runtime_environment = {
      "PITH_MODEL_PACK_MANIFEST": str(manifest_path),
      "PITH_MODEL_PATH": str(model_path),
      "PITH_LFM_MODEL_PATH": str(model_path),
      "PITH_LLAMACPP_PATH": str(backend_path),
      "PITH_ENABLE_WEB_SEARCH_FIXTURE": "1",
      "PITH_WEB_SEARCH_FIXTURE_PATH": str(web_search_fixture_path),
    }
    process = launch_runtime_process(
      app_path,
      support_dir,
      runtime_environment,
    )
    try:
      initialize = send_runtime_request(
        process,
        20,
        "initialize",
        {
          "clientInfo": {
            "name": "packaged-first-cowork-smoke",
            "version": "0.1.0",
          }
        },
      )
      if initialize["result"]["serverInfo"]["name"] != "pith-runtime":
        raise RuntimeError("Packaged first cowork initialize returned the wrong server name.")

      model_health = send_runtime_request(process, 21, "model/health")
      validate_ready_smoke_model_health(model_health, manifest_path, model_path)

      validate_packaged_runtime_workspace_bootstrap(
        process,
        support_dir,
        local_model_status="ready",
        expected_status="ready",
      )
      turn = send_runtime_request(
        process,
        22,
        "turn/start",
        {
          "threadId": "thread-1",
          "message": "Read README.md",
        },
      )
      items = turn["result"]["items"]
      if not any(item["kind"] == "assistantMessage" for item in items):
        raise RuntimeError("Packaged first cowork request did not produce an assistant item.")
      first_request_readiness = send_runtime_request(process, 23, "runtime/readiness")
      checks = {
        check["id"]: check["status"]
        for check in first_request_readiness["result"]["checks"]
      }
      if checks.get("firstRequest") != "ready":
        raise RuntimeError(
          "Packaged first cowork request did not mark firstRequest ready: "
          f"{checks.get('firstRequest')}"
      )
      validate_packaged_web_search_turn(process)
      validate_packaged_workspace_write_approval(process, support_dir)
      validate_packaged_mcp_plugin_command(process)
      process = validate_packaged_runtime_recovery(
        process,
        app_path,
        support_dir,
        runtime_environment,
        manifest_path,
        model_path,
      )
      print(
        "Packaged first cowork smoke passed with deterministic local model "
        f"under {support_dir}"
      )
    finally:
      terminate_process(process)


def validate_packaged_runtime_recovery(
  process: subprocess.Popen[str],
  app_path: Path,
  support_dir: Path,
  runtime_environment: dict[str, str],
  manifest_path: Path,
  model_path: Path,
) -> subprocess.Popen[str]:
  kill_process(process)
  recovered_process = launch_runtime_process(app_path, support_dir, runtime_environment)
  try:
    initialize = send_runtime_request(
      recovered_process,
      34,
      "initialize",
      {
        "clientInfo": {
          "name": "packaged-runtime-recovery-smoke",
          "version": "0.1.0",
        }
      },
    )
    if initialize["result"]["serverInfo"]["name"] != "pith-runtime":
      raise RuntimeError("Packaged runtime recovery returned the wrong server name.")

    model_health = send_runtime_request(recovered_process, 35, "model/health")
    validate_ready_smoke_model_health(model_health, manifest_path, model_path)

    workspace_current = send_runtime_request(recovered_process, 36, "workspace/current")
    workspace = workspace_current["result"]["workspace"]
    expected_workspace = (support_dir / "workspace").resolve()
    actual_workspace = Path(workspace["rootPath"]).resolve()
    if actual_workspace != expected_workspace:
      raise RuntimeError(
        "Packaged runtime recovery restored the wrong workspace: "
        f"{workspace['rootPath']}"
      )

    recovered_thread = send_runtime_request(
      recovered_process,
      37,
      "thread/read",
      {
        "threadId": "thread-1",
      },
    )
    if recovered_thread["result"]["thread"]["title"] != "Packaged Runtime Smoke":
      raise RuntimeError("Packaged runtime recovery did not restore the smoke thread.")

    recovered_output = support_dir / "workspace" / "docs" / "packaged-output.txt"
    if recovered_output.read_text(encoding="utf-8") != "Created from packaged approval flow":
      raise RuntimeError("Packaged runtime recovery lost the approved workspace write.")

    recovered_readiness = send_runtime_request(recovered_process, 38, "runtime/readiness")
    validate_runtime_readiness(
      recovered_readiness,
      {
        "localModel": "ready",
        "workspace": "ready",
        "thread": "ready",
        "firstRequest": "ready",
        "context": "ready",
        "executionControls": "ready",
        "plugins": "ready",
        "boundedRuntime": "ready",
      },
      expected_status="ready",
      workspace_open=True,
    )
    return recovered_process
  except Exception:
    terminate_process(recovered_process)
    raise


def validate_packaged_runtime_protocol(app_path: Path) -> None:
  with tempfile.TemporaryDirectory(prefix="pith-runtime-protocol-") as support_root:
    support_dir = Path(support_root)
    process = launch_runtime_process(app_path, support_dir)
    try:
      initialize = send_runtime_request(
        process,
        1,
        "initialize",
        {
          "clientInfo": {
            "name": "packaged-runtime-smoke",
            "version": "0.1.0",
          }
        },
      )
      if initialize["result"]["serverInfo"]["name"] != "pith-runtime":
        raise RuntimeError("Packaged runtime initialize returned the wrong server name.")

      bootstrap = send_runtime_request(process, 2, "model/bootstrap")
      manifest_path = Path(bootstrap["result"]["manifestPath"])
      validate_default_model_manifest(manifest_path)

      plugin_list = send_runtime_request(process, 3, "plugin/list")
      plugin_ids = {plugin["id"] for plugin in plugin_list["result"]["plugins"]}
      missing_plugins = sorted(REQUIRED_BUNDLED_PLUGINS - plugin_ids)
      if missing_plugins:
        raise RuntimeError(
          "Packaged runtime is missing bundled plugins "
          f"{', '.join(missing_plugins)}."
        )

      validate_runtime_readiness(send_runtime_request(process, 4, "runtime/readiness"))
      validate_packaged_runtime_workspace_bootstrap(process, support_dir)
      validate_runtime_database(support_dir / "storage" / "pith.db")
      validate_packaged_first_cowork_request(app_path)
      print(
        "Packaged runtime protocol smoke passed with model metadata and plugins "
        f"under {support_dir}"
      )
    finally:
      terminate_process(process)


def validate_app_runtime_stability(
  app_process: subprocess.Popen[str],
  launched_runtime_pids: set[int],
  stability_duration: float,
) -> None:
  stability_deadline = time.monotonic() + stability_duration
  while time.monotonic() < stability_deadline:
    if app_process.poll() is not None:
      stdout, stderr = app_process.communicate()
      raise RuntimeError(
        "Packaged app launched the runtime but exited during the smoke window.\n"
        f"stdout:\n{stdout[-2000:]}\n"
        f"stderr:\n{stderr[-2000:]}"
      )
    if launched_runtime_pids.isdisjoint(process_ids(RUNTIME_PROCESS_NAME)):
      raise RuntimeError("Packaged runtime exited during the smoke window.")
    time.sleep(0.2)


def print_packaged_app_success(
  app_process: subprocess.Popen[str],
  launched_runtime_pids: set[int],
  support_dir: Path,
) -> None:
  print(
    "Packaged app launch smoke passed with app PID "
    f"{app_process.pid}, runtime PIDs {sorted(launched_runtime_pids)}, "
    f"and isolated support root {support_dir}"
  )


def main() -> int:
  args = parse_args()
  if sys.platform != "darwin":
    print("Skipping macOS app launch smoke outside Darwin.")
    return 0

  app_path = args.app_path.resolve()
  validate_app_bundle(app_path)
  validate_packaged_runtime_protocol(app_path)
  before_runtime_pids = process_ids(RUNTIME_PROCESS_NAME)
  stability_duration = (
    args.duration if args.duration is not None else args.stability_duration
  )
  with tempfile.TemporaryDirectory(prefix="pith-app-smoke-") as support_root:
    support_dir = Path(support_root)
    app_process = launch_app_process(app_path, support_dir)
    launched_runtime_pids: set[int] = set()
    startup_deadline = time.monotonic() + args.startup_timeout
    try:
      while time.monotonic() < startup_deadline:
        if app_process.poll() is not None:
          stdout, stderr = app_process.communicate()
          raise RuntimeError(
            "Packaged app exited before launching the runtime.\n"
            f"stdout:\n{stdout[-2000:]}\n"
            f"stderr:\n{stderr[-2000:]}"
          )
        launched_runtime_pids = process_ids(RUNTIME_PROCESS_NAME) - before_runtime_pids
        if launched_runtime_pids:
          validate_app_runtime_stability(
            app_process,
            launched_runtime_pids,
            stability_duration,
          )
          validate_isolated_support_dir(support_dir)
          print_packaged_app_success(app_process, launched_runtime_pids, support_dir)
          return 0
        time.sleep(0.2)

      stdout, stderr = terminate_process_with_output(app_process)
      raise RuntimeError(
        "Packaged app did not start pith-runtime-bin within "
        f"{args.startup_timeout:.0f}s.\n"
        f"stdout:\n{stdout[-2000:]}\n"
        f"stderr:\n{stderr[-2000:]}"
      )
    finally:
      terminate_process(app_process)
      if launched_runtime_pids:
        terminate_processes(RUNTIME_PROCESS_NAME, launched_runtime_pids)


if __name__ == "__main__":
  try:
    raise SystemExit(main())
  except Exception as error:
    print(f"macOS app launch smoke failed: {error}", file=sys.stderr)
    raise SystemExit(1)
