#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path


APP_PROCESS_NAME = "Pith"
RUNTIME_PROCESS_NAME = "pith-runtime-bin"
APP_SUPPORT_ENV_KEY = "PITH_APP_SUPPORT_DIR"
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
    default=6.0,
    help="Seconds the launched app must remain alive.",
  )
  return parser.parse_args()


def require_file(path: Path) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Required packaged app file is missing: {path}")


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


def launch_runtime_process(app_path: Path, support_dir: Path) -> subprocess.Popen[str]:
  resources_path = bundled_resource_path(app_path)
  environment = os.environ.copy()
  environment["PITH_DATA_DIR"] = str(support_dir / "storage")
  environment["PITH_LOCAL_PLUGIN_DIR"] = str(support_dir / "plugins")
  environment["PITH_MODEL_PACK_ROOT"] = str(resources_path)
  environment["PITH_PLUGIN_DIR"] = str(resources_path / "plugins")
  environment["PITH_LLAMACPP_PATH"] = str(
    resources_path / "tools" / "llama.cpp" / "llama-cli"
  )
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


def validate_isolated_support_dir(support_dir: Path) -> None:
  storage_dir = support_dir / "storage"
  if not storage_dir.is_dir():
    raise RuntimeError(f"Packaged app did not create isolated storage: {storage_dir}")
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
    line = process.stdout.readline()
    if not line:
      raise RuntimeError(f"Packaged runtime exited before responding to {method}.")
    message = json.loads(line)
    if message.get("id") == request_id:
      if "error" in message:
        raise RuntimeError(f"Packaged runtime {method} failed: {message['error']}")
      return message


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
      require_file(manifest_path)
      if manifest_path.suffix != ".json":
        raise RuntimeError(f"Packaged runtime copied an invalid model manifest: {manifest_path}")
      if (manifest_path.parent / "LFM2.5-350M-Q4_K_M.gguf").exists():
        raise RuntimeError("Packaged runtime smoke unexpectedly found bundled model weights.")

      plugin_list = send_runtime_request(process, 3, "plugin/list")
      plugin_ids = {plugin["id"] for plugin in plugin_list["result"]["plugins"]}
      missing_plugins = sorted(REQUIRED_BUNDLED_PLUGINS - plugin_ids)
      if missing_plugins:
        raise RuntimeError(
          "Packaged runtime is missing bundled plugins "
          f"{', '.join(missing_plugins)}."
        )

      validate_runtime_database(support_dir / "storage" / "pith.db")
      print(
        "Packaged runtime protocol smoke passed with model metadata and plugins "
        f"under {support_dir}"
      )
    finally:
      terminate_process(process)


def main() -> int:
  args = parse_args()
  if sys.platform != "darwin":
    print("Skipping macOS app launch smoke outside Darwin.")
    return 0

  app_path = args.app_path.resolve()
  validate_app_bundle(app_path)
  validate_packaged_runtime_protocol(app_path)
  before_runtime_pids = process_ids(RUNTIME_PROCESS_NAME)
  with tempfile.TemporaryDirectory(prefix="pith-app-smoke-") as support_root:
    support_dir = Path(support_root)
    app_process = launch_app_process(app_path, support_dir)
    launched_runtime_pids: set[int] = set()
    deadline = time.monotonic() + args.duration
    try:
      while time.monotonic() < deadline:
        if app_process.poll() is not None:
          stdout, stderr = app_process.communicate()
          raise RuntimeError(
            "Packaged app exited before the launch smoke window.\n"
            f"stdout:\n{stdout[-2000:]}\n"
            f"stderr:\n{stderr[-2000:]}"
          )
        launched_runtime_pids = process_ids(RUNTIME_PROCESS_NAME) - before_runtime_pids
        if launched_runtime_pids:
          time.sleep(max(0.0, deadline - time.monotonic()))
          if app_process.poll() is not None:
            stdout, stderr = app_process.communicate()
            raise RuntimeError(
              "Packaged app launched but exited during the smoke window.\n"
              f"stdout:\n{stdout[-2000:]}\n"
              f"stderr:\n{stderr[-2000:]}"
            )
          if launched_runtime_pids.isdisjoint(process_ids(RUNTIME_PROCESS_NAME)):
            raise RuntimeError("Packaged runtime exited during the smoke window.")
          validate_isolated_support_dir(support_dir)
          print(
            "Packaged app launch smoke passed with app PID "
            f"{app_process.pid}, runtime PIDs {sorted(launched_runtime_pids)}, "
            f"and isolated support root {support_dir}"
          )
          return 0
        time.sleep(0.2)

      raise RuntimeError("Packaged app did not start pith-runtime-bin.")
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
