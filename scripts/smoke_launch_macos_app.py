#!/usr/bin/env python3

from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path


APP_PROCESS_NAME = "Pith"
RUNTIME_PROCESS_NAME = "pith-runtime-bin"
APP_SUPPORT_ENV_KEY = "PITH_APP_SUPPORT_DIR"


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


def main() -> int:
  args = parse_args()
  if sys.platform != "darwin":
    print("Skipping macOS app launch smoke outside Darwin.")
    return 0

  app_path = args.app_path.resolve()
  validate_app_bundle(app_path)
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
