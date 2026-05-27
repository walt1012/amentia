#!/usr/bin/env python3
"""Contract checks for the bundled Slack connector MCP server."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parents[1]
CONNECTOR_PATH = (
  ROOT / "plugins" / "bundled" / "slack-connector" / "bin" / "slack-mcp-local-message.py"
)
PLUGIN_ROOT = ROOT / "plugins" / "bundled" / "slack-connector"
POST_COMMAND_ID = "slack-connector::slack.post-message-draft"
WORKFLOW_ID = "slack.post-message"
CHANNEL_ID = "C123ABC456"
MESSAGE_TS = "1712345678.000100"


def load_connector() -> ModuleType:
  spec = importlib.util.spec_from_file_location("pith_slack_connector", CONNECTOR_PATH)
  if spec is None or spec.loader is None:
    raise AssertionError(f"Could not load connector at {CONNECTOR_PATH}")
  module = importlib.util.module_from_spec(spec)
  spec.loader.exec_module(module)
  return module


def first_item(structured_content: dict) -> dict:
  items = structured_content.get("items")
  if not isinstance(items, list) or not items:
    raise AssertionError(f"Expected at least one plugin item, got {structured_content}")
  item = items[0]
  if not isinstance(item, dict):
    raise AssertionError(f"Expected plugin item object, got {item!r}")
  return item


def response_first_item(response: dict) -> dict:
  structured = response.get("result", {}).get("structuredContent")
  if not isinstance(structured, dict):
    raise AssertionError(f"Expected structured MCP content, got {response}")
  return first_item(structured)


def assert_workflow(
  item: dict,
  *,
  action: str,
  stage: str,
  status: str,
  target: str,
  proof: str,
) -> None:
  attributes = item.get("attributes")
  if not isinstance(attributes, dict):
    raise AssertionError(f"Plugin item missed attributes: {item}")
  expected = {
    "connectorWorkflowId": WORKFLOW_ID,
    "connectorWorkflowName": "Slack Post Message",
    "connectorWorkflowService": "slack",
    "connectorWorkflowAction": action,
    "connectorWorkflowStage": stage,
    "connectorWorkflowStatus": status,
    "connectorWorkflowTarget": target,
    "connectorWorkflowProof": proof,
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(
        f"Expected {key}={value!r} in Slack workflow attributes, got {attributes}"
      )


def assert_post_follow_up(item: dict) -> None:
  attributes = item.get("attributes")
  if not isinstance(attributes, dict):
    raise AssertionError(f"Plugin item missed attributes: {item}")
  expected = {
    "nextCommand": "slack.post-message-draft",
    "nextCommandId": POST_COMMAND_ID,
    "nextCommandLabel": "Post to Slack",
    "nextCommandInputRequired": "true",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(
        f"Expected {key}={value!r} in Slack follow-up attributes, got {attributes}"
      )
  if "channelId" not in attributes.get("nextCommandInputHint", ""):
    raise AssertionError(f"Slack follow-up missed input hint: {attributes}")
  template = json.loads(attributes.get("nextCommandInputTemplate", "{}"))
  for key in ["channelId", "message"]:
    if key not in template or not isinstance(template[key], str):
      raise AssertionError(f"Slack follow-up missed {key} template: {template}")
  if template["channelId"] != "":
    raise AssertionError(f"Slack follow-up should leave channelId empty: {template}")


def assert_manifest_workflow_coverage() -> None:
  manifest = json.loads((PLUGIN_ROOT / "pith-plugin.json").read_text(encoding="utf-8"))
  command_capabilities = sorted(
    capability.removeprefix("command:")
    for capability in manifest.get("capabilities", [])
    if capability.startswith("command:")
  )
  command_files = sorted(path.stem for path in (PLUGIN_ROOT / "commands").glob("*.json"))
  if command_capabilities != command_files:
    raise AssertionError(
      f"Slack command capabilities and files diverged: {command_capabilities} != {command_files}"
    )

  workflows = {
    workflow.get("id"): workflow
    for workflow in manifest.get("connectorWorkflows", [])
    if isinstance(workflow, dict)
  }
  if WORKFLOW_ID not in workflows:
    raise AssertionError(f"Slack manifest missed workflow {WORKFLOW_ID}")
  if workflows[WORKFLOW_ID].get("maxAgentSteps") != 5:
    raise AssertionError(f"Slack workflow should request a 5-step bounded loop: {workflows}")

  for command_id in command_files:
    command = json.loads(
      (PLUGIN_ROOT / "commands" / f"{command_id}.json").read_text(encoding="utf-8")
    )
    execution = command.get("execution", {})
    if execution.get("workflowId") != WORKFLOW_ID:
      raise AssertionError(f"Slack command {command_id} is not bound to {WORKFLOW_ID}")
    if "slack" not in execution.get("connectors", []):
      raise AssertionError(f"Slack command {command_id} missed connector binding")


def assert_prepare_draft(connector: ModuleType) -> None:
  structured = connector.prepare_message_draft(
    {
      "input": (
        "Prepare a Slack update from docs/handoff.md.\n\n"
        "Saved artifact: docs/handoff.md\n"
        "Saved artifact preview: Ship the cowork connector path.\n"
        "Saved artifact truncated: false"
      )
    }
  )
  item = first_item(structured)
  attributes = item.get("attributes", {})
  expected = {
    "targetService": "slack",
    "draftMode": "localDraft",
    "remoteWrite": "false",
    "targetTool": "slack.prepareMessageDraft",
    "sourceArtifact": "docs/handoff.md",
    "sourceArtifactPreviewProvided": "true",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(f"Expected Slack draft {key}={value!r}, got {attributes}")
  assert_workflow(
    item,
    action="postMessage",
    stage="draftPrepared",
    status="prepared",
    target="docs/handoff.md",
    proof="localDraft",
  )
  assert_post_follow_up(item)
  memory_notes = structured.get("memoryNotes", [])
  if not isinstance(memory_notes, list) or memory_notes[0].get("source") != "plugin.slack-connector":
    raise AssertionError(f"Slack draft missed memory note: {structured}")


def assert_post_success(connector: ModuleType) -> None:
  captured: dict[str, object] = {}
  previous_token = os.environ.get("PITH_TEST_SLACK_TOKEN")
  os.environ["PITH_TEST_SLACK_TOKEN"] = "xoxb-test-token"

  def fake_slack_post(path: str, token: str, payload: dict) -> dict:
    captured["path"] = path
    captured["token"] = token
    captured["payload"] = payload
    return {
      "ok": True,
      "channel": CHANNEL_ID,
      "ts": MESSAGE_TS,
    }

  connector.slack_post = fake_slack_post
  try:
    response = connector.post_message_draft(
      3,
      {
        "input": json.dumps(
          {
            "channelId": CHANNEL_ID,
            "message": "Ship the cowork connector path.",
          },
          sort_keys=True,
        ),
        "connectors": [
          {
            "service": "slack",
            "credentialProvider": {
              "provider": "env",
              "envKey": "PITH_TEST_SLACK_TOKEN",
            },
          }
        ],
      },
    )
  finally:
    if previous_token is None:
      os.environ.pop("PITH_TEST_SLACK_TOKEN", None)
    else:
      os.environ["PITH_TEST_SLACK_TOKEN"] = previous_token

  item = response_first_item(response)
  structured = response.get("result", {}).get("structuredContent", {})
  attributes = item.get("attributes", {})
  expected_url = (
    f"https://slack.com/app_redirect?channel={CHANNEL_ID}&message_ts={MESSAGE_TS}"
  )
  expected = {
    "remoteWrite": "true",
    "remoteWriteStage": "completed",
    "remoteProofKind": "slackApiResponse",
    "remoteProofStatus": "success",
    "remoteProofId": f"{CHANNEL_ID}:{MESSAGE_TS}",
    "remoteProofUrl": expected_url,
    "remoteProofTitle": "Slack message sent",
    "remoteProofActionTitle": "Open Slack Message",
    "slackChannelId": CHANNEL_ID,
    "slackMessageTs": MESSAGE_TS,
    "messageTruncated": "false",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(f"Expected successful Slack post {key}={value!r}, got {attributes}")
  assert_workflow(
    item,
    action="postMessage",
    stage="completed",
    status="completed",
    target=CHANNEL_ID,
    proof="slackApiResponse",
  )
  if captured.get("path") != "/chat.postMessage" or captured.get("token") != "xoxb-test-token":
    raise AssertionError(f"Slack post called the wrong target: {captured}")
  payload = captured.get("payload")
  if not isinstance(payload, dict) or payload.get("channel") != CHANNEL_ID:
    raise AssertionError(f"Slack post used the wrong payload: {captured}")
  memory_notes = structured.get("memoryNotes", [])
  if not isinstance(memory_notes, list) or len(memory_notes) != 1:
    raise AssertionError(f"Slack post missed memory note: {structured}")
  memory_body = memory_notes[0].get("body", "")
  for expected_part in [CHANNEL_ID, MESSAGE_TS, expected_url, "Message truncated: false."]:
    if expected_part not in memory_body:
      raise AssertionError(f"Slack post memory missed {expected_part!r}: {memory_notes}")


def assert_post_retry(connector: ModuleType) -> None:
  response = connector.post_message_draft(
    4,
    {
      "input": json.dumps({"message": "Missing channel."}, sort_keys=True),
      "connectors": [],
    },
  )
  item = response_first_item(response)
  attributes = item.get("attributes", {})
  expected = {
    "remoteWrite": "false",
    "remoteWriteStage": "blockedBeforeWrite",
    "remoteProofKind": "slackApiResponse",
    "remoteProofStatus": "missing",
    "publishFailureReason": "missingChannelId",
    "retryCommandId": POST_COMMAND_ID,
    "retryInputEditable": "true",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(f"Expected Slack retry {key}={value!r}, got {attributes}")
  retry_input = json.loads(attributes.get("retryInput", "{}"))
  if retry_input.get("channelId") != "" or retry_input.get("message") != "Missing channel.":
    raise AssertionError(f"Slack retry did not preserve editable input: {attributes}")
  assert_workflow(
    item,
    action="postMessage",
    stage="blockedBeforeWrite",
    status="retryNeeded",
    target="missingChannel",
    proof="missing",
  )


def main() -> int:
  connector = load_connector()
  assert_manifest_workflow_coverage()
  assert_prepare_draft(connector)
  assert_post_success(connector)
  assert_post_retry(connector)
  print("Slack connector contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
