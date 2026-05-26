# Notion Workspace Connector

Use this connector skill when a user wants Pith to prepare workspace notes, summaries, or task context for Notion.

Current contract:

- Keep all analysis local until the user explicitly authorizes a Notion
  integration token.
- Treat Notion writes as network-enabled plugin actions that require approval.
- Inspect proposed Notion writes before any remote update; inspection must not
  send data to Notion.
- Publish only through `notion.publish-page-draft` after approval. The command
  input must include `parentPageId`, `title`, and optional `body`.
- Prefer concise page outlines, decision logs, and task lists over long chat transcripts.
- Do not assume credentials exist; check the runtime connector auth state before preparing Notion actions.
