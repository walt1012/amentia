# Amentia Development Plan

## North Star

Amentia is a small, strong, local-first macOS cowork agent for real daily work.

- Native `Amentia` app, macOS 12+, `x86_64` only.
- Cowork first; coding is one workflow, not the product boundary.
- Local model by default, no required hosted model API, one active model at a time.
- First use downloads and verifies Granite 4.0-H-350M as the default small model in app.
- Web Search is the active retrieval layer; generic local document RAG is deferred.
- Plugins are installable local bundles with bounded capabilities: Actions,
  Connections, Skills, MCP, Tools, and Checks.
- Users install a downloadable macOS DMG from GitHub Releases.

## Product Contract

Amentia learns from Codex and Claude Code where the patterns are durable:
workspace context, bounded tools, Web Search, approvals, sandbox visibility,
session continuity, reviewable receipts, and MCP-style local connections.

Amentia stays intentionally different where it matters: no required account,
local-first inference, small-model constraints, cowork-first tasks, and a
lightweight app that downloads model weights after install.

The daily loop is:

1. Understand the project and request.
2. Retrieve only useful context.
3. Choose a bounded tool, action, connector, skill, or search.
4. Explain the action with a compact receipt.
5. Ask before writes or external effects.
6. Execute, show the result, remember useful state, and continue.

## Architecture

- `apps/amentia-macos`: native UI, setup, timeline, approvals, model manager,
  settings, and app-facing state.
- `crates/amentia-runtime-bin`: JSON-RPC process, routing, notifications,
  request supervision, and lock boundaries.
- `crates/amentia-core`: turn lifecycle, context selection, plugin execution,
  approvals, receipts, and local execution policy.
- `crates/amentia-tools`: bounded workspace tools, shell, Web Search, output
  shaping, and path safety.
- `crates/amentia-sandbox`: native sandbox policy and diagnostics.
- `crates/amentia-model-runtime`: model discovery, validation, health, bounded
  inference, and failure wording.
- `crates/amentia-memory`: memory meaning, ranking, summaries, and context
  selection.
- `crates/amentia-storage`: SQLite-owned durable records for sessions,
  workspace state, approvals, change ledger, memory, and plugin state.
- `crates/amentia-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Ownership rules:

- Memory owns meaning and ranking. Storage owns records.
- Connector receipts stay generic; service-specific wording belongs in narrow
  presenters.
- Refactor only when ownership, contracts, or user-facing clarity improve.
- Do not split, merge, or rename files cosmetically.

## Current Baseline

M12 Public Release, M13 Product Quality, and M14 Connector Platform are complete
as baselines. Their details should live in code, tests, release artifacts, and
Git history, not as a growing plan backlog.

The current product baseline includes:

- Release: unsigned x86_64 DMG, checksum, install guide, manifest, and manual
  acceptance before visible ad-hoc prereleases.
- Model: in-app download, verification, activation, pause, resume, cancel,
  first-use guidance, startup recovery, packaged backend probing, and one
  active local model with cowork paused until model startup succeeds.
- Cowork loop: workspace tools, Web Search retrieval, approvals, sandbox
  diagnostics, cancellable subprocesses, compact receipts, and first prompt
  drafting from setup or composer.
- Continuity: session search, delete, change preview, revert, last-session
  welcome recovery, reset of app-owned data, and clean recovery states.
- Extensions: local plugin registry, install lifecycle, Actions, Connections,
  Skills, MCP, Tools, Checks, credentials, retries, Notion proof, and generic
  connector receipts.
- Product quality: normal UI avoids raw protocol names, hashes, paths, manifest
  details, runner/setup-file wording, and legacy Pith identity; diagnostics keep
  technical details when needed.

## M15 Cowork Continuity

Status: active, local model loop hardening is late-stage; guarded session
operations, reset visibility, project-aware search, and revert receipts are in
place; repeated saves to one file now revert as one clear file operation.

Current focus:

- Finish fresh-install model deploy, startup readiness check, invoke, and recovery polish.
- Keep session delete, revert, reset, and failure recovery clean and visible.
- Remove stale identity/model docs and user-visible internal wording.
- Polish the primary cowork path before adding more extension surface.

Goal: make Amentia useful across real sessions without turning it into a remote
server agent or a code-only assistant.

Exit criteria:

- A fresh install can download, activate, start, and invoke a local model
  without expert context.
- A failed model startup blocks cowork use until Amentia restarts with a
  passing model or the model is replaced.
- Users can delete sessions, revert session changes, and Reset Amentia from
  visible UI without leaving app-owned garbage behind.
- Web Search, workspace context, memory ranking, and session search provide
  enough retrieval for daily cowork use without a generic local RAG system.
- Long-running work is visible, cancellable, receipt-backed, and fails closed.
- Default UI stays calm, readable, system-appearance friendly, and free of
  unexplained internal terms.
- Tests cover user-visible and safety-critical contracts without repeating
  fixture scaffolding or implementation snapshots.

Work order:

1. Finish the local model loop: startup-only readiness check, invocation failure
   feedback, process cleanup, and plain-language first-use guidance.
2. Keep ordinary UI language product-first while leaving plugin, Skill, MCP,
   and diagnostics terms only where they help advanced users.
3. Finish session continuity: remaining delete/revert recovery gaps found in
   installed use.
4. Trim duplicated test fixtures and oversized presentation/plugin tests around
   stable user-visible contracts instead of implementation snapshots.
5. Improve cowork retrieval: Web Search by default, bounded memory ranking, and
   project/session context before any local document RAG.
6. Add a local follow-up queue only after approvals, receipts, and cancellation
   remain reliable.
7. Keep UI polish focused on the primary cowork path, not on admin panels or
   diagnostic surfaces.

## M16 Extension Hardening

Start only after M15 is reliable in installed-app use.

- Prove one more real connector beyond Notion before broad connector catalog work.
- Keep plugin execution local, bounded, cancellable, and receipt-backed.
- Improve sandbox policy and diagnostics without exposing implementation noise to
  normal users.
- Keep Skills and MCP as real interfaces, not renamed concepts or prompt
  templates.
- Defer marketplace, remote MCP transport, messaging gateways, and multi-agent
  orchestration until the single local cowork loop is excellent.

## Later

- Developer ID signing and notarization when an Apple Developer account exists.
- Optional bundled Git only if bounded system Git is insufficient for packaged
  users.
- Local document RAG only after Web Search, workspace context, memory ranking,
  and session search are reliable in daily use.
- Broader connector ecosystem after the local connector contract is proven.

## Guardrails

- No hosted model dependency.
- No required login, hosted user session, or subscription gate.
- No model weights, package-manager payloads, extra architectures, or unused
  runtimes in the app bundle.
- No generic local vector database before simpler retrieval is excellent.
- No multi-agent orchestration before the single cowork loop is excellent.
- Release assets stay limited to DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- Visible ad-hoc prereleases require a manual acceptance receipt.
- CI is remote source of truth; local checks should stay focused and lightweight.
- English-only source, docs, commits, branches, and PR text.
