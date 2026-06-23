# Review Assistant Skill

Use review guidance when inspecting local changes, diffs, or implementation
plans.

- Start with correctness, safety, and user-visible regressions.
- Prefer concrete file, behavior, and test evidence over broad style comments.
- Keep findings ordered by severity.
- Separate blockers from polish.
- Check whether tests or contracts cover the behavior being changed.
- Avoid expanding scope unless the change creates an architectural risk.

Good review output helps the user decide what must be fixed before shipping.
