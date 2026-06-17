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
VALID_PARENT_PAGE_ID = "11112222-3333-4444-5555-666677778888"
VALID_PARENT_PAGE_URL = "https://www.notion.so/Amentia-Test-11112222333344445555666677778888?pvs=4"


def load_connector() -> ModuleType:
  spec = importlib.util.spec_from_file_location("amentia_notion_connector", CONNECTOR_PATH)
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
  manifest = json.loads((PLUGIN_ROOT / "amentia-plugin.json").read_text(encoding="utf-8"))
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


def assert_publish_success(
  connector: ModuleType,
  *,
  parent_input: str = VALID_PARENT_PAGE_ID,
  expected_parent_id: str = VALID_PARENT_PAGE_ID,
  raw_input: str | None = None,
  expected_title: str = "Published by test",
  expected_payload_title: str | None = None,
  expected_block_count: str = "5",
  expected_title_truncated: str = "false",
  expected_body_truncated: str = "false",
) -> None:
  captured: dict[str, object] = {}
  previous_token = os.environ.get("AMENTIA_TEST_NOTION_TOKEN")
  os.environ["AMENTIA_TEST_NOTION_TOKEN"] = "secret-token"

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
        "input": raw_input
        if raw_input is not None
        else json.dumps(
          {
            "parentPageId": parent_input,
            "title": expected_title,
            "body": "\n".join(
              [
                "# Decision Log",
                "",
                "Paragraph one.",
                "",
                "- First action",
                "- [x] Completed task",
                "1. Numbered follow-up",
              ]
            ),
          },
          sort_keys=True,
        ),
        "connectors": [
          {
            "service": "notion",
            "credentialProvider": {
              "provider": "env",
              "envKey": "AMENTIA_TEST_NOTION_TOKEN",
            },
          }
        ],
      },
    )
  finally:
    if previous_token is None:
      os.environ.pop("AMENTIA_TEST_NOTION_TOKEN", None)
    else:
      os.environ["AMENTIA_TEST_NOTION_TOKEN"] = previous_token
  item = response_first_item(response)
  structured = response.get("result", {}).get("structuredContent", {})
  attributes = item.get("attributes", {})
  expected = {
    "remoteWrite": "true",
    "remoteWriteStage": "completed",
    "remoteProofKind": "notionApiResponse",
    "remoteProofStatus": "success",
    "notionPageId": "page-success-123",
    "notionPageUrl": "https://www.notion.so/page-success-123",
    "notionParentPageId": expected_parent_id,
    "titleTruncated": expected_title_truncated,
    "bodyTruncated": expected_body_truncated,
    "notionBlockCount": expected_block_count,
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
  expected_payload_title = expected_payload_title or expected_title
  memory_notes = structured.get("memoryNotes", [])
  if not isinstance(memory_notes, list) or len(memory_notes) != 1:
    raise AssertionError(f"Notion publish missed memory note: {structured}")
  memory_body = memory_notes[0].get("body", "")
  expected_memory_parts = [
    expected_payload_title,
    "https://www.notion.so/page-success-123",
    expected_parent_id,
    f"Title truncated: {expected_title_truncated}.",
    f"Body truncated: {expected_body_truncated}.",
    f"Blocks: {expected_block_count}.",
  ]
  for expected_part in expected_memory_parts:
    if expected_part not in memory_body:
      raise AssertionError(f"Notion publish memory missed {expected_part!r}: {memory_notes}")
  payload = captured.get("payload")
  if not isinstance(payload, dict):
    raise AssertionError(f"Notion publish payload was not a dict: {captured}")
  if payload.get("parent", {}).get("page_id") != expected_parent_id:
    raise AssertionError(f"Notion publish used the wrong parent: {payload}")
  title = payload.get("properties", {}).get("title", {}).get("title", [])
  if not title or title[0].get("text", {}).get("content") != expected_payload_title:
    raise AssertionError(f"Notion publish used the wrong title: {payload}")
  expected_child_count = int(expected_block_count)
  children = payload.get("children", [])
  if expected_child_count == 0:
    if children:
      raise AssertionError(f"Notion publish should omit empty children: {payload}")
  elif not isinstance(children, list) or len(children) != expected_child_count:
    raise AssertionError(f"Notion publish used wrong child count: {payload}")
  if raw_input is None:
    child_types = [child.get("type") for child in children]
    expected_child_types = [
      "heading_1",
      "paragraph",
      "bulleted_list_item",
      "to_do",
      "numbered_list_item",
    ]
    if child_types != expected_child_types:
      raise AssertionError(f"Notion publish used wrong block types: {child_types}")
    if children[3].get("to_do", {}).get("checked") is not True:
      raise AssertionError(f"Notion publish missed todo completion state: {children[3]}")


def assert_publish_missing_remote_proof_retry(connector: ModuleType) -> None:
  previous_token = os.environ.get("AMENTIA_TEST_NOTION_TOKEN")
  os.environ["AMENTIA_TEST_NOTION_TOKEN"] = "secret-token"

  def fake_notion_post(_path: str, _token: str, _payload: dict) -> dict:
    return {
      "id": "page-without-trusted-url",
      "url": "http://www.notion.so/page-without-trusted-url",
    }

  connector.notion_post = fake_notion_post
  try:
    response = connector.publish_page_draft(
      4,
      {
        "input": json.dumps(
          {
            "parentPageId": VALID_PARENT_PAGE_ID,
            "title": "Needs proof",
            "body": "The response is missing trusted proof.",
          },
          sort_keys=True,
        ),
        "connectors": [
          {
            "service": "notion",
            "credentialProvider": {
              "provider": "env",
              "envKey": "AMENTIA_TEST_NOTION_TOKEN",
            },
          }
        ],
      },
    )
  finally:
    if previous_token is None:
      os.environ.pop("AMENTIA_TEST_NOTION_TOKEN", None)
    else:
      os.environ["AMENTIA_TEST_NOTION_TOKEN"] = previous_token

  item = response_first_item(response)
  attributes = item.get("attributes", {})
  assert_workflow(
    item,
    action="createPage",
    stage="failedBeforeProof",
    status="retryNeeded",
    target=VALID_PARENT_PAGE_ID,
    proof="missing",
  )
  expected = {
    "remoteWrite": "false",
    "remoteWriteStage": "failedBeforeProof",
    "remoteProofStatus": "missing",
    "publishFailureReason": "missingRemoteProof",
    "retryInputEditable": "false",
  }
  for key, value in expected.items():
    if attributes.get(key) != value:
      raise AssertionError(f"Expected missing proof {key}={value!r}, got {attributes}")
  if "may have completed" not in attributes.get("retryInputHint", ""):
    raise AssertionError(f"Missing proof retry should warn about duplicate risk: {attributes}")


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
          "parentPageId": VALID_PARENT_PAGE_ID,
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
  if retry_input.get("parentPageId") != VALID_PARENT_PAGE_ID:
    raise AssertionError(f"Notion retry did not preserve publish input: {retry_input}")
  assert_workflow(
    retry_item,
    action="createPage",
    stage="blockedBeforeWrite",
    status="retryNeeded",
    target=VALID_PARENT_PAGE_ID,
    proof="notRequested",
  )
  if retry_attributes.get("connectorWorkflowRecovery") != "retry":
    raise AssertionError(f"Notion retry missed workflow recovery: {retry_attributes}")
  if retry_attributes.get("retryInputEditable") != "false":
    raise AssertionError(f"Notion credential retry should not force input editing: {retry_attributes}")

  invalid_parent_called_remote = False
  previous_token = os.environ.get("AMENTIA_TEST_NOTION_TOKEN")
  os.environ["AMENTIA_TEST_NOTION_TOKEN"] = "secret-token"

  def fail_if_invalid_parent_reaches_remote(_path: str, _token: str, _payload: dict) -> dict:
    nonlocal invalid_parent_called_remote
    invalid_parent_called_remote = True
    raise AssertionError("Invalid Notion parent should be blocked before the API call")

  connector.notion_post = fail_if_invalid_parent_reaches_remote
  try:
    invalid_parent_retry = connector.publish_page_draft(
      2,
      {
        "input": json.dumps(
          {
            "parentPageId": "not-a-notion-page",
            "title": "Fix my target",
            "body": "This should be blocked before any remote write.",
          },
          sort_keys=True,
        ),
        "connectors": [
          {
            "service": "notion",
            "credentialProvider": {
              "provider": "env",
              "envKey": "AMENTIA_TEST_NOTION_TOKEN",
            },
          }
        ],
      },
    )
  finally:
    if previous_token is None:
      os.environ.pop("AMENTIA_TEST_NOTION_TOKEN", None)
    else:
      os.environ["AMENTIA_TEST_NOTION_TOKEN"] = previous_token
  invalid_parent_item = response_first_item(invalid_parent_retry)
  invalid_parent_attributes = invalid_parent_item.get("attributes", {})
  if invalid_parent_called_remote:
    raise AssertionError("Notion invalid parent reached the remote API path")
  assert_workflow(
    invalid_parent_item,
    action="createPage",
    stage="blockedBeforeWrite",
    status="retryNeeded",
    target="not-a-notion-page",
    proof="notRequested",
  )
  if invalid_parent_attributes.get("publishFailureReason") != "invalidParentPageId":
    raise AssertionError(
      f"Notion invalid parent retry missed failure reason: {invalid_parent_attributes}"
    )
  if invalid_parent_attributes.get("retryInputEditable") != "true":
    raise AssertionError(
      f"Notion invalid parent retry should reopen editable input: {invalid_parent_attributes}"
    )
  if "32-character Notion page ID" not in invalid_parent_attributes.get("retryInputHint", ""):
    raise AssertionError(
      f"Notion invalid parent retry missed precise guidance: {invalid_parent_attributes}"
    )

  missing_parent_retry = connector.publish_page_draft(2, {"input": "{}"})
  missing_parent_item = response_first_item(missing_parent_retry)
  missing_parent_attributes = missing_parent_item.get("attributes", {})
  assert_workflow(
    missing_parent_item,
    action="createPage",
    stage="blockedBeforeWrite",
    status="retryNeeded",
    target="missingParentPageId",
    proof="notRequested",
  )
  if missing_parent_attributes.get("retryInputEditable") != "true":
    raise AssertionError(
      f"Notion missing parent retry should reopen editable input: {missing_parent_attributes}"
    )
  missing_parent_retry_input = json.loads(missing_parent_attributes.get("retryInput", "{}"))
  for key in ["parentPageId", "title", "body"]:
    if key not in missing_parent_retry_input:
      raise AssertionError(
        f"Notion missing parent retry missed editable {key}: {missing_parent_retry_input}"
      )
  if "parentPageId" not in missing_parent_attributes.get("retryInputHint", ""):
    raise AssertionError(
      f"Notion missing parent retry missed input guidance: {missing_parent_attributes}"
    )
  assert_publish_success(connector)
  assert_publish_success(
    connector,
    parent_input=VALID_PARENT_PAGE_URL,
    expected_parent_id=VALID_PARENT_PAGE_ID,
  )
  assert_publish_success(
    connector,
    raw_input=VALID_PARENT_PAGE_URL,
    expected_parent_id=VALID_PARENT_PAGE_ID,
    expected_title="Amentia Draft",
    expected_block_count="0",
  )
  assert_publish_success(
    connector,
    raw_input="\n".join(
      [
        f"parent: {VALID_PARENT_PAGE_URL}",
        "title: Alias Parent",
        "body:",
        "- Captured from a forgiving input parser",
      ]
    ),
    expected_parent_id=VALID_PARENT_PAGE_ID,
    expected_title="Alias Parent",
    expected_block_count="1",
  )
  assert_publish_success(
    connector,
    raw_input="\n".join(
      [
        f"notion_url: {VALID_PARENT_PAGE_URL}",
        "title: Inline Body",
        "body: Inline first paragraph",
        "- Inline follow-up",
      ]
    ),
    expected_parent_id=VALID_PARENT_PAGE_ID,
    expected_title="Inline Body",
    expected_block_count="2",
  )
  long_title = "Amentia Long Title " * 20
  assert_publish_success(
    connector,
    raw_input=json.dumps(
      {
        "parentPageId": VALID_PARENT_PAGE_ID,
        "title": long_title,
        "body": "Long title proof should reflect the actual payload title.",
      },
      sort_keys=True,
    ),
    expected_title=long_title,
    expected_payload_title=long_title[:200],
    expected_title_truncated="true",
    expected_block_count="1",
  )
  assert_publish_success(
    connector,
    raw_input=json.dumps(
      {
        "parentPageId": VALID_PARENT_PAGE_ID,
        "title": "Block Truncation",
        "body": "\n".join(f"- Item {index}" for index in range(25)),
      },
      sort_keys=True,
    ),
    expected_title="Block Truncation",
    expected_block_count="20",
    expected_body_truncated="true",
  )
  assert_publish_missing_remote_proof_retry(connector)

  print("notion connector contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
