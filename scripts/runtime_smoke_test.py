#!/usr/bin/env python3

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


def send_request(process: subprocess.Popen[str], payload: dict) -> dict:
  assert process.stdin is not None
  assert process.stdout is not None

  process.stdin.write(json.dumps(payload) + "\n")
  process.stdin.flush()

  line = process.stdout.readline().strip()
  if not line:
    raise RuntimeError("runtime produced an empty response")

  return json.loads(line)


def main() -> int:
  repo_root = Path(__file__).resolve().parent.parent
  command = ["cargo", "run", "-p", "cavell-runtime-bin"]

  process = subprocess.Popen(
    command,
    cwd=repo_root,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
  )

  try:
    initialize = send_request(
      process,
      {
        "id": 1,
        "method": "initialize",
        "params": {
          "clientInfo": {
            "name": "runtime-smoke-test",
            "version": "0.1.0",
          }
        },
      },
    )
    assert initialize["result"]["serverInfo"]["name"] == "cavell-runtime"

    health = send_request(
      process,
      {
        "id": 2,
        "method": "health/ping",
      },
    )
    assert health["result"]["status"] == "ok"

    started = send_request(
      process,
      {
        "id": 3,
        "method": "thread/start",
        "params": {
          "title": "Smoke Test Thread",
        },
      },
    )
    assert started["result"]["thread"]["title"] == "Smoke Test Thread"

    thread_list = send_request(
      process,
      {
        "id": 4,
        "method": "thread/list",
      },
    )
    assert len(thread_list["result"]["threads"]) == 1

    turn = send_request(
      process,
      {
        "id": 5,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Hello from CI",
        },
      },
    )
    assert turn["result"]["messages"][0]["role"] == "user"
    assert turn["result"]["messages"][1]["role"] == "assistant"
    return 0
  finally:
    process.terminate()
    process.wait(timeout=5)


if __name__ == "__main__":
  try:
    raise SystemExit(main())
  except Exception as error:
    print(f"runtime smoke test failed: {error}", file=sys.stderr)
    raise
