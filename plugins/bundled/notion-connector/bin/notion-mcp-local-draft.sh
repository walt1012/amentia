#!/bin/sh
set -eu

while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"pith-notion-local-draft","version":"0.1.0"}}}'
      ;;
    *'"method":"tools/call"'*)
      source_artifact=$(printf '%s' "$line" | sed -n 's#.*Saved artifact: \([A-Za-z0-9._/-][A-Za-z0-9._/-]*\).*#\1#p')
      artifact_preview=$(printf '%s' "$line" | sed -n 's/.*Saved artifact preview: \([^\\]*\)\\nSaved artifact truncated:.*/\1/p')
      artifact_truncated=$(printf '%s' "$line" | sed -n 's/.*Saved artifact truncated: \([^\\"]*\).*/\1/p')
      artifact_read_error=$(printf '%s' "$line" | sed -n 's/.*Saved artifact read error: \([^\\"]*\).*/\1/p')
      preview_provided=false
      if [ -n "$artifact_preview" ]; then
        preview_provided=true
      fi
      if printf '%s' "$line" | grep -q '"name":"inspectPageWrite"'; then
        if [ -n "$source_artifact" ]; then
          printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Remote Write Inspection","content":"Inspected a proposed Notion remote write from saved artifact '"$source_artifact"'. No remote write was sent. Review the source, target service, and credential scope before enabling a write-capable connector runner.","attributes":{"targetService":"notion","draftMode":"remoteWriteInspection","remoteWrite":"false","remoteWriteStage":"inspectBeforeWrite","remoteWriteRequiresApproval":"true","credentialScoped":"true","targetTool":"notion.inspectPageWrite","sourceArtifact":"'"$source_artifact"'","sourceArtifactPreviewProvided":"'"$preview_provided"'"}}],"memoryNotes":[{"title":"Notion Write Inspected","body":"The Notion connector inspected a proposed remote write from saved artifact '"$source_artifact"'. No remote write was sent.","source":"plugin.notion-connector","tags":["connector","notion","inspection"]}]}}}'
        else
          printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Remote Write Inspection","content":"Inspected a proposed Notion remote write from workspace context. No remote write was sent. Review the target service and credential scope before enabling a write-capable connector runner.","attributes":{"targetService":"notion","draftMode":"remoteWriteInspection","remoteWrite":"false","remoteWriteStage":"inspectBeforeWrite","remoteWriteRequiresApproval":"true","credentialScoped":"true","targetTool":"notion.inspectPageWrite","sourceArtifact":"workspace","sourceArtifactPreviewProvided":"false"}}],"memoryNotes":[{"title":"Notion Write Inspected","body":"The Notion connector inspected a proposed remote write from workspace context. No remote write was sent.","source":"plugin.notion-connector","tags":["connector","notion","inspection"]}]}}}'
        fi
      elif [ -n "$source_artifact" ] && [ -n "$artifact_preview" ]; then
        printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Page Draft","content":"Prepared a credential-scoped local Notion page draft from saved artifact '"$source_artifact"'. Preview: '"$artifact_preview"' Review the title, summary, and next actions before any remote Notion update.","attributes":{"targetService":"notion","draftMode":"localDraft","remoteWrite":"false","credentialScoped":"true","targetTool":"notion.preparePageDraft","sourceArtifact":"'"$source_artifact"'","sourceArtifactPreviewProvided":"true","sourceArtifactTruncated":"'"$artifact_truncated"'"}}],"memoryNotes":[{"title":"Notion Draft Prepared","body":"The Notion connector prepared a credential-scoped local page draft from saved artifact '"$source_artifact"' using bounded preview: '"$artifact_preview"'","source":"plugin.notion-connector","tags":["connector","notion","local-draft"]}]}}}'
      elif [ -n "$source_artifact" ]; then
        printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Page Draft","content":"Found saved artifact reference '"$source_artifact"', but no bounded preview was available. '"$artifact_read_error"' Review the workspace context before any remote Notion update.","attributes":{"targetService":"notion","draftMode":"localDraft","remoteWrite":"false","credentialScoped":"true","targetTool":"notion.preparePageDraft","sourceArtifact":"'"$source_artifact"'","sourceArtifactPreviewProvided":"false"}}],"memoryNotes":[{"title":"Notion Draft Prepared","body":"The Notion connector found saved artifact '"$source_artifact"' but did not receive a bounded preview, so no remote write was attempted.","source":"plugin.notion-connector","tags":["connector","notion","local-draft"]}]}}}'
      else
        printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"content":[],"structuredContent":{"items":[{"kind":"pluginResult","title":"Notion Page Draft","content":"Prepared a credential-scoped local Notion page draft. Review the title, summary, and next actions before any remote Notion update.","attributes":{"targetService":"notion","draftMode":"localDraft","remoteWrite":"false","credentialScoped":"true","targetTool":"notion.preparePageDraft","sourceArtifact":"workspace"}}],"memoryNotes":[{"title":"Notion Draft Prepared","body":"The Notion connector prepared a credential-scoped local page draft without a remote write.","source":"plugin.notion-connector","tags":["connector","notion","local-draft"]}]}}}'
      fi
      ;;
  esac
done
