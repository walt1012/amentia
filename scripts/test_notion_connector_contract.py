#!/usr/bin/env python3
"""Contract checks for the bundled Notion connector MCP server."""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path
from types import ModuleType


ROOT = Path(__file__).resolve().parents[1]
CONNECTOR_PATH = (
  ROOT / "plugins" / "bundled" / "notion-connector" / "bin" / "notion-mcp-local-draft.py"
)
PUBLISH_COMMAND_ID = "notion-connector::notion.publish-page-draft"


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


def main() -> int:
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
  assert_publish_follow_up(first_item(draft))

  inspection = connector.inspect_page_write({"input": "Saved artifact: docs/handoff.md"})
  assert_publish_follow_up(first_item(inspection))

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

  print("notion connector contract tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
