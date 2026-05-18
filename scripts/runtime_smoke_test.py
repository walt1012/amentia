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
    ["cargo", "run", "-p", "pith-runtime-bin"],
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

def assert_plugin_install_remove(
  process: subprocess.Popen[str],
  plugin_import_dir: Path,
  request_id_start: int,
) -> None:
  plugin_inspect, _ = send_request(
    process,
    {
      "id": request_id_start,
      "method": "plugin/inspect",
      "params": {
        "sourcePath": str(plugin_import_dir / "focus-review"),
      },
    },
  )
  assert plugin_inspect["result"]["plugin"]["id"] == "focus-review"
  assert plugin_inspect["result"]["plugin"]["provenance"] == "local"
  assert plugin_inspect["result"]["installStatus"] == "ready"

  plugin_install, _ = send_request(
    process,
    {
      "id": request_id_start + 1,
      "method": "plugin/install",
      "params": {
        "sourcePath": str(plugin_import_dir / "focus-review"),
      },
    },
  )
  assert plugin_install["result"]["plugin"]["id"] == "focus-review"
  assert plugin_install["result"]["plugin"]["provenance"] == "local"

  duplicate_plugin_inspect, _ = send_request(
    process,
    {
      "id": request_id_start + 2,
      "method": "plugin/inspect",
      "params": {
        "sourcePath": str(plugin_import_dir / "focus-review"),
      },
    },
  )
  assert duplicate_plugin_inspect["result"]["installStatus"] == "alreadyInstalled"
  assert "already installed" in duplicate_plugin_inspect["result"]["installBlocker"]

  plugin_list_after_install, _ = send_request(
    process,
    {
      "id": request_id_start + 3,
      "method": "plugin/list",
    },
  )
  installed_plugin = next(
    plugin
    for plugin in plugin_list_after_install["result"]["plugins"]
    if plugin["id"] == "focus-review"
  )
  assert installed_plugin["enabled"] is True
  assert Path(installed_plugin["manifestPath"]).is_file()

  plugin_remove, _ = send_request(
    process,
    {
      "id": request_id_start + 4,
      "method": "plugin/remove",
      "params": {
        "manifestPath": installed_plugin["manifestPath"],
      },
    },
  )
  assert plugin_remove["result"]["pluginId"] == "focus-review"
  assert Path(plugin_remove["result"]["removedPath"]).exists() is False

  plugin_list_after_remove, _ = send_request(
    process,
    {
      "id": request_id_start + 5,
      "method": "plugin/list",
    },
  )
  assert not any(
    plugin["id"] == "focus-review"
    for plugin in plugin_list_after_remove["result"]["plugins"]
  )

def assert_builtin_plugin_commands(
  process: subprocess.Popen[str],
  request_id_start: int,
) -> None:
  review_command_turn, _ = send_request(
    process,
    {
      "id": request_id_start,
      "method": "plugin/commandRun",
      "params": {
        "threadId": "thread-1",
        "commandId": "review-assistant::review.inspect-diff",
      },
    },
  )
  assert review_command_turn["result"]["items"][0]["kind"] == "pluginCommand"
  assert review_command_turn["result"]["items"][0]["attributes"]["pluginId"] == "review-assistant"
  assert review_command_turn["result"]["items"][1]["kind"] == "pluginResult"
  assert (
    review_command_turn["result"]["items"][1]["attributes"]["executionKind"]
    == "builtin.reviewDiffSummary"
  )

  command_turn, _ = send_request(
    process,
    {
      "id": request_id_start + 1,
      "method": "plugin/commandRun",
      "params": {
        "threadId": "thread-1",
        "commandId": "workspace-notes::workspace.capture-note",
      },
    },
  )
  assert command_turn["result"]["items"][0]["kind"] == "pluginCommand"
  assert command_turn["result"]["items"][0]["attributes"]["pluginId"] == "workspace-notes"
  assert command_turn["result"]["items"][1]["kind"] == "pluginResult"
  assert (
    command_turn["result"]["items"][1]["attributes"]["executionKind"]
    == "builtin.workspaceReadmeNote"
  )
  memory_item = next(
    item
    for item in command_turn["result"]["items"]
    if item["title"] == "Memory Note Saved"
  )
  assert memory_item["attributes"]["memoryNoteTitle"] == "Workspace Capture"
  memory_list_after_plugin, _ = send_request(
    process,
    {
      "id": request_id_start + 2,
      "method": "memory/list",
    },
  )
  assert any(
    note["title"] == "Workspace Capture" and note["source"] == "plugin.workspace-notes"
    for note in memory_list_after_plugin["result"]["notes"]
  )
  cancel_running, _ = send_request(
    process,
    {
      "id": request_id_start + 3,
      "method": "turn/cancelRunning",
      "params": {
        "threadId": "thread-1",
      },
    },
  )
  assert cancel_running["result"]["threadId"] == "thread-1"

  cancelled_command_turn, _ = send_request(
    process,
    {
      "id": request_id_start + 4,
      "method": "plugin/commandRun",
      "params": {
        "threadId": "thread-1",
        "commandId": "workspace-notes::workspace.capture-note",
        "input": "Cancel before plugin execution",
      },
    },
  )
  assert cancelled_command_turn["result"]["items"][0]["kind"] == "pluginCommand"
  assert cancelled_command_turn["result"]["items"][1]["kind"] == "warning"
  assert (
    cancelled_command_turn["result"]["items"][1]["attributes"]["pluginCommandStatus"]
    == "cancelled"
  )
  assert (
    cancelled_command_turn["result"]["items"][1]["attributes"]["pluginRunnerFailureKind"]
    == "cancelled"
  )
  post_cancel_readiness, _ = send_request(
    process,
    {
      "id": request_id_start + 5,
      "method": "runtime/readiness",
    },
  )
  post_cancel_checks = {
    check["id"]: check for check in post_cancel_readiness["result"]["checks"]
  }
  assert post_cancel_checks["executionControls"]["status"] == "ready"
  assert post_cancel_readiness["result"]["metrics"]["runningPluginCommandCount"] == "0"

def main() -> int:
  repo_root = Path(__file__).resolve().parent.parent
  state_dir = repo_root / ".tmp-runtime-state"
  plugin_dir = repo_root / ".tmp-runtime-plugins"
  plugin_import_dir = repo_root / ".tmp-runtime-plugin-import"
  workspace_dir = repo_root / ".tmp-runtime-workspace"
  if state_dir.exists():
    shutil.rmtree(state_dir)
  if plugin_dir.exists():
    shutil.rmtree(plugin_dir)
  if plugin_import_dir.exists():
    shutil.rmtree(plugin_import_dir)
  if workspace_dir.exists():
    shutil.rmtree(workspace_dir)
  (plugin_dir / "workspace-notes").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "shell-recorder").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "review-assistant").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "notion-connector").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "workspace-notes" / "commands").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "shell-recorder" / "commands").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "shell-recorder" / "hooks").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "review-assistant" / "commands").mkdir(parents=True, exist_ok=True)
  (plugin_dir / "workspace-notes" / "pith-plugin.json").write_text(
    json.dumps(
      {
        "name": "workspace-notes",
        "version": "0.1.0",
        "displayName": "Workspace Notes",
        "description": "Captures reusable workspace notes and preferences for local threads.",
        "author": {
          "name": "Pith",
        },
        "capabilities": [
          "command:workspace.capture-note",
          "prompt_pack:workspace.notes",
          "settings:workspace.preferences",
        ],
        "permissions": [
          "file.read",
          "file.write",
        ],
        "defaultEnabled": True,
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "notion-connector" / "pith-plugin.json").write_text(
    json.dumps(
      {
        "name": "notion-connector",
        "version": "0.1.0",
        "displayName": "Notion Connector",
        "description": "Declares the Notion connector surface for MCP and OAuth-backed workspace integrations.",
        "author": {
          "name": "Pith",
        },
        "capabilities": [],
        "permissions": [
          "network.outbound",
          "mcp.connect",
        ],
        "mcpServers": [
          {
            "id": "notion",
            "transport": "stdio",
          },
        ],
        "appConnectors": [
          {
            "id": "notion",
            "displayName": "Notion",
            "service": "notion",
            "homepage": "https://www.notion.so",
          },
        ],
        "authPolicy": {
          "type": "oauth2",
          "required": True,
          "scopes": [
            "read_content",
            "insert_content",
          ],
          "credentialStore": "local",
        },
        "defaultEnabled": False,
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "workspace-notes" / "commands" / "workspace.capture-note.json").write_text(
    json.dumps(
      {
        "title": "Capture Workspace Note",
        "description": "Read the workspace README and prepare a concise note candidate.",
        "prompt": "Read README.md and summarize the most reusable workspace detail as a concise note candidate.",
        "execution": {
          "kind": "builtin.workspaceReadmeNote",
        },
        "memory": {
          "noteTitle": "Workspace Capture",
          "noteSource": "plugin.workspace-notes",
          "noteTags": ["workspace", "preference", "plugin"],
        },
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "shell-recorder" / "pith-plugin.json").write_text(
    json.dumps(
      {
        "name": "shell-recorder",
        "version": "0.1.0",
        "displayName": "Shell Recorder",
        "description": "Tracks shell-oriented workspace actions for later inspection and summaries.",
        "author": {
          "name": "Pith",
        },
        "capabilities": [
          "command:shell.summarize-session",
          "hook:shell.recorder",
          "tool:shell.timeline",
        ],
        "permissions": [
          "shell.exec",
        ],
        "defaultEnabled": False,
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "shell-recorder" / "commands" / "shell.summarize-session.json").write_text(
    json.dumps(
      {
        "title": "Summarize Shell Session",
        "description": "Ask Pith to explain recent shell work in a compact summary.",
        "prompt": "Summarize the most relevant recent shell activity for this workspace.",
        "execution": {
          "kind": "builtin.shellSessionSummary",
        },
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "shell-recorder" / "hooks" / "shell.recorder.json").write_text(
    json.dumps(
      {
        "title": "Record Shell Completion",
        "description": "Capture a compact shell completion note in the thread timeline.",
        "event": "shell.completed",
        "messageTemplate": "Shell Recorder observed `{{command}}` in {{workspaceName}} with exit code {{exitCode}}. stdout: {{stdoutPreview}} stderr: {{stderrPreview}}",
        "memory": {
          "noteTitle": "Shell Completion",
          "noteSource": "plugin.shell-recorder",
          "noteTags": ["shell", "hook", "plugin"],
        },
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "review-assistant" / "pith-plugin.json").write_text(
    json.dumps(
      {
        "name": "review-assistant",
        "version": "0.1.0",
        "displayName": "Review Assistant",
        "description": "Provides review-oriented prompts and metadata for local code inspection flows.",
        "author": {
          "name": "Pith",
        },
        "capabilities": [
          "command:review.inspect-diff",
          "prompt_pack:review.prompts",
          "tool:diff.summaries",
        ],
        "permissions": [
          "file.read",
          "model.invoke",
        ],
        "defaultEnabled": True,
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_dir / "review-assistant" / "commands" / "review.inspect-diff.json").write_text(
    json.dumps(
      {
        "title": "Inspect Current Diff",
        "description": "Ask Pith to review the active workspace diff with a code review mindset.",
        "prompt": "Inspect the current workspace diff and review it for bugs, regressions, missing tests, and risky behavior changes. Report findings first with clear severity.",
        "execution": {
          "kind": "builtin.reviewDiffSummary",
        },
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_import_dir / "focus-review" / "commands").mkdir(parents=True, exist_ok=True)
  (plugin_import_dir / "focus-review" / "pith-plugin.json").write_text(
    json.dumps(
      {
        "name": "focus-review",
        "version": "0.1.0",
        "displayName": "Focus Review",
        "description": "Installs into the local plugin catalog during the runtime smoke test.",
        "author": {
          "name": "Pith",
        },
        "capabilities": [
          "command:focus.review",
        ],
        "permissions": [
          "file.read",
        ],
        "defaultEnabled": True,
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  (plugin_import_dir / "focus-review" / "commands" / "focus.review.json").write_text(
    json.dumps(
      {
        "title": "Focus Review",
        "description": "Prepare a focused local review summary.",
        "prompt": "Review the latest local changes and keep the summary focused on the most important issues.",
      },
      indent=2,
    ),
    encoding="utf-8",
  )
  workspace_dir.mkdir(parents=True, exist_ok=True)
  (workspace_dir / "README.md").write_text("# Pith\nMilestone 1 smoke test\n", encoding="utf-8")
  (workspace_dir / "apps").mkdir()
  (workspace_dir / "notes.txt").write_text("Needle term for search tool\n", encoding="utf-8")
  env = os.environ.copy()
  env["PITH_DATA_DIR"] = str(state_dir)
  env["PITH_PLUGIN_DIR"] = str(plugin_dir)
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
    assert initialize["result"]["serverInfo"]["name"] == "pith-runtime"
    assert initialize["result"]["capabilities"]["supportsRuntimeReadiness"] is True

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
    assert model_health["result"]["displayName"] == "LFM2.5-350M Q4_K_M"
    assert model_health["result"]["backend"] in {"unconfigured", "llama.cpp"}
    assert model_health["result"]["status"] in {"unavailable", "ready"}
    model_is_ready = model_health["result"]["status"] == "ready"
    assert model_health["result"]["source"] in {"default-manifest", "environment", "path-scan"}
    assert model_health["result"]["metrics"]["contextSize"] == "4096"
    assert model_health["result"]["metrics"]["modelContextSize"] == "32768"
    assert model_health["result"]["metrics"]["fileName"] == "LFM2.5-350M-Q4_K_M.gguf"
    assert model_health["result"]["metrics"]["downloadUrl"].startswith(
      "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF/resolve/main/"
    )
    assert (
      model_health["result"]["metrics"]["sha256"]
      == "7e6f72643caafc9a68256686638c4d7916f2cec76d1df478d4c3ddcd95a6aed4"
    )
    assert model_health["result"]["metrics"]["sizeBytes"] == "229312224"
    assert model_health["result"]["metrics"]["readiness"] in {
      "ready",
      "manifest_only",
      "model_missing",
      "binary_missing",
      "misconfigured",
      "unconfigured",
    }
    assert model_health["result"]["metrics"]["packReady"] in {"true", "false"}
    assert model_health["result"]["metrics"]["installHint"]
    assert model_health["result"]["metrics"]["suggestedManifestPath"]
    assert model_health["result"]["metrics"]["suggestedModelPath"].endswith(
      "LFM2.5-350M-Q4_K_M.gguf"
    )
    assert model_health["result"]["metrics"]["suggestedBinaryPath"]

    runtime_readiness, _ = send_request(
      process,
      {
        "id": 22,
        "method": "runtime/readiness",
      },
    )
    assert runtime_readiness["result"]["status"] in {"setup_required", "ready"}
    assert runtime_readiness["result"]["summary"]
    readiness_check_ids = {check["id"] for check in runtime_readiness["result"]["checks"]}
    assert {
      "localModel",
      "workspace",
      "thread",
      "firstRequest",
      "context",
      "executionControls",
      "nativeSandbox",
      "webSearch",
      "plugins",
      "boundedRuntime",
    }.issubset(readiness_check_ids)
    assert runtime_readiness["result"]["metrics"]["contextWindowTokens"] == "4096"
    assert "workspaceThreadCount" in runtime_readiness["result"]["metrics"]
    assert runtime_readiness["result"]["metrics"]["firstRequestSent"] in {"true", "false"}
    assert runtime_readiness["result"]["metrics"]["shellTimeoutSeconds"] == "120"
    assert runtime_readiness["result"]["metrics"]["llamaTimeoutSeconds"] == "180"
    assert runtime_readiness["result"]["metrics"]["sandboxMode"] == "workspaceReadWrite"
    assert runtime_readiness["result"]["metrics"]["sandboxActive"] in {"true", "false"}
    assert runtime_readiness["result"]["metrics"]["webSearchTimeoutSeconds"] == "20"
    assert runtime_readiness["result"]["metrics"]["webSearchProvider"] == "DuckDuckGo Lite"
    assert runtime_readiness["result"]["metrics"]["webSearchClient"] == "curl"
    assert runtime_readiness["result"]["metrics"]["webSearchAvailable"] in {"true", "false"}
    assert "pluginCommandCount" in runtime_readiness["result"]["metrics"]
    assert "enabledPluginCommandCount" in runtime_readiness["result"]["metrics"]
    assert runtime_readiness["result"]["metrics"]["pendingApprovalCount"] == "0"
    assert runtime_readiness["result"]["metrics"]["runningTurnCount"] == "0"
    assert runtime_readiness["result"]["metrics"]["runningApprovalCount"] == "0"
    assert runtime_readiness["result"]["metrics"]["runningPluginCommandCount"] == "0"

    model_bootstrap, _ = send_request(
      process,
      {
        "id": 23,
        "method": "model/bootstrap",
      },
    )
    assert Path(model_bootstrap["result"]["manifestPath"]).is_file()
    if model_bootstrap["result"]["readmePath"] is not None:
      assert Path(model_bootstrap["result"]["readmePath"]).is_file()

    memory_status, _ = send_request(
      process,
      {
        "id": 24,
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
    plugin_ids = {plugin["id"] for plugin in plugin_list["result"]["plugins"]}
    assert "workspace-notes" in plugin_ids
    assert "shell-recorder" in plugin_ids
    assert "review-assistant" in plugin_ids
    assert "notion-connector" in plugin_ids

    workspace_notes_enable, _ = send_request(
      process,
      {
        "id": 36,
        "method": "plugin/setEnabled",
        "params": {
          "pluginId": "workspace-notes",
          "enabled": True,
        },
      },
    )
    assert workspace_notes_enable["result"]["plugin"]["enabled"] is True

    shell_recorder_enable, _ = send_request(
      process,
      {
        "id": 37,
        "method": "plugin/setEnabled",
        "params": {
          "pluginId": "shell-recorder",
          "enabled": True,
        },
      },
    )
    assert shell_recorder_enable["result"]["plugin"]["enabled"] is True

    plugin_readiness, _ = send_request(
      process,
      {
        "id": 43,
        "method": "runtime/readiness",
      },
    )
    plugin_checks = {
      check["id"]: check for check in plugin_readiness["result"]["checks"]
    }
    assert plugin_checks["plugins"]["status"] == "ready"
    assert "command capability" in plugin_checks["plugins"]["detail"]
    assert int(plugin_readiness["result"]["metrics"]["pluginCommandCount"]) >= 3
    assert int(plugin_readiness["result"]["metrics"]["enabledPluginCommandCount"]) >= 3

    capability_registry, _ = send_request(
      process,
      {
        "id": 38,
        "method": "plugin/capabilityRegistry",
      },
    )
    assert capability_registry["result"]["summary"]["enabledPluginCount"] >= 3
    capability_plugin_ids = {
      capability["pluginId"] for capability in capability_registry["result"]["capabilities"]
    }
    assert "workspace-notes" in capability_plugin_ids
    assert "shell-recorder" in capability_plugin_ids
    assert "review-assistant" in capability_plugin_ids
    command_registry, _ = send_request(
      process,
      {
        "id": 39,
        "method": "plugin/commandRegistry",
      },
    )
    assert len(command_registry["result"]["commands"]) == 3
    command_ids = {command["commandId"] for command in command_registry["result"]["commands"]}
    assert "workspace-notes::workspace.capture-note" in command_ids
    assert "shell-recorder::shell.summarize-session" in command_ids
    assert "review-assistant::review.inspect-diff" in command_ids
    workspace_capture_command = next(
      command
      for command in command_registry["result"]["commands"]
      if command["commandId"] == "workspace-notes::workspace.capture-note"
    )
    assert (
      workspace_capture_command["memorySummary"]
      == "Stores a workspace memory note as `Workspace Capture` after execution."
    )
    assert workspace_capture_command["executionKind"] == "builtin.workspaceReadmeNote"
    connector_registry, _ = send_request(
      process,
      {
        "id": 41,
        "method": "plugin/connectorRegistry",
      },
    )
    connectors = connector_registry["result"]["connectors"]
    notion_connector = next(
      connector
      for connector in connectors
      if connector["connectorId"] == "notion-connector::notion"
    )
    assert notion_connector["status"] == "disabled"
    assert notion_connector["authType"] == "oauth2"
    assert notion_connector["credentialStore"] == "local"
    assert notion_connector["authScopes"] == ["read_content", "insert_content"]
    hook_registry, _ = send_request(
      process,
      {
        "id": 42,
        "method": "plugin/hookRegistry",
      },
    )
    assert len(hook_registry["result"]["hooks"]) == 1
    assert hook_registry["result"]["hooks"][0]["hookId"] == "shell-recorder::shell.recorder"
    assert hook_registry["result"]["hooks"][0]["event"] == "shell.completed"

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

    workspace_readiness, _ = send_request(
      process,
      {
        "id": 49,
        "method": "runtime/readiness",
      },
    )
    workspace_readiness_checks = {
      check["id"]: check for check in workspace_readiness["result"]["checks"]
    }
    assert workspace_readiness["result"]["metrics"]["workspaceBound"] == "true"
    assert workspace_readiness_checks["workspace"]["status"] == "ready"

    memory_status_after_workspace, _ = send_request(
      process,
      {
        "id": 27,
        "method": "memory/status",
      },
    )
    assert memory_status_after_workspace["result"]["noteCount"] == 1

    created_memory_note, _ = send_request(
      process,
      {
        "id": 34,
        "method": "memory/create",
        "params": {
          "title": "Workspace preference",
          "body": "Prefer concise Milestone 1 execution summaries.",
        },
      },
    )
    assert created_memory_note["result"]["note"]["title"] == "Workspace preference"
    assert created_memory_note["result"]["note"]["source"] == "user"

    memory_status_after_manual_note, _ = send_request(
      process,
      {
        "id": 35,
        "method": "memory/status",
      },
    )
    assert memory_status_after_manual_note["result"]["noteCount"] == 2

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
    if not model_is_ready:
      assert turn["error"]["code"] == -32060
      assert "Local model is not ready" in turn["error"]["message"]
      assert_plugin_install_remove(process, plugin_import_dir, 60)
      assert_builtin_plugin_commands(process, 70)
      return 0

    assert turn["result"]["items"][0]["kind"] == "userMessage"
    assert turn["result"]["items"][1]["kind"] == "plan"
    assert turn["result"]["items"][1]["attributes"]["responseRole"] == "planner"
    assert int(turn["result"]["items"][1]["attributes"]["memoryNoteCount"]) >= 2
    assert "Opened workspace" in turn["result"]["items"][1]["attributes"]["memoryNoteTitles"]
    assert "Workspace preference" in turn["result"]["items"][1]["attributes"]["memoryNoteTitles"]
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
    assert restarted_initialize["result"]["serverInfo"]["name"] == "pith-runtime"

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
    assert any(
      "Thread summary: Smoke Test Thread" == note["title"]
      for note in memory_list_after_write["result"]["notes"]
    )

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
    assert shell_turn["result"]["items"][2]["attributes"]["sandboxMode"] == "workspaceReadWrite"
    assert shell_turn["result"]["items"][2]["attributes"]["sandboxAvailable"] in {
      "true",
      "false",
    }
    assert shell_turn["result"]["items"][2]["attributes"]["sandboxActive"] in {
      "true",
      "false",
    }
    assert "Sandbox:" in shell_turn["result"]["items"][2]["content"]
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
    assert shell_approval["result"]["items"][2]["attributes"]["sandboxMode"] == "workspaceReadWrite"
    assert shell_approval["result"]["items"][2]["attributes"]["sandboxBackend"] in {
      "macosSeatbelt",
      "processOnly",
    }
    assert shell_approval["result"]["items"][2]["attributes"]["sandboxAvailable"] in {
      "true",
      "false",
    }
    assert shell_approval["result"]["items"][2]["attributes"]["sandboxActive"] in {
      "true",
      "false",
    }
    assert any(item["kind"] == "pluginHook" for item in shell_approval["result"]["items"])
    assert any(
      item["title"] == "Record Shell Completion"
      and item["attributes"]["hookEvent"] == "shell.completed"
      and item["attributes"]["sandboxMode"] == "workspaceReadWrite"
      for item in shell_approval["result"]["items"]
    )
    assert any(
      item["title"] == "Hook Memory Note Saved"
      and item["attributes"]["memoryNoteTitle"] == "Shell Completion"
      for item in shell_approval["result"]["items"]
    )
    memory_list_after_shell, _ = send_request(
      process,
      {
        "id": 47,
        "method": "memory/list",
      },
    )
    assert any(
      note["title"] == "Shell Completion" and note["source"] == "plugin.shell-recorder"
      for note in memory_list_after_shell["result"]["notes"]
    )

    assert_plugin_install_remove(process, plugin_import_dir, 42)
    assert_builtin_plugin_commands(process, 48)
    return 0
  finally:
    process.terminate()
    process.wait(timeout=5)
    if plugin_dir.exists():
      shutil.rmtree(plugin_dir)
    if plugin_import_dir.exists():
      shutil.rmtree(plugin_import_dir)
    if workspace_dir.exists():
      shutil.rmtree(workspace_dir)


if __name__ == "__main__":
  try:
    raise SystemExit(main())
  except Exception as error:
    print(f"runtime smoke test failed: {error}", file=sys.stderr)
    raise
