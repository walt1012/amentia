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

Milestones 1-5 are closed. Keep their details in git history, not in this plan.

Closed capabilities:

- Local model setup, resumable downloads, verified single-model activation,
  runtime recovery, bounded shell/model work, workspace-safe tools, web search,
  sandbox diagnostics, compact context packing, and progressive inspector
  surfaces.
- Plugin registry, local install/remove, inspect-before-install, enable/disable,
  connector auth, bounded `stdio` runners, response-aware MCP stdio sessions,
  permission gates, approval gates, output envelopes, repair hints, and retry
  flows.
- Timeline trust boundaries for approvals, plugin runs, connector blockers,
  source reveal, refresh recovery, runtime status, and credential-safe metadata.
- Daily-driver package proof: CI validates the x86_64 macOS app bundle, bundled
  runtime protocol, first-use model metadata, app-owned support directories,
  workspace bootstrap/search, deterministic first local request, web search,
  bundled MCP plugin command execution, connector authorization, approval
  recovery, runner memory capture, launch smoke coverage, internal DMG shape,
  release-state safety, and the Developer ID upgrade path.

## Current Milestone: M6 Agent Loop and Real Connectors

M6 upgrades Pith from a safe single-action local assistant into a small,
auditable agent that can plan, use tools repeatedly, observe results, and stop
cleanly. The goal is not to copy Codex or Claude feature-for-feature; it is to
keep Pith native, local-first, and small while closing the biggest product gap:
the agent loop.

Workstreams:

- Bounded agent loop: replace the single heuristic turn router with a
  request-scoped Plan/Act/Observe loop that has a strict step cap, cancellation,
  tool budgets, approval pauses, recovery items, and readable timeline state.
- Tool contracts: make file read/write, search, web search, shell, plugin
  command, and future Git actions available through one typed local tool
  contract instead of one-off routing branches.
- Real connector proof: graduate the Notion-style connector from dry-run proof
  to one real local MCP connector path with credential handling, sandbox
  visibility, safe failure modes, and no secret leakage.
- Source-grounded retrieval: keep Web Search as the retrieval layer, but improve
  source attribution, result inspection, and citation-ready summaries before
  considering any generic local RAG.
- Minimal Git loop: add only the daily-driver Git surface Pith needs first:
  status, diff, stage selected changes, commit message draft, and optional
  worktree isolation. Avoid becoming a full terminal or Git client.

Current Status:

- Pith has a solid local-first app foundation: model manager, sandbox,
  approvals, recovery, workspace tools, web search, plugin manifests, local MCP
  stdio execution, packaging, release gates, and CI smoke coverage.
- The main gap versus Codex and Claude is now the agent loop: most turns still
  resolve to one prepared action before execution instead of model-guided
  multi-step tool use.
- The plugin stack is structurally ready for real connectors, but the bundled
  Notion connector is still a safe dry-run proof.

Next Work:

- Define the typed local tool contract and a compact agent step record.
- Move turn planning behind a bounded agent-loop coordinator while preserving
  existing single-action behavior as the first implementation path.
- Add timeline output for each agent step: plan, tool call, observation,
  approval pause, cancellation, and final answer.
- Add one real connector proof after the loop can call plugin tools naturally.
- Add the minimal Git surface only after the loop can observe diffs and feed
  them back into planning.

Architecture Watchlist:

- Keep `AppViewModel`, timeline presenters, runtime request routing, agent loop,
  and plugin runner modules split by ownership and failure boundary, not
  cosmetic file size.
- Keep MCP target resolution, stdio session supervision, and output/protocol
  parsing separate as connector support grows.
- Do not split Swift files further unless a domain owner, state owner, or
  failure boundary becomes clearer.
- Keep release and CI policy in tested scripts and reusable workflow steps, not
  growing inline shell blocks.
- Do not let M6 become generic RAG, multi-agent orchestration, browser
  automation, or marketplace work.

M6 Exit Gate:

- A single user request can run at least three bounded agent steps across two
  tool types, then produce a final answer with visible observations.
- Cancelling the turn stops pending model/tool work and leaves a coherent
  timeline state.
- Approval-paused tools can resume the same agent step after approval without
  losing workspace, memory, or connector context.
- Web search results carry source attribution into the final answer.
- One real connector command works through the same agent loop and remains
  sandboxed, bounded, and credential-safe.

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
