# Slack Workspace Guidance

- Prepare concise cowork updates with a clear audience, current status, blocker,
  and next action.
- Keep drafts local until the user explicitly approves a Slack post.
- Use `slack.inspect-message-send` before any remote send request.
- Use `slack.post-message-draft` only with a valid `channelId` and message text.
- Treat Slack proof as successful only when the API returns `ok`, `channel`, and
  `ts`; otherwise preserve retry input and report that no trusted proof was
  accepted.
