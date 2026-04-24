# Notion Workspace Connector

Use this connector skill when a user wants Pith to prepare workspace notes, summaries, or task context for Notion.

Current contract:

- Keep all analysis local until the user explicitly connects a Notion account.
- Treat Notion writes as network-enabled plugin actions that require approval.
- Prefer concise page outlines, decision logs, and task lists over long chat transcripts.
- Do not assume credentials exist; check the connector auth state first once runtime auth support is implemented.
