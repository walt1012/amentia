#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path


def start_runtime(repo_root: Path, env: dict[str, str]) -> subprocess.Popen[str]:
  return subprocess.Popen(
    ["cargo", "run", "-p", "cavell-runtime-bin"],
    cwd=repo_root,
    env=env,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
  )


def send_request(process: subprocess.Popen[str], payload: dict) -> tuple[dict, list[dict]]:
  assert process.stdin is not None
  assert process.stdout is not None

  process.stdin.write(json.dumps(payload) + "\n")
  process.stdin.flush()

  notifications: list[dict] = []
  while True:
    line = process.stdout.readline().strip()
    if not line:
      raise RuntimeError("runtime produced an empty response")

    message = json.loads(line)
    if message.get("id") == payload["id"]:
      return message, notifications

    if "method" in message:
      notifications.append(message)

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
  env = os.environ.copy()
  env["CAVELL_DATA_DIR"] = str(state_dir)
  process = start_runtime(repo_root, env)

  try:
    initialize, _ = send_request(
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

    health, _ = send_request(
      process,
      {
        "id": 2,
        "method": "health/ping",
      },
    )
    assert health["result"]["status"] == "ok"

    model_health, _ = send_request(
      process,
      {
        "id": 21,
        "method": "model/health",
      },
    )
    assert model_health["result"]["displayName"] == "LFM2.5-350M"
    assert model_health["result"]["backend"] in {"heuristic", "llama.cpp"}
    assert model_health["result"]["status"] in {"fallback", "ready"}
    assert model_health["result"]["source"] in {"bundle-manifest", "environment", "path-scan"}
    assert model_health["result"]["metrics"]["contextSize"] == "4096"

    memory_status, _ = send_request(
      process,
      {
        "id": 22,
        "method": "memory/status",
      },
    )
    assert memory_status["result"]["noteCount"] == 0

    memory_list, _ = send_request(
      process,
      {
        "id": 23,
        "method": "memory/list",
      },
    )
    assert memory_list["result"]["notes"] == []

    plugin_list, _ = send_request(
      process,
      {
        "id": 25,
        "method": "plugin/list",
      },
    )
    assert isinstance(plugin_list["result"]["plugins"], list)

    workspace, _ = send_request(
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

    memory_status_after_workspace, _ = send_request(
      process,
      {
        "id": 27,
        "method": "memory/status",
      },
    )
    assert memory_status_after_workspace["result"]["noteCount"] == 1

    started, _ = send_request(
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

    thread_list, _ = send_request(
      process,
      {
        "id": 5,
        "method": "thread/list",
      },
    )
    assert len(thread_list["result"]["threads"]) == 1

    thread_read, _ = send_request(
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

    turn, _ = send_request(
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
    assert turn["result"]["items"][1]["attributes"]["responseRole"] == "planner"
    assert int(turn["result"]["items"][1]["attributes"]["memoryNoteCount"]) >= 1
    assert "Opened workspace" in turn["result"]["items"][1]["attributes"]["memoryNoteTitles"]
    assert turn["result"]["items"][2]["kind"] == "toolStart"
    assert turn["result"]["items"][3]["kind"] == "toolResult"
    assert turn["result"]["items"][4]["kind"] == "assistantMessage"
    assert turn["result"]["items"][4]["attributes"]["responseRole"] == "summarizer"
    assert int(turn["result"]["items"][4]["attributes"]["memoryNoteCount"]) >= 1
    assert turn["result"]["items"][4]["attributes"]["streamingStatus"] in {"in_progress", "completed"}
    assert "Milestone 1 smoke test" in turn["result"]["items"][3]["content"]
    assert turn["result"]["activeTurnId"] == "thread-1-turn-1"
    time.sleep(0.35)

    streamed_thread, _ = send_request(
      process,
      {
        "id": 26,
        "method": "thread/read",
        "params": {
          "threadId": "thread-1",
        },
      },
    )
    assistant_items = [
      item for item in streamed_thread["result"]["items"]
      if item["kind"] == "assistantMessage" and item.get("attributes", {}).get("turnId") == "thread-1-turn-1"
    ]
    assert assistant_items
    latest_assistant = assistant_items[-1]
    assert latest_assistant["attributes"]["streamingStatus"] in {"in_progress", "completed"}
    assert int(latest_assistant["attributes"]["totalCharacters"]) >= int(
      latest_assistant["attributes"]["streamedCharacters"]
    )

    if streamed_thread["result"].get("activeTurnId") is not None:
      cancelled_turn, _ = send_request(
        process,
        {
          "id": 8,
          "method": "turn/cancel",
          "params": {
            "turnId": turn["result"]["activeTurnId"],
          },
        },
      )
      assert cancelled_turn["result"]["items"][0]["kind"] == "warning"
      assert cancelled_turn["result"].get("activeTurnId") is None

      cancelled_thread, _ = send_request(
        process,
        {
          "id": 9,
          "method": "thread/read",
          "params": {
            "threadId": "thread-1",
          },
        },
      )
      assert cancelled_thread["result"].get("activeTurnId") is None
    else:
      assert latest_assistant["attributes"]["streamingStatus"] == "completed"

    search_turn, _ = send_request(
      process,
      {
        "id": 10,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Find Needle term",
        },
      },
    )
    assert search_turn["result"]["items"][2]["title"] == "search_files"
    assert "notes.txt:1" in search_turn["result"]["items"][3]["content"]
    assert int(search_turn["result"]["items"][1]["attributes"]["memoryNoteCount"]) >= 1

    write_turn, _ = send_request(
      process,
      {
        "id": 11,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Created from approval flow",
        },
      },
    )
    assert write_turn["result"]["items"][2]["title"] == "generate_diff"
    assert write_turn["result"]["items"][3]["kind"] == "diffArtifact"
    assert "+++ b/docs/output.txt" in write_turn["result"]["items"][3]["content"]
    assert write_turn["result"]["items"][4]["kind"] == "approvalRequested"
    approval_id = write_turn["result"]["pendingApprovals"][0]["id"]

    process.terminate()
    process.wait(timeout=5)
    process = start_runtime(repo_root, env)

    restarted_initialize, _ = send_request(
      process,
      {
        "id": 28,
        "method": "initialize",
        "params": {
          "clientInfo": {
            "name": "runtime-smoke-test",
            "version": "0.1.0",
          }
        },
      },
    )
    assert restarted_initialize["result"]["serverInfo"]["name"] == "cavell-runtime"

    restarted_thread, _ = send_request(
      process,
      {
        "id": 29,
        "method": "thread/read",
        "params": {
          "threadId": "thread-1",
        },
      },
    )
    assert restarted_thread["result"]["pendingApprovals"][0]["id"] == approval_id
    assert any(
      item["kind"] == "approvalRequested"
      for item in restarted_thread["result"]["items"]
    )

    restarted_workspace, _ = send_request(
      process,
      {
        "id": 30,
        "method": "workspace/current",
      },
    )
    assert restarted_workspace["result"]["workspace"]["displayName"] == workspace_dir.name
    assert restarted_workspace["result"]["workspace"]["rootPath"] == str(workspace_dir)

    restarted_memory_status, _ = send_request(
      process,
      {
        "id": 31,
        "method": "memory/status",
      },
    )
    assert restarted_memory_status["result"]["noteCount"] >= 1

    approval, _ = send_request(
      process,
      {
        "id": 12,
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

    memory_list_after_write, _ = send_request(
      process,
      {
        "id": 32,
        "method": "memory/list",
      },
    )
    assert any("Wrote docs/output.txt" == note["title"] for note in memory_list_after_write["result"]["notes"])

    post_write_turn, _ = send_request(
      process,
      {
        "id": 33,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Read docs/output.txt",
        },
      },
    )
    assert "Wrote docs/output.txt" in post_write_turn["result"]["items"][1]["attributes"]["memoryNoteTitles"]

    shell_turn, _ = send_request(
      process,
      {
        "id": 13,
        "method": "turn/start",
        "params": {
          "threadId": "thread-1",
          "message": "Run shell: ls",
        },
      },
    )
    assert shell_turn["result"]["items"][2]["kind"] == "approvalRequested"
    shell_approval_id = shell_turn["result"]["pendingApprovals"][0]["id"]

    shell_approval, _ = send_request(
      process,
      {
        "id": 14,
        "method": "approval/respond",
        "params": {
          "approvalId": shell_approval_id,
          "decision": "approved",
        },
      },
    )
    assert shell_approval["result"]["items"][1]["title"] == "run_shell"
    assert "notes.txt" in shell_approval["result"]["items"][2]["content"]
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
