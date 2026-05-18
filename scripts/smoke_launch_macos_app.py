#!/usr/bin/env python3

from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import time
from pathlib import Path


APP_PROCESS_NAME = "Pith"


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


def terminate_processes(process_ids_to_stop: set[int]) -> None:
  for pid in process_ids_to_stop:
    try:
      os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
      pass
  deadline = time.monotonic() + 5
  while time.monotonic() < deadline:
    if process_ids_to_stop.isdisjoint(process_ids(APP_PROCESS_NAME)):
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
  require_file(app_path / "Contents" / "Resources" / "PithPackage.json")


def main() -> int:
  args = parse_args()
  if sys.platform != "darwin":
    print("Skipping macOS app launch smoke outside Darwin.")
    return 0

  app_path = args.app_path.resolve()
  validate_app_bundle(app_path)
  before_pids = process_ids(APP_PROCESS_NAME)
  open_process = subprocess.Popen(
    ["/usr/bin/open", "-n", "-W", str(app_path)],
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
  )
  launched_pids: set[int] = set()
  deadline = time.monotonic() + args.duration
  try:
    while time.monotonic() < deadline:
      if open_process.poll() is not None:
        stdout, stderr = open_process.communicate()
        raise RuntimeError(
          "Packaged app exited before the launch smoke window.\n"
          f"stdout:\n{stdout[-2000:]}\n"
          f"stderr:\n{stderr[-2000:]}"
        )
      launched_pids = process_ids(APP_PROCESS_NAME) - before_pids
      if launched_pids:
        time.sleep(max(0.0, deadline - time.monotonic()))
        if open_process.poll() is not None:
          stdout, stderr = open_process.communicate()
          raise RuntimeError(
            "Packaged app launched but exited during the smoke window.\n"
            f"stdout:\n{stdout[-2000:]}\n"
            f"stderr:\n{stderr[-2000:]}"
          )
        print(f"Packaged app launch smoke passed with PIDs: {sorted(launched_pids)}")
        return 0
      time.sleep(0.2)

    raise RuntimeError("Packaged app did not appear as a running Pith process.")
  finally:
    if launched_pids:
      terminate_processes(launched_pids)
    if open_process.poll() is None:
      open_process.terminate()
      try:
        open_process.wait(timeout=5)
      except subprocess.TimeoutExpired:
        open_process.kill()


if __name__ == "__main__":
  try:
    raise SystemExit(main())
  except Exception as error:
    print(f"macOS app launch smoke failed: {error}", file=sys.stderr)
    raise SystemExit(1)
