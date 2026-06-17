#!/usr/bin/env python3
"""Reusable checks for bundled connector workflow contracts."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
BUNDLED_PLUGIN_ROOT = ROOT / "plugins" / "bundled"
MIN_AGENT_STEPS = 1
MAX_AGENT_STEPS = 8


def read_json_object(path: Path) -> dict[str, Any]:
  value = json.loads(path.read_text(encoding="utf-8"))
  if not isinstance(value, dict):
    raise AssertionError(f"Expected JSON object: {path}")
  return value


def command_files(plugin_root: Path) -> dict[str, dict[str, Any]]:
  commands: dict[str, dict[str, Any]] = {}
  for path in sorted((plugin_root / "commands").glob("*.json")):
    commands[path.stem] = read_json_object(path)
  return commands


def require(condition: bool, message: str) -> None:
  if not condition:
    raise AssertionError(message)


def validate_plugin(plugin_root: Path) -> None:
  manifest_path = plugin_root / "amentia-plugin.json"
  if not manifest_path.exists():
    return

  manifest = read_json_object(manifest_path)
  capabilities = {
    value
    for value in manifest.get("capabilities", [])
    if isinstance(value, str)
  }
  workflows = [
    workflow
    for workflow in manifest.get("connectorWorkflows", [])
    if isinstance(workflow, dict)
  ]
  if not workflows:
    return

  connectors = {
    connector.get("id")
    for connector in manifest.get("appConnectors", [])
    if isinstance(connector, dict) and isinstance(connector.get("id"), str)
  }
  commands = command_files(plugin_root)
  workflow_ids = {
    workflow.get("id")
    for workflow in workflows
    if isinstance(workflow.get("id"), str)
  }
  workflow_command_ids: dict[str, list[str]] = {workflow_id: [] for workflow_id in workflow_ids}

  for workflow in workflows:
    workflow_id = workflow.get("id")
    connector_id = workflow.get("connectorId")
    action = workflow.get("action")
    stages = workflow.get("stages")
    statuses = workflow.get("statuses")
    max_agent_steps = workflow.get("maxAgentSteps")
    require(isinstance(workflow_id, str) and bool(workflow_id.strip()), "workflow id is required")
    require(
      f"connector_workflow:{workflow_id}" in capabilities,
      f"{plugin_root.name} workflow {workflow_id} is missing a capability",
    )
    require(
      connector_id in connectors,
      f"{plugin_root.name} workflow {workflow_id} references an undeclared connector",
    )
    require(
      isinstance(action, str) and bool(action.strip()),
      f"{plugin_root.name} workflow {workflow_id} is missing an action",
    )
    require(
      isinstance(stages, list) and all(isinstance(stage, str) and stage for stage in stages),
      f"{plugin_root.name} workflow {workflow_id} needs non-empty string stages",
    )
    require(
      isinstance(statuses, list)
      and all(isinstance(status, str) and status for status in statuses),
      f"{plugin_root.name} workflow {workflow_id} needs non-empty string statuses",
    )
    if max_agent_steps is not None:
      require(
        isinstance(max_agent_steps, int)
        and MIN_AGENT_STEPS <= max_agent_steps <= MAX_AGENT_STEPS,
        f"{plugin_root.name} workflow {workflow_id} maxAgentSteps must be 1..8",
      )

  for command_id, command in commands.items():
    execution = command.get("execution")
    if not isinstance(execution, dict):
      continue
    workflow_id = execution.get("workflowId")
    if not isinstance(workflow_id, str) or not workflow_id.strip():
      continue
    require(
      workflow_id in workflow_ids,
      f"{plugin_root.name} command {command_id} references undeclared workflow {workflow_id}",
    )
    require(
      f"command:{command_id}" in capabilities,
      f"{plugin_root.name} command {command_id} is missing a command capability",
    )
    workflow = next(item for item in workflows if item.get("id") == workflow_id)
    connectors_for_command = execution.get("connectors")
    require(
      isinstance(connectors_for_command, list)
      and workflow.get("connectorId") in connectors_for_command,
      f"{plugin_root.name} command {command_id} is not bound to the workflow connector",
    )
    workflow_command_ids.setdefault(workflow_id, []).append(command_id)

  for workflow_id, command_ids in workflow_command_ids.items():
    require(
      bool(command_ids),
      f"{plugin_root.name} workflow {workflow_id} has no command coverage",
    )


def main() -> int:
  for plugin_root in sorted(BUNDLED_PLUGIN_ROOT.iterdir()):
    if plugin_root.is_dir():
      validate_plugin(plugin_root)
  print("connector workflow contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
