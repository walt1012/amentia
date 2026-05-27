#!/usr/bin/env python3
"""Small local MCP server for Pith's bundled Slack connector."""

from __future__ import annotations

import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from typing import Any


SLACK_API_BASE = os.environ.get(
  "PITH_TEST_SLACK_API_BASE",
  "https://slack.com/api",
).rstrip("/")
HTTP_TIMEOUT_SECONDS = 30
MAX_MESSAGE_CHARS = 4_000
DEFAULT_POST_COMMAND_ID = "slack-connector::slack.post-message-draft"
WORKFLOW_ID = "slack.post-message"
WORKFLOW_NAME = "Slack Post Message"


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
      "serverInfo": {"name": "pith-slack-local-connector", "version": "0.1.0"},
    },
  }


def handle_tool_call(request: dict[str, Any]) -> dict[str, Any]:
  request_id = request.get("id")
  params = request.get("params") or {}
  tool_name = params.get("name")
  arguments = params.get("arguments") or {}
  if tool_name == "prepareMessageDraft":
    return mcp_success(request_id, prepare_message_draft(arguments))
  if tool_name == "inspectMessageSend":
    return mcp_success(request_id, inspect_message_send(arguments))
  if tool_name == "postMessageDraft":
    return post_message_draft(request_id, arguments)
  return mcp_error(request_id, -32601, f"Unknown Slack tool: {tool_name}")


def prepare_message_draft(arguments: dict[str, Any]) -> dict[str, Any]:
  source = source_artifact(arguments)
  preview = saved_artifact_preview(arguments)
  truncated = saved_artifact_truncated(arguments)
  if source != "workspace" and preview:
    content = (
      f"Prepared a credential-scoped local Slack message draft from saved artifact {source}. "
      f"Preview: {preview} Review the channel, audience, and wording before any "
      "remote Slack post."
    )
  elif source != "workspace":
    content = (
      f"Found saved artifact reference {source}, but no bounded preview was available. "
      "Review the workspace context before any remote Slack post."
    )
  else:
    content = (
      "Prepared a credential-scoped local Slack message draft. Review the channel, "
      "audience, and wording before any remote Slack post."
    )
  draft_text = default_message_body(source, preview)
  return {
    "items": [
      plugin_result(
        "Slack Message Draft",
        content,
        {
          "targetService": "slack",
          "draftMode": "localDraft",
          "remoteWrite": "false",
          "credentialScoped": "true",
          "targetTool": "slack.prepareMessageDraft",
          "sourceArtifact": source,
          "sourceArtifactPreviewProvided": str(bool(preview)).lower(),
          "sourceArtifactTruncated": truncated,
          **connector_workflow_attributes(
            stage="draftPrepared",
            status="prepared",
            action="postMessage",
            target=source,
            proof="localDraft",
          ),
          **post_follow_up_attributes(draft_text),
        },
      )
    ],
    "memoryNotes": [
      memory_note(
        "Slack Draft Prepared",
        (
          f"The Slack connector prepared a credential-scoped local message draft "
          f"from {source}. No remote write was attempted."
        ),
        ["connector", "slack", "local-draft"],
      )
    ],
  }


def inspect_message_send(arguments: dict[str, Any]) -> dict[str, Any]:
  source = source_artifact(arguments)
  preview = saved_artifact_preview(arguments)
  draft_text = default_message_body(source, preview)
  content = (
    f"Inspected a proposed Slack remote message from {source}. No remote write was sent. "
    "Review the channel ID, audience, and credential scope before running the post command."
  )
  return {
    "items": [
      plugin_result(
        "Slack Message Send Inspection",
        content,
        {
          "targetService": "slack",
          "draftMode": "remoteWriteInspection",
          "remoteWrite": "false",
          "remoteWriteStage": "inspectBeforeWrite",
          "remoteWriteRequiresApproval": "true",
          "credentialScoped": "true",
          "targetTool": "slack.inspectMessageSend",
          "sourceArtifact": source,
          "sourceArtifactPreviewProvided": str(bool(preview)).lower(),
          **connector_workflow_attributes(
            stage="inspectBeforeWrite",
            status="inspected",
            action="postMessage",
            target=source,
            proof="inspection",
          ),
          **post_follow_up_attributes(draft_text),
        },
      )
    ],
    "memoryNotes": [
      memory_note(
        "Slack Send Inspected",
        f"The Slack connector inspected a proposed remote message from {source}. No remote write was sent.",
        ["connector", "slack", "inspection"],
      )
    ],
  }


def post_message_draft(request_id: Any, arguments: dict[str, Any]) -> dict[str, Any]:
  input_data = parse_post_input(arguments.get("input"))
  channel_id = string_value(input_data, "channelId", "channel_id", "channel")
  if not channel_id:
    return post_retry_needed(
      request_id,
      input_data,
      "Slack post requires `channelId` in the command input. No remote write was sent.",
      "missingChannelId",
    )
  if not valid_slack_channel_id(channel_id):
    return post_retry_needed(
      request_id,
      input_data,
      "Slack post requires a valid channel ID such as C123, G123, or D123. No remote write was sent.",
      "invalidChannelId",
    )

  raw_text = string_value(input_data, "message", "text", "body", "content")
  if not raw_text:
    return post_retry_needed(
      request_id,
      input_data,
      "Slack post requires message text. No remote write was sent.",
      "missingMessageText",
    )
  message_text, message_truncated = bounded_message(raw_text)

  token = slack_token(arguments)
  if not token:
    return post_retry_needed(
      request_id,
      input_data,
      "Slack post requires an authorized bot token. No remote write was sent.",
      "missingCredential",
    )

  payload = {"channel": channel_id, "text": message_text}
  try:
    response = slack_post("/chat.postMessage", token, payload)
  except SlackApiError as error:
    return post_retry_needed(request_id, input_data, error.safe_message, error.reason)

  try:
    proof_channel, proof_ts = slack_response_message_proof(response)
  except SlackApiError as error:
    return post_retry_needed(request_id, input_data, error.safe_message, error.reason)

  proof_id = f"{proof_channel}:{proof_ts}"
  proof_url = slack_message_url(proof_channel, proof_ts)
  content = f"Posted Slack message to `{proof_channel}` with timestamp `{proof_ts}`."
  return mcp_success(
    request_id,
    {
      "items": [
        plugin_result(
          "Slack Message Posted",
          content,
          {
            "targetService": "slack",
            "draftMode": "remoteWrite",
            "remoteWrite": "true",
            "remoteWriteStage": "completed",
            "remoteWriteRequiresApproval": "true",
            "credentialScoped": "true",
            "targetTool": "slack.chatPostMessage",
            "slackChannelId": proof_channel,
            "slackMessageTs": proof_ts,
            "messageTruncated": str(message_truncated).lower(),
            "remoteProofKind": "slackApiResponse",
            "remoteProofStatus": "success",
            "remoteProofId": proof_id,
            "remoteProofUrl": proof_url,
            "remoteProofTitle": "Slack message sent",
            "remoteProofActionTitle": "Open Slack Message",
            **connector_workflow_attributes(
              stage="completed",
              status="completed",
              action="postMessage",
              target=proof_channel,
              proof="slackApiResponse",
            ),
          },
        )
      ],
      "memoryNotes": [
        memory_note(
          "Slack Message Posted",
          (
            f"Pith posted a Slack message to {proof_channel} with timestamp {proof_ts}. "
            f"Proof URL: {proof_url}. Message truncated: {str(message_truncated).lower()}."
          ),
          ["connector", "slack", "posted"],
        )
      ],
    },
  )


def slack_post(path: str, token: str, payload: dict[str, Any]) -> dict[str, Any]:
  request = urllib.request.Request(
    f"{SLACK_API_BASE}{path}",
    data=json.dumps(payload).encode("utf-8"),
    method="POST",
    headers={
      "Authorization": f"Bearer {token}",
      "Content-Type": "application/json; charset=utf-8",
      "User-Agent": "Pith/0.1",
    },
  )
  try:
    with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
      data = json.loads(response.read().decode("utf-8"))
  except urllib.error.HTTPError as error:
    detail = bounded_error_detail(error.read().decode("utf-8", errors="replace"))
    raise SlackApiError(
      f"Slack post failed with HTTP {error.code}: {detail}. No remote write proof was accepted.",
      f"http{error.code}",
    ) from error
  except urllib.error.URLError as error:
    detail = bounded_error_detail(str(error))
    raise SlackApiError(
      f"Slack post failed before completion: {detail}. No remote write proof was accepted.",
      "connectionError",
    ) from error

  if not isinstance(data, dict):
    raise SlackApiError("Slack post returned a non-object response. No remote write proof was accepted.", "invalidResponse")
  if data.get("ok") is not True:
    reason = str(data.get("error") or "slackError")
    raise SlackApiError(
      f"Slack post was rejected by the API: {bounded_error_detail(reason)}. No remote write proof was accepted.",
      reason,
    )
  return data


class SlackApiError(Exception):
  def __init__(self, safe_message: str, reason: str) -> None:
    super().__init__(safe_message)
    self.safe_message = safe_message
    self.reason = reason


def slack_response_message_proof(response: dict[str, Any]) -> tuple[str, str]:
  channel = str(response.get("channel") or "").strip()
  ts = str(response.get("ts") or "").strip()
  if not valid_slack_channel_id(channel) or not valid_slack_ts(ts):
    raise SlackApiError(
      "Slack post returned without trusted channel and timestamp proof. Check Slack before retrying because the remote write may have completed.",
      "missingRemoteProof",
    )
  return channel, ts


def post_retry_needed(
  request_id: Any,
  input_data: dict[str, str],
  message: str,
  reason_code: str,
) -> dict[str, Any]:
  retry_input = dict(input_data)
  retry_input.setdefault("channelId", "")
  retry_input.setdefault("message", "")
  return mcp_success(
    request_id,
    {
      "items": [
        plugin_result(
          "Slack Message Post Needs Retry",
          message,
          {
            "targetService": "slack",
            "draftMode": "remoteWrite",
            "remoteWrite": "false",
            "remoteWriteStage": "blockedBeforeWrite",
            "remoteWriteRequiresApproval": "true",
            "credentialScoped": "true",
            "targetTool": "slack.chatPostMessage",
            "remoteProofKind": "slackApiResponse",
            "remoteProofStatus": "missing",
            "publishFailureReason": reason_code,
            "retryCommandId": DEFAULT_POST_COMMAND_ID,
            "retryInput": json.dumps(retry_input, ensure_ascii=False, indent=2),
            "retryInputEditable": "true",
            "retryInputHint": retry_input_hint(reason_code),
            **connector_workflow_attributes(
              stage="blockedBeforeWrite",
              status="retryNeeded",
              action="postMessage",
              target=retry_input.get("channelId", "") or "missingChannel",
              proof="missing",
              recovery=retry_input_hint(reason_code),
            ),
          },
        )
      ],
      "memoryNotes": [
        memory_note(
          "Slack Post Retry Needed",
          f"The Slack connector did not accept a remote proof. Reason: {reason_code}.",
          ["connector", "slack", "retry"],
        )
      ],
    },
  )


def retry_input_hint(reason_code: str) -> str:
  if reason_code == "missingChannelId":
    return "Add a Slack channel ID such as C123, G123, or D123, then retry after approval."
  if reason_code == "invalidChannelId":
    return "Replace channelId with a Slack channel, group, or direct-message ID."
  if reason_code == "missingMessageText":
    return "Add message text, then retry after approval."
  if reason_code == "missingCredential":
    return "Authorize the Slack connector with a bot token, then retry with the preserved input."
  if reason_code == "missingRemoteProof":
    return "Check Slack before retrying because the remote write may have completed without trusted proof."
  return "Review the preserved input and retry after the remote issue is resolved."


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
    "connectorWorkflowId": WORKFLOW_ID,
    "connectorWorkflowName": WORKFLOW_NAME,
    "connectorWorkflowService": "slack",
    "connectorWorkflowAction": action,
    "connectorWorkflowStage": stage,
    "connectorWorkflowStatus": status,
    "connectorWorkflowTarget": target,
    "connectorWorkflowProof": proof,
  }
  if recovery:
    attributes["connectorWorkflowRecovery"] = recovery
  return attributes


def post_follow_up_attributes(message: str) -> dict[str, str]:
  return {
    "nextCommand": "slack.post-message-draft",
    "nextCommandId": DEFAULT_POST_COMMAND_ID,
    "nextCommandLabel": "Post to Slack",
    "nextCommandInputHint": (
      "Fill channelId with a Slack channel ID and message with the approved text. "
      "Pith will request approval before any remote Slack write."
    ),
    "nextCommandInputTemplate": json.dumps(
      {
        "channelId": "",
        "message": message,
      },
      ensure_ascii=False,
      indent=2,
    ),
    "nextCommandInputRequired": "true",
  }


def parse_post_input(raw_input: Any) -> dict[str, str]:
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
  return {"message": text}


def parse_key_value_text(text: str) -> dict[str, str]:
  data: dict[str, str] = {}
  message_lines: list[str] = []
  message_mode = False
  for line in text.splitlines():
    if message_mode:
      message_lines.append(line)
      continue
    match = re.match(r"^\s*([A-Za-z][A-Za-z0-9_-]{1,40})\s*[:=]\s*(.*)$", line)
    if not match:
      continue
    key = normalize_input_key(match.group(1))
    value = match.group(2).strip()
    if key == "message":
      if value:
        message_lines.append(value)
      message_mode = True
      continue
    data[key] = value
  if message_lines:
    data["message"] = "\n".join(message_lines).strip()
  return data


def normalize_input_key(key: str) -> str:
  mapping = {
    "channel": "channelId",
    "channelid": "channelId",
    "channel_id": "channelId",
    "room": "channelId",
    "message": "message",
    "text": "message",
    "body": "message",
    "content": "message",
  }
  compact = key.replace("-", "_")
  return mapping.get(compact.lower(), key)


def slack_token(arguments: dict[str, Any]) -> str | None:
  for connector in arguments.get("connectors") or []:
    if connector.get("service") != "slack":
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


def string_value(data: dict[str, str], *keys: str) -> str | None:
  for key in keys:
    value = data.get(key)
    if value and value.strip():
      return value.strip()
  return None


def default_message_body(source: str, preview: str) -> str:
  if source != "workspace" and preview:
    return f"Workspace update from {source}: {preview}"
  if source != "workspace":
    return f"Workspace update from {source}: please review the saved artifact before posting."
  return "Workspace update: please review the latest Pith context and next action."


def bounded_message(message: str) -> tuple[str, bool]:
  text = message.strip()
  if len(text) <= MAX_MESSAGE_CHARS:
    return text, False
  return text[:MAX_MESSAGE_CHARS], True


def bounded_error_detail(value: str) -> str:
  text = value.strip().replace("\n", " ")
  return text[:500] if text else "no response detail"


def valid_slack_channel_id(value: str) -> bool:
  return re.fullmatch(r"[CGD][A-Z0-9]{2,79}", value.strip()) is not None


def valid_slack_ts(value: str) -> bool:
  return re.fullmatch(r"\d{10,}\.\d{3,}", value.strip()) is not None


def slack_message_url(channel_id: str, ts: str) -> str:
  return (
    "https://slack.com/app_redirect?"
    + urllib.parse.urlencode({"channel": channel_id, "message_ts": ts})
  )


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
    "source": "plugin.slack-connector",
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
