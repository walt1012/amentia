# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS agent app that people can actually
use for controlled local work. It should feel native, focused, recoverable, and
capable without becoming a prototype, generic chatbot, terminal skin,
hosted-model frontend, or feature zoo.

## Product Rules

- Product: `Pith`, macOS 12+, `x86_64` only.
- Intelligence: local model by default; no required external model API.
- First use: in-app model download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Plugins: real local capabilities, not prompt templates.
- Retrieval: the default-enabled Web Search plugin is the active retrieval
  layer; no generic local RAG yet.
- Repository: English-only source, docs, commits, branches, and PR text.
- Foundation: free and open source.

## Daily-Use Standard

- A normal user can install, launch, download a model, open a workspace, send a
  request, review results, and recover from common failures without using a
  terminal.
- Every core action must have clear in-app state: ready, running, blocked,
  failed, cancelled, or recovered.
- Failure messages must explain the next useful action, not expose internal
  implementation details as the primary user experience.
- Runtime, model, plugin, web search, sandbox, and packaging work is complete
  only when it holds together in the packaged macOS app.
- Developer convenience must not replace product readiness; CI and scripts
  prove the app path, but the app experience is the product.

## Architecture Boundaries

- `apps/pith-macos`: native UI, setup, approvals, timeline, model manager,
  inspector, and app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, request routing, request
  supervision, notifications, and runtime lock boundaries.
- `crates/pith-core`: orchestration, request reducers, turn lifecycle,
  readiness, permissions, context packing, memory usage, and plugin execution.
- `crates/pith-tools`: bounded workspace tools, shell, web search, output
  compaction, and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory semantics, note ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory and storage do not conflict: memory owns meaning and ranking; storage
owns durable records.

## Closed Foundation

Milestones 1-4 are closed. Keep their details in git history, not in this plan.

Closed capabilities:

- Guided local model setup, resumable downloads, verified single-model
  activation, runtime recovery, bounded shell and model work, workspace-safe file
  tools, web search, native sandbox diagnostics, compact context packing, and
  progressive inspector surfaces.
- Typed plugin registries, local install/remove, inspect-before-install,
  enable/disable, connector auth, bounded `stdio` and MCP stdio runners,
  permission gates, approval gates, deterministic output envelopes, runner
  memory notes, repair hints, and retry flows.
- Timeline trust boundaries for approvals, plugin runs, connector blockers,
  source reveal, refresh recovery, runtime status, and credential-safe metadata.

## Current Milestone: M5 Daily Driver Hardening

M5 turns the working local agent platform into a dependable daily-driver macOS
app: first launch, model setup, workspace work, recovery, packaging, and plugins
must hold together as one usable product without expanding into a feature zoo.

Order of Work:

- First-run daily loop: auto-start the local runtime, download or resume a
  model, activate it, open a workspace, create a thread, send the first
  request, and recover in-app when model, runtime, web search, plugin, or
  sandbox readiness is missing.
- Agent execution loop: keep turns, approvals, workspace search, web search,
  plugin commands, and model activation request-scoped, cancellable, and
  visible without blocking unrelated read-only UI updates.
- Native safety loop: keep workspace file tools symlink-safe, sandbox decisions
  visible, sandbox temporary roots symlink-safe, plugin runner output untrusted
  by default, and recovery actions tied to trusted runtime metadata.
- Package loop: keep the x86_64 macOS 12 app bundle signed-ready with runtime
  binary, exact x86_64 architecture validation, model metadata, plugin
  manifests, no model weights, parallel cached executable builds, and launch
  smoke coverage.

Immediate Next:

- Prove the first-run loop in the packaged app: runtime autostart, model
  download/resume, activation, workspace bootstrap, and the first local request.
- Keep execution visibility accurate for every lane: turns, approvals,
  workspace search, web search, plugin commands, and model activation.
- Tighten plugin runner, connector, sandbox, and retry diagnostics only where
  they unblock real local plugin execution.
- Keep UI and architecture compact; refactor only when ownership, failure
  boundaries, or cancellation behavior become clearer.

M5 Exit Gate:

- A fresh install can download a model, open a workspace, use web search, run a
  plugin command, and recover from model/runtime/plugin failures in-app.
- Sandbox and approval decisions are visible, bounded, and reversible.
- The packaged app can be used for a short real workflow without manual CLI
  setup, hidden required files, or unexplained blocked states.
- CI produces a validated, ad-hoc signed x86_64 macOS 12 app bundle artifact
  with model metadata and plugin manifests, but no model weights.

## Not Now

- No hosted model dependency.
- No multi-agent workflows.
- No generic document RAG or local vector database.
- No broad connector marketplace.
- No manifest-declared Keychain credentials until native Keychain storage exists.
- No remote MCP transport until bounded local execution supports it.
- No cosmetic refactor that only moves code around.
- No large UI expansion before plugin execution is real.

## Engineering Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when ownership or failure boundaries become clearer.
