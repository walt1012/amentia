#!/usr/bin/env python3
"""Local dry-run MCP server for the bundled Notion connector."""

from __future__ import annotations

import json
import sys


def main() -> int:
  for line in sys.stdin:
    line = line.strip()
    if not line:
      continue
    try:
      request = json.loads(line)
    except json.JSONDecodeError:
      continue

    method = request.get("method")
    if method == "initialize":
      emit(
        {
          "jsonrpc": "2.0",
          "id": request.get("id"),
          "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "pith-notion-dry-run", "version": "0.1.0"},
          },
        }
      )
    elif method == "tools/call":
      emit(tool_response(request))
  return 0


def tool_response(request: dict) -> dict:
  params = request.get("params") if isinstance(request.get("params"), dict) else {}
  tool_name = params.get("name")
  if tool_name != "preparePageDraft":
    return {
      "jsonrpc": "2.0",
      "id": request.get("id"),
      "error": {"code": -32601, "message": f"Unknown Notion dry-run tool: {tool_name}"},
    }

  envelope = {
    "items": [
      {
        "kind": "pluginResult",
        "title": "Notion Page Draft",
        "content": (
          "Prepared a local Notion page draft. Review the title, summary, and "
          "next actions before sending anything to Notion."
        ),
        "attributes": {
          "connector": "notion",
          "mode": "dryRun",
          "networkWrite": "false",
        },
      }
    ],
    "memoryNotes": [
      {
        "title": "Notion Draft Prepared",
        "body": "The Notion connector prepared a local dry-run page draft.",
        "source": "plugin.notion-connector",
        "tags": ["connector", "notion", "dry-run"],
      }
    ],
  }
  return {
    "jsonrpc": "2.0",
    "id": request.get("id"),
    "result": {
      "content": [{"type": "text", "text": json.dumps(envelope, separators=(",", ":"))}],
      "structuredContent": envelope,
    },
  }


def emit(payload: dict) -> None:
  print(json.dumps(payload, separators=(",", ":")), flush=True)


if __name__ == "__main__":
  raise SystemExit(main())
