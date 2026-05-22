# Pith Development Plan

## Product Direction

- Pith is a small, strong, local-first macOS cowork agent for real daily work.
- Target: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the product boundary.
- Intelligence: local model by default; no required hosted model API.
- First use: in-app verified GGUF download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Retrieval: Web Search is the active retrieval layer; no generic local
  document RAG until the cowork loop is excellent.
- Plugins: real local capabilities and connectors, not prompt templates.
- Delivery: users install a downloadable macOS app package; CI proves the app
  path, but the app experience is the product.

## Architecture Map

- `apps/pith-macos`: native UI, setup, approvals, timeline, model manager,
  inspector, and app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, request routing, request
  supervision, notifications, and runtime lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, permissions, context,
  memory usage, and plugin execution.
- `crates/pith-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory semantics, note ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records.

## Closed Foundation

Milestones 1-5 are closed. Keep details in git history, not in this plan.

Closed capabilities:

- First-use model setup, resumable downloads, verified activation, runtime
  recovery, bounded shell/model work, workspace-safe tools, Web Search, sandbox
  diagnostics, compact context packing, and progressive inspector surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector auth,
  bounded runners, MCP stdio sessions, permission gates, approval gates, output
  envelopes, repair hints, retry flows, and runner memory capture.
- Package proof: x86_64 app bundle, internal DMG workflow, packaged smoke
  coverage, release-state safety, native sandbox fallback, and unsigned
  distribution path with optional Developer ID upgrade later.

## Current Milestone: M6 Cowork Agent Loop

Goal: replace the polished single-action assistant with one compact,
auditable Plan/Act/Observe loop that can call tools, observe results, pause for
approval, resume, cancel, and produce source-grounded cowork output.

Current state:

- Done: timeline items carry stable loop, step, local-tool, tool-call status,
  and Web Search source metadata.
- Done: approval resume preserves the same agent step metadata.
- Done: prepared actions execute through a turn step dispatcher.
- Done: normal turns now use dispatcher loop metadata with step count, stop
  reason, remaining budget, and observation count.
- Done: turn execution now runs through a request-scoped loop runner with a
  hard three-step budget and a next-action seam.
- Done: the loop can continue from workspace search to read_file when one file
  is the clear search result.
- Done: project overview requests can list the workspace and then read the
  best root entry point without auto-reading on list-only requests.
- Done: project overview can continue into a root manifest after reading an
  entry point, proving three bounded steps across workspace and file tools.
- Done: final file summaries now receive prior tool observations so multi-step
  handoffs are grounded in the whole loop, not only the last file.
- Done: Web Search final items persist source attribution, source titles, and
  source URLs.
- Done: fresh public requests route to Web Search before workspace search even
  when a workspace is open.
- Done: the bundled Notion connector now proves a credential-scoped local MCP
  draft command with approval, structured output, and memory capture.
- Gap: planner integration covers safe workspace observation paths, but not
  shell/write approvals, plugin commands, connector commands, or review/apply.
- Gap: workspace review/apply/handoff is not yet a general cowork flow.

M6 work order:

1. Planner integration: expand observation-based next actions beyond the safe
   search-to-read path.
2. Tool migration: Web Search next, then shell/write
   approvals, plugin commands, connector commands, and review/apply.
3. Connector loop integration: route the Notion local draft command through the
   same loop observations as workspace and Web Search tools.
4. Cowork proof: one request can search/read or use Web Search, cite sources,
   explain observations, and finish with a concise handoff.

M6 exit criteria:

- One user request can run at least three bounded steps across at least two
  tool types.
- Cancellation stops pending model/tool work and leaves a coherent timeline.
- Approval-paused tools resume the same step without losing workspace, memory,
  or connector context.
- Web Search answers include visible source attribution.
- One connector command works through the same loop and remains sandboxed,
  bounded, and credential-safe.

## Next Milestone: M7 Practical Cowork

Start M7 only after M6 exits.

- Workspace-aware editing loop with safe diffs and reviewable writes for notes,
  docs, config, and code.
- Practical handoff flows: summarize work, draft next actions, prepare
  connector updates, and optionally package local changes for Git workspaces.
- Better context compaction for long sessions and small local models.
- Connector hardening based on the M6 real connector proof.
- App polish only where it improves daily cowork clarity.

## Guardrails

- No hosted model dependency.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No multi-agent orchestration before the single cowork loop is excellent.
- No marketplace or remote MCP transport until local connector execution is
  safe and useful.
- No cosmetic refactor that only moves code around.
- English-only source, docs, commits, branches, and PR text.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
- Split modules only when ownership or failure boundaries become clearer.
