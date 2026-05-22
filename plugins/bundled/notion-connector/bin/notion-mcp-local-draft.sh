#!/bin/sh
set -eu

while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"pith-notion-local-draft","version":"0.1.0"}}}'
      ;;
    *'"method":"tools/call"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Page Draft","content":"Prepared a credential-scoped local Notion page draft. Review the title, summary, and next actions before any remote Notion update.","attributes":{"connector":"notion","mode":"localDraft","networkWrite":"false","credentialScoped":"true","mcpTool":"notion.preparePageDraft"}}],"memoryNotes":[{"title":"Notion Draft Prepared","body":"The Notion connector prepared a credential-scoped local page draft without a remote write.","source":"plugin.notion-connector","tags":["connector","notion","local-draft"]}]}}}'
      ;;
  esac
done
