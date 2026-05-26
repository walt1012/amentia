#!/usr/bin/env python3
"""Small local MCP server for Pith's bundled Notion connector."""

from __future__ import annotations

import json
import os
import re
import sys
import urllib.error
import urllib.request
from typing import Any


NOTION_API_BASE = os.environ.get(
  "PITH_TEST_NOTION_API_BASE",
  "https://api.notion.com/v1",
).rstrip("/")
NOTION_VERSION = os.environ.get("PITH_NOTION_VERSION", "2026-03-11")
HTTP_TIMEOUT_SECONDS = 30
MAX_BODY_CHARS = 20_000
MAX_PARAGRAPHS = 20
RICH_TEXT_CHUNK_SIZE = 1_900
DEFAULT_PUBLISH_COMMAND_ID = "notion-connector::notion.publish-page-draft"


def main() -> int:
  for line in sys.stdin:
    try:
      request = json.loads(line)
    except json.JSONDecodeError:
      continue
    method = request.get("method")
    if method == "initialize":
      write_response(initialize_response(request.get("id")))
    elif method == "tools/call":
      write_response(handle_tool_call(request))
  return 0


def initialize_response(request_id: Any) -> dict[str, Any]:
  return {
    "jsonrpc": "2.0",
    "id": request_id,
    "result": {
      "protocolVersion": "2025-06-18",
      "capabilities": {"tools": {}},
      "serverInfo": {"name": "pith-notion-local-connector", "version": "0.2.0"},
    },
  }


def handle_tool_call(request: dict[str, Any]) -> dict[str, Any]:
  request_id = request.get("id")
  params = request.get("params") or {}
  tool_name = params.get("name")
  arguments = params.get("arguments") or {}
  if tool_name == "preparePageDraft":
    return mcp_success(request_id, prepare_page_draft(arguments))
  if tool_name == "inspectPageWrite":
    return mcp_success(request_id, inspect_page_write(arguments))
  if tool_name == "publishPageDraft":
    return publish_page_draft(request_id, arguments)
  return mcp_error(request_id, -32601, f"Unknown Notion tool: {tool_name}")


def prepare_page_draft(arguments: dict[str, Any]) -> dict[str, Any]:
  source = source_artifact(arguments)
  preview = saved_artifact_preview(arguments)
  truncated = saved_artifact_truncated(arguments)
  read_error = saved_artifact_read_error(arguments)
  if source != "workspace" and preview:
    content = (
      f"Prepared a credential-scoped local Notion page draft from saved artifact {source}. "
      f"Preview: {preview} Review the title, summary, and next actions before any "
      "remote Notion update."
    )
  elif source != "workspace":
    content = (
      f"Found saved artifact reference {source}, but no bounded preview was available. "
      f"{read_error} Review the workspace context before any remote Notion update."
    )
  else:
    content = (
      "Prepared a credential-scoped local Notion page draft. Review the title, summary, "
      "and next actions before any remote Notion update."
    )
  if source != "workspace" and preview:
    memory_body = (
      f"The Notion connector prepared a credential-scoped local page draft from saved "
      f"artifact {source} using bounded preview: {preview}"
    )
  elif source != "workspace":
    memory_body = (
      f"The Notion connector found saved artifact {source} but did not receive a "
      "bounded preview, so no remote write was attempted."
    )
  else:
    memory_body = (
      "The Notion connector prepared a credential-scoped local page draft without a remote write."
    )
  draft_title = "Pith Notion Draft" if source == "workspace" else f"Pith draft from {source}"
  return {
    "items": [
      plugin_result(
        "Notion Page Draft",
        content,
        {
          "targetService": "notion",
          "draftMode": "localDraft",
          "remoteWrite": "false",
          "credentialScoped": "true",
          "targetTool": "notion.preparePageDraft",
          "sourceArtifact": source,
          "sourceArtifactPreviewProvided": str(bool(preview)).lower(),
          "sourceArtifactTruncated": truncated,
          **connector_workflow_attributes(
            stage="draftPrepared",
            status="prepared",
            action="createPage",
            target=source,
            proof="localDraft",
          ),
          **publish_follow_up_attributes(draft_title, content),
        },
      )
    ],
    "memoryNotes": [
      memory_note(
        "Notion Draft Prepared",
        memory_body,
        ["connector", "notion", "local-draft"],
      )
    ],
  }


def inspect_page_write(arguments: dict[str, Any]) -> dict[str, Any]:
  source = source_artifact(arguments)
  preview = saved_artifact_preview(arguments)
  preview_provided = bool(preview)
  if source != "workspace":
    content = (
      f"Inspected a proposed Notion remote write from saved artifact {source}. "
      "No remote write was sent. Review the source, target service, and "
      "credential scope before running the publish command."
    )
  else:
    content = (
      "Inspected a proposed Notion remote write from workspace context. No remote "
      "write was sent. Review the target page, content, and credential scope before "
      "running the publish command."
    )
  draft_title = "Pith Notion Publish" if source == "workspace" else f"Pith publish from {source}"
  return {
    "items": [
      plugin_result(
        "Notion Remote Write Inspection",
        content,
        {
          "targetService": "notion",
          "draftMode": "remoteWriteInspection",
          "remoteWrite": "false",
          "remoteWriteStage": "inspectBeforeWrite",
          "remoteWriteRequiresApproval": "true",
          "credentialScoped": "true",
          "targetTool": "notion.inspectPageWrite",
          "sourceArtifact": source,
          "sourceArtifactPreviewProvided": str(preview_provided).lower(),
          **connector_workflow_attributes(
            stage="inspectBeforeWrite",
            status="inspected",
            action="createPage",
            target=source,
            proof="inspection",
          ),
          **publish_follow_up_attributes(draft_title, content),
        },
      )
    ],
    "memoryNotes": [
      memory_note(
        "Notion Write Inspected",
        f"The Notion connector inspected a proposed remote write from {source}. No remote write was sent.",
        ["connector", "notion", "inspection"],
      )
    ],
  }


def publish_page_draft(request_id: Any, arguments: dict[str, Any]) -> dict[str, Any]:
  input_data = parse_publish_input(arguments.get("input"))
  parent_page_id = string_value(
    input_data,
    "parentPageId",
    "parent_page_id",
    "pageId",
    "page_id",
    "targetPageId",
    "target_page_id",
  )
  if not parent_page_id:
    return publish_retry_needed(
      request_id,
      input_data,
      "Publish requires `parentPageId` in the command input. No remote write was sent.",
      "missingParentPageId",
    )

  token = notion_token(arguments)
  if not token:
    return publish_retry_needed(
      request_id,
      input_data,
      "Publish requires an authorized Notion integration token. No remote write was sent.",
      "missingCredential",
    )

  title = string_value(input_data, "title", "pageTitle", "page_title") or "Pith Draft"
  body = string_value(input_data, "body", "content", "markdown", "text") or ""
  body, truncated = bounded_body(body)
  payload = notion_create_page_payload(parent_page_id, title, body)

  try:
    response = notion_post("/pages", token, payload)
  except NotionApiError as error:
    return publish_retry_needed(
      request_id,
      input_data,
      error.safe_message,
      error.reason,
    )

  page_id = str(response.get("id", ""))
  page_url = str(response.get("url", ""))
  content = f"Created Notion page `{title}`"
  if page_url:
    content += f" at {page_url}"
  content += "."
  return mcp_success(
    request_id,
    {
      "items": [
        plugin_result(
          "Notion Page Published",
          content,
          {
            "targetService": "notion",
            "draftMode": "remoteWrite",
            "remoteWrite": "true",
            "remoteWriteStage": "completed",
            "remoteWriteRequiresApproval": "true",
            "credentialScoped": "true",
            "targetTool": "notion.createPage",
            "notionPageId": page_id,
            "notionPageUrl": page_url,
            "notionParentPageId": parent_page_id,
            "remoteProofKind": "notionApiResponse",
            "remoteProofStatus": "success",
            "bodyTruncated": str(truncated).lower(),
            **connector_workflow_attributes(
              stage="completed",
              status="completed",
              action="createPage",
              target=page_url or page_id,
              proof="notionApiResponse",
            ),
          },
        )
      ],
      "memoryNotes": [
        memory_note(
          "Notion Page Published",
          f"Pith created Notion page `{title}` under parent page {parent_page_id}.",
          ["connector", "notion", "published"],
        )
      ],
    },
  )


def notion_post(path: str, token: str, payload: dict[str, Any]) -> dict[str, Any]:
  request = urllib.request.Request(
    f"{NOTION_API_BASE}{path}",
    data=json.dumps(payload).encode("utf-8"),
    method="POST",
    headers={
      "Authorization": f"Bearer {token}",
      "Content-Type": "application/json",
      "Notion-Version": NOTION_VERSION,
      "User-Agent": "Pith/0.1",
    },
  )
  try:
    with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
      return json.loads(response.read().decode("utf-8"))
  except urllib.error.HTTPError as error:
    detail = bounded_error_detail(error.read().decode("utf-8", errors="replace"))
    message = (
      f"Notion create page failed with HTTP {error.code}: {detail}. "
      "No remote write proof was accepted."
    )
    raise NotionApiError(
      message,
      f"http{error.code}",
    ) from error
  except urllib.error.URLError as error:
    detail = bounded_error_detail(str(error))
    message = (
      f"Notion create page failed before completion: {detail}. "
      "No remote write proof was accepted."
    )
    raise NotionApiError(
      message,
      "connectionError",
    ) from error


class NotionApiError(Exception):
  def __init__(self, safe_message: str, reason: str) -> None:
    super().__init__(safe_message)
    self.safe_message = safe_message
    self.reason = reason


def notion_create_page_payload(parent_page_id: str, title: str, body: str) -> dict[str, Any]:
  payload: dict[str, Any] = {
    "parent": {"page_id": parent_page_id},
    "properties": {
      "title": {
        "title": [{"type": "text", "text": {"content": title[:200]}}],
      }
    },
  }
  children = notion_children(body)
  if children:
    payload["children"] = children
  return payload


def publish_retry_needed(
  request_id: Any,
  input_data: dict[str, str],
  reason: str,
  reason_code: str,
) -> dict[str, Any]:
  parent_page_id = string_value(
    input_data,
    "parentPageId",
    "parent_page_id",
    "pageId",
    "page_id",
    "targetPageId",
    "target_page_id",
  ) or ""
  blocked_before_write = reason_code in {"missingParentPageId", "missingCredential"}
  content = (
    f"{reason} Review the target page, credential, and content, then run "
    "`notion.publish-page-draft` again when you are ready."
  )
  attributes = {
    "targetService": "notion",
    "draftMode": "remoteWrite",
    "remoteWrite": "false",
    "remoteWriteStage": "blockedBeforeWrite" if blocked_before_write else "failedBeforeProof",
    "remoteWriteRequiresApproval": "true",
    "credentialScoped": "true",
    "targetTool": "notion.createPage",
    "remoteProofKind": "notionApiResponse",
    "remoteProofStatus": "notRequested" if blocked_before_write else "missing",
    "publishRetryable": "true",
    "publishFailureReason": reason_code,
    "retryCommand": "notion.publish-page-draft",
    "retryCommandId": os.environ.get("PITH_PLUGIN_COMMAND_ID", DEFAULT_PUBLISH_COMMAND_ID),
    "retryInput": retry_input_json(input_data),
    "notionParentPageId": parent_page_id,
    **connector_workflow_attributes(
      stage="blockedBeforeWrite" if blocked_before_write else "failedBeforeProof",
      status="retryNeeded",
      action="createPage",
      target=parent_page_id or reason_code,
      proof="notRequested" if blocked_before_write else "missing",
      recovery="retry",
    ),
  }
  return mcp_success(
    request_id,
    {
      "items": [
        plugin_result(
          "Notion Publish Needs Retry",
          content,
          attributes,
        )
      ],
      "memoryNotes": [
        memory_note(
          "Notion Publish Retry Needed",
          f"Pith could not confirm a Notion page publish. Reason: {reason_code}.",
          ["connector", "notion", "retry-needed"],
        )
      ],
    },
  )


def retry_input_json(input_data: dict[str, str]) -> str:
  return json.dumps(input_data, ensure_ascii=False, sort_keys=True)


def connector_workflow_attributes(
  *,
  stage: str,
  status: str,
  action: str,
  target: str,
  proof: str,
  recovery: str = "",
) -> dict[str, str]:
  attributes = {
    "connectorWorkflowId": "notion.create-page",
    "connectorWorkflowName": "Notion Create Page",
    "connectorWorkflowService": "notion",
    "connectorWorkflowAction": action,
    "connectorWorkflowStage": stage,
    "connectorWorkflowStatus": status,
    "connectorWorkflowTarget": target,
    "connectorWorkflowProof": proof,
  }
  if recovery:
    attributes["connectorWorkflowRecovery"] = recovery
  return attributes


def publish_follow_up_attributes(title: str, body: str) -> dict[str, str]:
  return {
    "nextCommand": "notion.publish-page-draft",
    "nextCommandId": DEFAULT_PUBLISH_COMMAND_ID,
    "nextCommandLabel": "Publish to Notion",
    "nextCommandInputHint": (
      "Fill parentPageId before publishing. Pith will request approval before any "
      "remote Notion write."
    ),
    "nextCommandInputTemplate": publish_input_template(title, body),
    "nextCommandInputRequired": "true",
  }


def publish_input_template(title: str, body: str) -> str:
  body, _ = bounded_body(body)
  return json.dumps(
    {
      "parentPageId": "",
      "title": title[:200],
      "body": body,
    },
    ensure_ascii=False,
    indent=2,
  )


def notion_children(body: str) -> list[dict[str, Any]]:
  paragraphs = [part.strip() for part in re.split(r"\n\s*\n", body) if part.strip()]
  return [
    {
      "object": "block",
      "type": "paragraph",
      "paragraph": {"rich_text": rich_text_chunks(paragraph)},
    }
    for paragraph in paragraphs[:MAX_PARAGRAPHS]
  ]


def rich_text_chunks(text: str) -> list[dict[str, Any]]:
  return [
    {"type": "text", "text": {"content": text[index:index + RICH_TEXT_CHUNK_SIZE]}}
    for index in range(0, len(text), RICH_TEXT_CHUNK_SIZE)
  ] or [{"type": "text", "text": {"content": ""}}]


def parse_publish_input(raw_input: Any) -> dict[str, str]:
  if isinstance(raw_input, dict):
    return {str(key): str(value) for key, value in raw_input.items() if value is not None}
  if raw_input is None:
    return {}
  text = str(raw_input).strip()
  if not text:
    return {}
  try:
    parsed = json.loads(text)
  except json.JSONDecodeError:
    parsed = None
  if isinstance(parsed, dict):
    return {str(key): str(value) for key, value in parsed.items() if value is not None}
  parsed_text = parse_key_value_text(text)
  if parsed_text:
    return parsed_text
  return {"body": text}


def parse_key_value_text(text: str) -> dict[str, str]:
  data: dict[str, str] = {}
  body_lines: list[str] = []
  body_mode = False
  for line in text.splitlines():
    match = re.match(r"^\s*([A-Za-z][A-Za-z0-9_-]{1,40})\s*[:=]\s*(.*)$", line)
    if match:
      key = normalize_input_key(match.group(1))
      value = match.group(2).strip()
      if key == "body" and not value:
        body_mode = True
        continue
      data[key] = value
      body_mode = key == "body"
      continue
    if body_mode:
      body_lines.append(line)
  if body_lines:
    data["body"] = "\n".join(body_lines).strip()
  return data


def normalize_input_key(key: str) -> str:
  mapping = {
    "parentpageid": "parentPageId",
    "parent_page_id": "parentPageId",
    "pageid": "pageId",
    "page_id": "pageId",
    "targetpageid": "targetPageId",
    "target_page_id": "targetPageId",
    "pagetitle": "pageTitle",
    "page_title": "pageTitle",
    "content": "content",
    "markdown": "markdown",
    "text": "text",
    "body": "body",
    "title": "title",
  }
  compact = key.replace("-", "_")
  return mapping.get(compact.lower(), key)


def notion_token(arguments: dict[str, Any]) -> str | None:
  for connector in arguments.get("connectors") or []:
    if connector.get("service") != "notion":
      continue
    provider = connector.get("credentialProvider") or {}
    env_key = provider.get("envKey")
    if env_key:
      token = os.environ.get(str(env_key), "").strip()
      if token:
        return token
  return None


def source_artifact(arguments: dict[str, Any]) -> str:
  text = str(arguments.get("input") or "")
  match = re.search(r"Saved artifact: ([A-Za-z0-9._/-][A-Za-z0-9._/-]*)", text)
  return match.group(1) if match else "workspace"


def saved_artifact_preview(arguments: dict[str, Any]) -> str:
  text = str(arguments.get("input") or "")
  match = re.search(r"Saved artifact preview: ([^\n]+)", text)
  return match.group(1) if match else ""


def saved_artifact_truncated(arguments: dict[str, Any]) -> str:
  text = str(arguments.get("input") or "")
  match = re.search(r"Saved artifact truncated: ([^\n]+)", text)
  return match.group(1).strip() if match else "false"


def saved_artifact_read_error(arguments: dict[str, Any]) -> str:
  text = str(arguments.get("input") or "")
  match = re.search(r"Saved artifact read error: ([^\n]+)", text)
  return match.group(1).strip() if match else ""


def string_value(data: dict[str, str], *keys: str) -> str | None:
  for key in keys:
    value = data.get(key)
    if value and value.strip():
      return value.strip()
  return None


def bounded_body(body: str) -> tuple[str, bool]:
  if len(body) <= MAX_BODY_CHARS:
    return body, False
  return body[:MAX_BODY_CHARS], True


def bounded_error_detail(value: str) -> str:
  text = value.strip().replace("\n", " ")
  return text[:500] if text else "no response detail"


def plugin_result(title: str, content: str, attributes: dict[str, str]) -> dict[str, Any]:
  return {
    "kind": "pluginResult",
    "title": title,
    "content": content,
    "attributes": attributes,
  }


def memory_note(title: str, body: str, tags: list[str]) -> dict[str, Any]:
  return {
    "title": title,
    "body": body,
    "source": "plugin.notion-connector",
    "tags": tags,
  }


def mcp_success(request_id: Any, structured_content: dict[str, Any]) -> dict[str, Any]:
  return {
    "jsonrpc": "2.0",
    "id": request_id,
    "result": {
      "content": [],
      "structuredContent": structured_content,
    },
  }


def mcp_tool_error(request_id: Any, message: str) -> dict[str, Any]:
  return {
    "jsonrpc": "2.0",
    "id": request_id,
    "result": {
      "content": [{"type": "text", "text": message}],
      "isError": True,
    },
  }


def mcp_error(request_id: Any, code: int, message: str) -> dict[str, Any]:
  return {
    "jsonrpc": "2.0",
    "id": request_id,
    "error": {"code": code, "message": message},
  }


def write_response(response: dict[str, Any]) -> None:
  print(json.dumps(response, separators=(",", ":")), flush=True)


if __name__ == "__main__":
  raise SystemExit(main())
