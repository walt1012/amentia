# Contributing

This repository has completed the Milestone 1 local agent baseline for `Pith`.

## Project Rules

- Use English only in source files, comments, docs, commit messages, branch names, and PR titles.
- Keep changes scoped and reviewable.
- Preserve the `macOS 12+` and `x86_64` product target. Do not introduce Apple Silicon-only assumptions.
- Keep `Pith` lightweight. Prefer simple, maintainable solutions over broad heavy abstractions.
- Preserve the local-first inference path. Do not add required external model APIs to the core product loop.
- Treat plugins as first-class product modules rather than optional afterthoughts.
- Favor free and open source dependencies and delivery paths.
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

## Milestone 1 Baseline

The current repository target is a stable local agent loop:

- native macOS shell with sidebar, timeline, composer, and inspector
- Rust runtime process over `stdio`
- thread, turn, streaming, and cancellation flow
- filesystem, shell, diff, and approval-gated write tools
- SQLite-backed persistence and built-in memory
- local model health inspection and pack metadata bootstrap

Later milestones can expand plugin management, desktop polish, and multi-agent behavior on top of this baseline.
