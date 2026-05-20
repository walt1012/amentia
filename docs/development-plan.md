# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS agent app that people can actually
use for controlled local work. It should feel native, focused, recoverable, and
capable without becoming a prototype, generic chatbot, terminal skin,
hosted-model frontend, or feature zoo.

## Non-Negotiables

- Product: `Pith`, macOS 12+, `x86_64` only.
- Intelligence: local model by default; no required external model API.
- First use: in-app model download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Plugins: real local capabilities, not prompt templates.
- Retrieval: the default-enabled Web Search plugin is the active retrieval
  layer; no generic local RAG yet.
- Repository: English-only source, docs, commits, branches, and PR text.
- Foundation: free, open source, native, and lightweight.

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

## Foundation Already Closed

Milestones 1-4 are closed. Keep their details in git history, not in this plan.

Closed capabilities:

- Local model setup, resumable downloads, verified single-model activation,
  runtime recovery, bounded shell/model work, workspace-safe tools, web search,
  sandbox diagnostics, compact context packing, and progressive inspector
  surfaces.
- Plugin registry, local install/remove, inspect-before-install, enable/disable,
  connector auth, bounded `stdio` and MCP stdio runners, permission gates,
  approval gates, output envelopes, repair hints, and retry flows.
- Timeline trust boundaries for approvals, plugin runs, connector blockers,
  source reveal, refresh recovery, runtime status, and credential-safe metadata.

## Current Milestone: M5 Daily Driver Hardening

M5 turns the working local agent platform into a dependable daily-driver macOS
app: first launch, model setup, workspace work, recovery, packaging, and plugins
must hold together as one usable product without expanding into a feature zoo.

Workstreams:

- First-run daily loop: auto-start the local runtime, download or resume a
  model, activate it, open a workspace, create a thread, send the first
  request, and recover in-app when model, runtime, web search, plugin, or
  sandbox readiness is missing.
- Agent execution loop: keep turns, approvals, workspace search, web search,
  plugin commands, and model activation request-scoped, cancellable, and
  visible without blocking unrelated read-only UI updates; shell artifacts stay
  bounded even when subprocess output is noisy.
- Native safety loop: keep workspace file tools symlink-safe, sandbox decisions
  visible, sandbox temporary roots symlink-safe, plugin runner output untrusted
  by default, runtime launch environment app-owned, and recovery actions tied
  to trusted runtime metadata.
- Package loop: keep the x86_64 macOS 12 app bundle signed-ready with runtime
  binary, self-contained local inference backend, exact x86_64 architecture
  validation, model metadata, plugin manifests, no model weights, parallel
  cached executable builds, Swift model-manager proof tests, packaged runtime
  protocol probes, launch smoke coverage, and a signed notarized DMG release
  path for users.

Current Status:

- Proven in CI: packaged runtime protocol health, isolated app support state,
  runtime database initialization, workspace bootstrap, workspace search, thread
  creation, web search readiness, packaged web search execution, sandbox
  readiness, deterministic first request coverage, fresh app-owned directory
  preparation, model manager
  download/resume/activation planning, packaged app launch smoke coverage, and
  local inference backend dependency portability and launch checks. CI is split
  into parallel policy, Rust, Swift, runtime, cached backend, and packaging
  gates so speed does not weaken release proof. Release tags produce the user
  installer path as a Developer ID signed, notarized DMG.
- Remaining M5 product work: prove the live first-run app path, keep execution
  cancellation/status accurate across every lane, and make real local plugin
  execution feel recoverable rather than experimental.

Next Work:

- Packaged first-run UI proof: launch fresh, guide model download or resume,
  activate one verified model, open a workspace, create a thread, send a first
  request, and recover without terminal help.
- Real local inference proof: run a valid downloaded GGUF through the packaged
  backend in CI when a small release-safe fixture is available.
- Execution lane hardening: keep turns, approvals, workspace search, web search,
  plugin commands, and model activation cancellable, visible, and non-blocking
  where the work is read-only.
- Plugin execution polish: improve runner diagnostics, connector auth recovery,
  sandbox visibility, and retry flows only when they improve real local plugin
  execution.
- Architecture guardrails: keep `AppViewModel`, timeline presenters, runtime
  request routing, and plugin runner modules split by ownership and failure
  boundary, not by cosmetic file size.

M5 Exit Gate:

- A fresh install can download a model, open a workspace, use web search, run a
  plugin command, and recover from model/runtime/plugin failures in-app.
- Sandbox and approval decisions are visible, bounded, and reversible.
- The packaged app can be used for a short real workflow without manual CLI
  setup, hidden required files, or unexplained blocked states.
- CI produces a validated, ad-hoc signed x86_64 macOS 12 app bundle artifact
  with model metadata, plugin manifests, and a self-contained local inference
  backend, but no model weights.
- Release tags publish a signed and notarized `Pith-<tag>-macos-x86_64.dmg`
  installer plus checksum to GitHub Releases.

## Not Now

- No hosted model dependency.
- No multi-agent workflows.
- No generic document RAG or local vector database.
- No connector marketplace until secure connector execution is proven with
  native credential storage.
- No remote MCP transport until bounded local execution supports it.
- No cosmetic refactor that only moves code around.
- No large UI expansion before plugin execution is real.

## Engineering Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
- Prefer parallel jobs, pinned external inputs, and narrow caches over weaker
  checks.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when ownership or failure boundaries become clearer.
