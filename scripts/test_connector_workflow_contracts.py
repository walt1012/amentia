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
REQUIRED_ENVELOPE_KEYS = ("envelope", "fields")
REMOTE_WRITE_INPUT_FIELD_KINDS = {
  "input": "text",
  "connectors": "connectorRefs",
}
WORKFLOW_OUTPUT_FIELD_KINDS = {
  "items": "timelineItems",
  "memoryNotes": "memoryNotes",
}


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


def is_safe_bundle_relative_path(value: object) -> bool:
  if not isinstance(value, str):
    return False
  path = value.strip()
  if not path or "\\" in path or ":" in path or path.startswith("/"):
    return False
  return not any(part in {"", ".", ".."} for part in path.split("/"))


def validate_bundle_local_paths(plugin_root: Path, manifest: dict[str, Any]) -> None:
  for skill in manifest.get("skills", []):
    if not isinstance(skill, dict):
      raise AssertionError(f"{plugin_root.name} skill entries must be objects")
    skill_id = skill.get("id", "unknown")
    require(
      is_safe_bundle_relative_path(skill.get("path")),
      f"{plugin_root.name} skill {skill_id} path must stay inside the plugin bundle",
    )

  for server in manifest.get("mcpServers", []):
    if not isinstance(server, dict):
      raise AssertionError(f"{plugin_root.name} MCP server entries must be objects")
    command = server.get("command")
    if command is None:
      continue
    server_id = server.get("id", "unknown")
    require(
      is_safe_bundle_relative_path(command),
      f"{plugin_root.name} MCP server {server_id} command must stay inside the plugin bundle",
    )


def validate_skill_capabilities(
  plugin_root: Path,
  manifest: dict[str, Any],
  capabilities: set[str],
) -> None:
  declared_skill_ids: set[str] = set()
  for skill in manifest.get("skills", []):
    if not isinstance(skill, dict):
      raise AssertionError(f"{plugin_root.name} skill entries must be objects")
    skill_id = skill.get("id")
    require(
      isinstance(skill_id, str) and bool(skill_id.strip()),
      f"{plugin_root.name} skill entry must declare an id",
    )
    declared_skill_ids.add(skill_id)
    require(
      f"skill:{skill_id}" in capabilities,
      f"{plugin_root.name} skill {skill_id} is missing a skill capability",
    )

  for capability in capabilities:
    if not capability.startswith("skill:"):
      continue
    skill_id = capability.removeprefix("skill:")
    require(
      skill_id in declared_skill_ids,
      f"{plugin_root.name} skill capability {capability} has no skill entry",
    )


def envelope_field_kinds(envelope: object) -> dict[str, str]:
  if not isinstance(envelope, dict):
    return {}
  fields = envelope.get("fields")
  if not isinstance(fields, list):
    return {}
  kinds: dict[str, str] = {}
  for field in fields:
    if not isinstance(field, dict):
      continue
    name = field.get("name")
    kind = field.get("kind")
    if isinstance(name, str) and isinstance(kind, str):
      kinds[name] = kind
  return kinds


def validate_execution_envelope(
  plugin_root: Path,
  command_id: str,
  execution: dict[str, Any],
  envelope_key: str,
) -> dict[str, str]:
  envelope = execution.get(envelope_key)
  require(
    isinstance(envelope, dict),
    f"{plugin_root.name} command {command_id} must declare execution.{envelope_key}",
  )
  for key in REQUIRED_ENVELOPE_KEYS:
    require(
      key in envelope,
      f"{plugin_root.name} command {command_id} execution.{envelope_key} missed {key}",
    )
  envelope_name = envelope.get("envelope")
  require(
    isinstance(envelope_name, str) and bool(envelope_name.strip()),
    f"{plugin_root.name} command {command_id} execution.{envelope_key}.envelope is required",
  )
  fields = envelope.get("fields")
  require(
    isinstance(fields, list) and bool(fields),
    f"{plugin_root.name} command {command_id} execution.{envelope_key}.fields must be non-empty",
  )
  return envelope_field_kinds(envelope)


def require_named_fields(
  plugin_root: Path,
  command_id: str,
  envelope_key: str,
  actual: dict[str, str],
  expected: dict[str, str],
) -> None:
  for name, kind in expected.items():
    require(
      actual.get(name) == kind,
      f"{plugin_root.name} command {command_id} execution.{envelope_key} "
      f"must declare {name}:{kind}",
    )


def assert_bundle_path_classifier() -> None:
  safe_paths = ["skills/notion-workspace.md", "bin/notion-mcp-local-draft.sh"]
  unsafe_paths = [
    "",
    "../outside.md",
    "skills/../../outside.md",
    "/tmp/outside.md",
    "C:\\outside.md",
    "bin\\runner.sh",
    "skills//notes.md",
    "skills/./notes.md",
  ]
  for path in safe_paths:
    require(is_safe_bundle_relative_path(path), f"{path} should be bundle-local")
  for path in unsafe_paths:
    require(not is_safe_bundle_relative_path(path), f"{path} should be rejected")


def validate_plugin(plugin_root: Path) -> None:
  manifest_path = plugin_root / "amentia-plugin.json"
  if not manifest_path.exists():
    return

  manifest = read_json_object(manifest_path)
  validate_bundle_local_paths(plugin_root, manifest)
  capabilities = {
    value
    for value in manifest.get("capabilities", [])
    if isinstance(value, str)
  }
  validate_skill_capabilities(plugin_root, manifest, capabilities)
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
    input_fields = validate_execution_envelope(plugin_root, command_id, execution, "input")
    output_fields = validate_execution_envelope(plugin_root, command_id, execution, "output")
    require_named_fields(
      plugin_root,
      command_id,
      "output",
      output_fields,
      WORKFLOW_OUTPUT_FIELD_KINDS,
    )
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
    if command_id.endswith(".publish-page-draft"):
      require_named_fields(
        plugin_root,
        command_id,
        "input",
        input_fields,
        REMOTE_WRITE_INPUT_FIELD_KINDS,
      )
    workflow_command_ids.setdefault(workflow_id, []).append(command_id)

  for workflow_id, command_ids in workflow_command_ids.items():
    require(
      bool(command_ids),
      f"{plugin_root.name} workflow {workflow_id} has no command coverage",
    )


def main() -> int:
  assert_bundle_path_classifier()
  for plugin_root in sorted(BUNDLED_PLUGIN_ROOT.iterdir()):
    if plugin_root.is_dir():
      validate_plugin(plugin_root)
  print("connector workflow contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
