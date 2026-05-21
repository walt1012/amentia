#!/bin/sh
set -eu

while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"pith-notion-dry-run","version":"0.1.0"}}}'
      ;;
    *'"method":"tools/call"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Page Draft","content":"Prepared a local Notion page draft. Review the title, summary, and next actions before sending anything to Notion.","attributes":{"connector":"notion","mode":"dryRun","networkWrite":"false"}}],"memoryNotes":[{"title":"Notion Draft Prepared","body":"The Notion connector prepared a local dry-run page draft.","source":"plugin.notion-connector","tags":["connector","notion","dry-run"]}]}}}'
      ;;
  esac
done
