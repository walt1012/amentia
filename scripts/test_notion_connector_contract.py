#!/usr/bin/env python3
"""Contract checks for the bundled Notion connector MCP server."""

from __future__ import annotations

import importlib.util
import json
import os
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parents[1]
CONNECTOR_PATH = (
  ROOT / "plugins" / "bundled" / "notion-connector" / "bin" / "notion-mcp-local-draft.py"
)
PLUGIN_ROOT = ROOT / "plugins" / "bundled" / "notion-connector"
PUBLISH_COMMAND_ID = "notion-connector::notion.publish-page-draft"
WORKFLOW_ID = "notion.create-page"


def load_connector() -> ModuleType:
  spec = importlib.util.spec_from_file_location("pith_notion_connector", CONNECTOR_PATH)
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


def assert_publish_follow_up(item: dict) -> None:
  attributes = item.get("attributes")
  if not isinstance(attributes, dict):
    raise AssertionError(f"Plugin item missed attributes: {item}")
  expected = {
    "nextCommand": "notion.publish-page-draft",
    "nextCommandId": PUBLISH_COMMAND_ID,
    "nextCommandLabel": "Publish to Notion",
    "nextCommandInputRequired": "true",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(
        f"Expected {key}={value!r} in Notion follow-up attributes, got {attributes}"
      )
  if "parentPageId" not in attributes.get("nextCommandInputHint", ""):
    raise AssertionError(f"Notion follow-up missed input hint: {attributes}")
  template = json.loads(attributes.get("nextCommandInputTemplate", "{}"))
  for key in ["parentPageId", "title", "body"]:
    if key not in template or not isinstance(template[key], str):
      raise AssertionError(f"Notion follow-up missed {key} template: {template}")
  if template["parentPageId"] != "":
    raise AssertionError(f"Notion follow-up should leave parentPageId empty: {template}")


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
    "connectorWorkflowName": "Notion Create Page",
    "connectorWorkflowService": "notion",
    "connectorWorkflowAction": action,
    "connectorWorkflowStage": stage,
    "connectorWorkflowStatus": status,
    "connectorWorkflowTarget": target,
    "connectorWorkflowProof": proof,
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(
        f"Expected {key}={value!r} in Notion workflow attributes, got {attributes}"
      )


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
      f"Notion command capabilities and files diverged: {command_capabilities} != {command_files}"
    )

  workflows = {
    workflow.get("id"): workflow
    for workflow in manifest.get("connectorWorkflows", [])
    if isinstance(workflow, dict)
  }
  if WORKFLOW_ID not in workflows:
    raise AssertionError(f"Notion manifest missed workflow {WORKFLOW_ID}")
  if workflows[WORKFLOW_ID].get("maxAgentSteps") != 5:
    raise AssertionError(f"Notion workflow should request a 5-step bounded loop: {workflows}")

  for command_id in command_files:
    command = json.loads(
      (PLUGIN_ROOT / "commands" / f"{command_id}.json").read_text(encoding="utf-8")
    )
    execution = command.get("execution", {})
    if execution.get("workflowId") != WORKFLOW_ID:
      raise AssertionError(f"Notion command {command_id} is not bound to {WORKFLOW_ID}")
    if "notion" not in execution.get("connectors", []):
      raise AssertionError(f"Notion command {command_id} missed connector binding")


def assert_publish_success(connector: ModuleType) -> None:
  captured: dict[str, object] = {}
  previous_token = os.environ.get("PITH_TEST_NOTION_TOKEN")
  os.environ["PITH_TEST_NOTION_TOKEN"] = "secret-token"

  def fake_notion_post(path: str, token: str, payload: dict) -> dict:
    captured["path"] = path
    captured["token"] = token
    captured["payload"] = payload
    return {
      "id": "page-success-123",
      "url": "https://www.notion.so/page-success-123",
    }

  connector.notion_post = fake_notion_post
  try:
    response = connector.publish_page_draft(
      3,
      {
        "input": json.dumps(
          {
            "parentPageId": "parent-123",
            "title": "Published by test",
            "body": "Paragraph one.\n\nParagraph two.",
          },
          sort_keys=True,
        ),
        "connectors": [
          {
            "service": "notion",
            "credentialProvider": {
              "provider": "env",
              "envKey": "PITH_TEST_NOTION_TOKEN",
            },
          }
        ],
      },
    )
  finally:
    if previous_token is None:
      os.environ.pop("PITH_TEST_NOTION_TOKEN", None)
    else:
      os.environ["PITH_TEST_NOTION_TOKEN"] = previous_token
  item = response_first_item(response)
  attributes = item.get("attributes", {})
  expected = {
    "remoteWrite": "true",
    "remoteWriteStage": "completed",
    "remoteProofKind": "notionApiResponse",
    "remoteProofStatus": "success",
    "notionPageId": "page-success-123",
    "notionPageUrl": "https://www.notion.so/page-success-123",
    "notionParentPageId": "parent-123",
    "bodyTruncated": "false",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(f"Expected successful publish {key}={value!r}, got {attributes}")
  assert_workflow(
    item,
    action="createPage",
    stage="completed",
    status="completed",
    target="https://www.notion.so/page-success-123",
    proof="notionApiResponse",
  )
  if captured.get("path") != "/pages" or captured.get("token") != "secret-token":
    raise AssertionError(f"Notion publish called the wrong target: {captured}")
  payload = captured.get("payload")
  if not isinstance(payload, dict):
    raise AssertionError(f"Notion publish payload was not a dict: {captured}")
  if payload.get("parent", {}).get("page_id") != "parent-123":
    raise AssertionError(f"Notion publish used the wrong parent: {payload}")
  children = payload.get("children")
  if not isinstance(children, list) or len(children) != 2:
    raise AssertionError(f"Notion publish did not preserve paragraph blocks: {payload}")


def main() -> int:
  assert_manifest_workflow_coverage()
  connector = load_connector()

  draft = connector.prepare_page_draft(
    {
      "input": "\n".join(
        [
          "Saved artifact: docs/handoff.md",
          "Saved artifact preview: Draft content",
          "Saved artifact truncated: false",
        ]
      )
    }
  )
  draft_item = first_item(draft)
  assert_publish_follow_up(draft_item)
  assert_workflow(
    draft_item,
    action="createPage",
    stage="draftPrepared",
    status="prepared",
    target="docs/handoff.md",
    proof="localDraft",
  )

  inspection = connector.inspect_page_write({"input": "Saved artifact: docs/handoff.md"})
  inspection_item = first_item(inspection)
  assert_publish_follow_up(inspection_item)
  assert_workflow(
    inspection_item,
    action="createPage",
    stage="inspectBeforeWrite",
    status="inspected",
    target="docs/handoff.md",
    proof="inspection",
  )

  retry = connector.publish_page_draft(
    1,
    {
      "input": json.dumps(
        {
          "parentPageId": "page-123",
          "title": "Retry me",
          "body": "No token is present, so this should be retryable.",
        },
        sort_keys=True,
      )
    },
  )
  retry_item = response_first_item(retry)
  retry_attributes = retry_item.get("attributes", {})
  if retry_attributes.get("retryCommandId") != PUBLISH_COMMAND_ID:
    raise AssertionError(f"Notion retry missed command handoff: {retry_attributes}")
  retry_input = json.loads(retry_attributes.get("retryInput", "{}"))
  if retry_input.get("parentPageId") != "page-123":
    raise AssertionError(f"Notion retry did not preserve publish input: {retry_input}")
  assert_workflow(
    retry_item,
    action="createPage",
    stage="blockedBeforeWrite",
    status="retryNeeded",
    target="page-123",
    proof="notRequested",
  )
  if retry_attributes.get("connectorWorkflowRecovery") != "retry":
    raise AssertionError(f"Notion retry missed workflow recovery: {retry_attributes}")

  missing_parent_retry = connector.publish_page_draft(2, {"input": "{}"})
  missing_parent_item = response_first_item(missing_parent_retry)
  assert_workflow(
    missing_parent_item,
    action="createPage",
    stage="blockedBeforeWrite",
    status="retryNeeded",
    target="missingParentPageId",
    proof="notRequested",
  )
  assert_publish_success(connector)

  print("notion connector contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
