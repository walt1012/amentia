# Contributing

This repository is in Milestone 3 daily-driver hardening for `Pith`.

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

## Verification Policy

Use GitHub Actions as the canonical verification environment. Local Rust, Swift, and Python
toolchains are optional and should not block development when they are missing, stale, or broken.
Push scoped changes to a `codex/**` branch, inspect the remote CI result, and fix only the concrete
remote failures.

The remote CI suite owns:

- Rust formatting, clippy, tests, and runtime smoke coverage
- model pack manifest validation
- Intel macOS Swift package builds

## CI Checklist

Before pushing a change that touches protocol shapes, plugin loading, permissions, defaults, or discovery logic:

- update the relevant unit tests in the Rust workspace
- update `scripts/runtime_smoke_test.py` if runtime behavior changed across the process boundary
- verify bundled sample plugin manifests still deserialize and validate against the runtime schema
- review sample data and fixtures for field casing so `camelCase` protocol types do not drift from checked-in JSON
- prefer self-contained smoke fixtures over assumptions about the surrounding repository layout

If a change affects plugin permissions or command discovery, treat these four surfaces as one compatibility set:

- runtime behavior
- unit and protocol tests
- runtime smoke coverage
- bundled plugin sample data

## Current Baseline

The current repository target is a stable local daily-driver loop:

- native macOS shell with sidebar, timeline, composer, and inspector
- Rust runtime process over `stdio`
- thread, turn, streaming, and cancellation flow
- filesystem, shell, web search, diff, and approval-gated write tools
- SQLite-backed persistence and built-in memory
- first-use local model download, verification, activation, and health inspection
- native sandbox diagnostics and bounded subprocess execution
- plugin metadata, discovery, permissions, and visibility foundations

Milestone 4 can add real plugin execution contracts and third-party connectors after Milestone 3 is stable.
