# Contributing

This repository is currently in the foundation phase for `Pith`.

## Project Rules

- Use English only in source files, comments, docs, commit messages, branch names, and PR titles.
- Keep changes scoped and reviewable.
- Prefer extending the protocol and runtime through typed data models instead of stringly typed ad hoc payloads.
- Preserve the local-first product direction.

## Branching

- Default branch prefix: `codex/`

## Recommended Checks

Rust:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `python3 scripts/runtime_smoke_test.py`

Swift:

- `cd apps/pith-macos`
- `swift build`

## Milestone 0 Intent

The current repository target is a stable foundation:

- monorepo structure
- macOS app shell
- Rust runtime process
- stdio protocol boundary
- thread and turn scaffolding
- baseline persistence

Later milestones can expand execution, tools, approvals, and plugin behavior on top of this base.
