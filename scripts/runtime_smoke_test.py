#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import shutil
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
  state_dir = repo_root / ".tmp-runtime-state"
  workspace_dir = repo_root / ".tmp-runtime-workspace"
  if state_dir.exists():
    shutil.rmtree(state_dir)
  if workspace_dir.exists():
    shutil.rmtree(workspace_dir)
  workspace_dir.mkdir(parents=True, exist_ok=True)
  (workspace_dir / "README.md").write_text("# Cavell\nMilestone 1 smoke test\n", encoding="utf-8")
  (workspace_dir / "apps").mkdir()
  (workspace_dir / "notes.txt").write_text("Needle term for search tool\n", encoding="utf-8")
  command = ["cargo", "run", "-p", "cavell-runtime-bin"]
  env = os.environ.copy()
  env["CAVELL_DATA_DIR"] = str(state_dir)

  process = subprocess.Popen(
    command,
    cwd=repo_root,
    env=env,
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

    workspace = send_request(
      process,
      {
        "id": 3,
        "method": "workspace/open",
        "params": {
          "path": str(workspace_dir),
        },
      },
    )
    assert workspace["result"]["workspace"]["displayName"] == workspace_dir.name

    started = send_request(
      process,
      {
        "id": 4,
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
        "id": 5,
        "method": "thread/list",
      },
    )
    assert len(thread_list["result"]["threads"]) == 1

    thread_read = send_request(
      process,
      {
        "id": 6,
        "method": "thread/read",
        "params": {
          "threadId": "thread-1",
        },
      },
    )
    assert thread_read["result"]["thread"]["id"] == "thread-1"
    assert thread_read["result"]["items"][0]["kind"] == "system"

    turn = send_request(
      process,
      {
        "id": 7,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Read README.md",
        },
      },
    )
    assert turn["result"]["items"][0]["kind"] == "userMessage"
    assert turn["result"]["items"][1]["kind"] == "plan"
    assert turn["result"]["items"][2]["kind"] == "toolStart"
    assert turn["result"]["items"][3]["kind"] == "toolResult"
    assert "Milestone 1 smoke test" in turn["result"]["items"][3]["content"]

    search_turn = send_request(
      process,
      {
        "id": 8,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Find Needle term",
        },
      },
    )
    assert search_turn["result"]["items"][2]["title"] == "search_files"
    assert "notes.txt:1" in search_turn["result"]["items"][3]["content"]

    write_turn = send_request(
      process,
      {
        "id": 9,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Created from approval flow",
        },
      },
    )
    assert write_turn["result"]["items"][2]["kind"] == "approvalRequested"
    approval_id = write_turn["result"]["pendingApprovals"][0]["id"]

    approval = send_request(
      process,
      {
        "id": 10,
        "method": "approval/respond",
        "params": {
          "approvalId": approval_id,
          "decision": "approved",
        },
      },
    )
    assert approval["result"]["items"][0]["kind"] == "approvalResolved"
    assert approval["result"]["items"][1]["title"] == "write_file"
    assert (workspace_dir / "docs" / "output.txt").read_text(encoding="utf-8") == "Created from approval flow"
    return 0
  finally:
    process.terminate()
    process.wait(timeout=5)
    if workspace_dir.exists():
      shutil.rmtree(workspace_dir)


if __name__ == "__main__":
  try:
    raise SystemExit(main())
  except Exception as error:
    print(f"runtime smoke test failed: {error}", file=sys.stderr)
    raise
