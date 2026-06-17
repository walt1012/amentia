# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: native `Pith` app, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the product boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- Setup: first use downloads and verifies a local model in-app, defaulting to
  `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer. Generic local document
  RAG stays deferred until the daily cowork loop is excellent.
- Extensions: plugins are installable bundles; skills, actions, connections,
  MCP servers, checks, and tools are bounded local capabilities, not prompt
  templates or marketplace theater.
- Delivery: users install a downloadable macOS package from GitHub Releases.

## Product Contract

Pith learns from Codex and Claude Code at durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox visibility, session continuity,
reviewable evidence, and MCP-style local connections.

Pith also learns from Hermes Agent, but only where it supports the local macOS
cowork goal:

- Keep the core narrow and move optional capability to plugins, tools,
  connectors, MCP, checks, and skills.
- Make execution observable, interruptible, resumable, and receipt-backed.
- Keep memory and skills bounded, curated, and progressively loaded.
- Prefer edge expansion over core bloat.
- Do not copy server-first messaging sprawl, multi-agent orchestration, or
  provider complexity before the single local cowork loop is excellent.

Pith stays intentionally different where it matters: local-first inference, no
account requirement, small-model constraints, cowork-first tasks, and a
lightweight app that downloads model weights after install.

The daily loop is:

1. Understand the workspace and request.
2. Retrieve only useful context.
3. Choose a bounded tool, action, connector, or skill.
4. Explain the action with a compact receipt.
5. Ask before writes or external effects.
6. Execute, show proof, remember useful state, and continue.

## Architecture Boundaries

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, routing, notifications, request
  supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, context selection, memory
  use, plugin execution, and local execution safety.
- `crates/pith-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory meaning, ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, workspace change ledger, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records. Connector
evidence stays generic at protocol and timeline layers; service-specific detail
belongs in connector attributes and narrow presenter adapters. Refactor only
when ownership, contracts, or user-facing clarity improve; do not split or merge
files cosmetically.

## Current State

Active milestone: **M14 Connector and Skill Platform**.

Current capabilities:

- Local model setup: in-app download, verification, activation, pause, resume,
  cancel, recovery, backend launch probing, reliable relaunch after model
  selection, automatic post-selection self-check, file-backed bounded inference,
  in-app model self-check, Reset Pith, curated model choice, and one active
  model.
- Cowork loop: project-scoped tools, Web Search retrieval, approvals, sandbox
  diagnostics, bounded subprocesses, human receipts, session delete, and
  session-level change preview/revert.
- Extension baseline: local plugin registry, installation lifecycle, actions,
  connections, skills, MCP servers, tools, checks, credentials, retries, generic
  proof surfaces, and Notion as the reference connection.
- Product language: normal setup, model, project, session, readiness, timeline,
  inspector, plugin, connection, permission, and local-data paths avoid raw
  protocol names, paths, IDs, hashes, and manifest details by default.
- Primary window polish: native sidebar density, calm timeline cards, focused
  composer, readiness, first-run setup, session sidebar ownership, model
  management, project search, plugin management, inspector sections, settings
  surfaces, and subtle state-driven motion without fixed dark styling.
- Release baseline: unsigned x86_64 DMG, concise GitHub Release assets, install
  guidance, checksum, manifest, package smoke proof, manual acceptance receipt,
  transparent-corner macOS icon packaging, and split CI as the source of truth.

Current constraints:

- Installed-app blockers beat platform expansion: first-run model download and
  activation, packaged backend launch, human default UI, visible session
  deletion, and Reset Pith must stay reliable before more M14 connector surface
  area.
- Treat M13 as the installed-app quality baseline; only return to product polish
  when real use exposes confusing copy, stale UI, or release blockers.
- Keep Web Search as retrieval; generic local document RAG remains deferred.
- Keep M14 focused on safe local extension execution before broad connector
  catalogs, marketplaces, or remote transports.
- Do not bundle Git, model weights, package-manager payloads, extra
  architectures, or unused runtimes.
- Release assets stay limited to the DMG, checksum, `README-FIRST.txt`, and
  release manifest.
- Visible ad-hoc prereleases require an explicit manual acceptance receipt.

## M12: Public Release

Status: complete for the first public ad-hoc prerelease.

Goal: ship a usable macOS installer from GitHub Releases.

Completed:

- Published `v0.1.14` as the current accepted ad-hoc prerelease.
- Keep the downloaded-asset acceptance receipt at
  `docs/release/manual-acceptance-receipt-v0.1.14.json` as the release gate.
- The release exposes exactly the DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- The GitHub Release page stays concise; detailed install, Gatekeeper,
  verification, and package metadata stay in `README-FIRST.txt` and the release
  manifest.

Evidence:

- A fresh unsigned install completes the daily cowork loop without hosted model
  dependency or manual model import.
- The GitHub Release exposes exactly the four public assets and final publish
  validation passes against the live release.
- Release plan, release manifest, `README-FIRST.txt`, release notes, packaged
  smoke proof, manual acceptance receipt, and downloaded-asset rehearsal
  describe the same user path.
- CI stays fast, split, strict, and understandable enough to block release
  contract drift.

## M13: Product Quality and Identity

Status: baseline established; continue only as needed for daily-use quality.

Goal: make the shipped app feel intentional, maintainable, and worthy of daily
use before expanding the platform surface.

Completed baseline:

- Clean the installed app experience: human UI copy, clear first-run model
  setup, light-mode/system appearance support, and no internal wording in the
  normal cowork path.
- Polish the primary window: native sidebar density, readable timeline width,
  calm inspector sections, and subtle state-driven motion inspired by Codex and
  Claude rather than decorative animation.
- Keep proof useful but quiet: receipts are human-readable first; protocol
  fields, raw counters, hashes, paths, and setup files stay in technical detail
  surfaces or reveal-on-demand actions.
- Preserve architecture clarity: remove dead code and stale release scaffolding,
  keep root views composition-only, split presenter or runner ownership only
  when contracts are mixed, and avoid cosmetic moves.
- Keep execution reliable: long work is observable, cancellable, resumable when
  appropriate, receipt-backed, and CI-verified through shared scripts instead of
  repeated workflow shell blocks.
- Finish product identity around a refined blue lowercase `p` mark, keep the
  vector source and PNG candidate aligned, require transparent outer icon
  corners, and ship it through the native macOS icon package contract.
- Keep extension management understandable: plugin installation is the bundle
  workflow, while capabilities are grouped as Actions, Connections, Skills,
  MCP, Tools, and Checks.
- Keep plugin management clean enough for real users: confirmation dialogs show
  what a plugin can do, what access it needs, and what Pith will store without
  exposing source paths or manifest capability strings.
- Keep timeline and inspector evidence product-first: domain presenters own copy
  and proof for runtime, model, session, plugin, connection, and action
  surfaces; paths and protocol fields stay secondary.
- Keep support diagnostics available without letting them dominate the default
  inspector path.

Remaining quality bar:

- A fresh install can download, verify, activate, re-download, recover, and
  invoke a local model without expert context; readiness must fail early if the
  packaged backend cannot launch, and generation must use bounded file-backed
  prompt input with an automatic short self-check after model selection plus a
  visible manual check from first-use setup and the model manager.
- Users can delete a session, review or revert session changes, and Reset Pith
  from visible UI without learning hidden menus; Reset Pith must remove
  app-owned folders, paused downloads, preferences, caches, saved app state, and
  local connector secrets.
- The default path avoids raw runtime, manifest, checksum, ID, and backend
  wording; advanced diagnostics remain available only when useful.
- Pith can run the cowork loop, use Web Search, manage sessions, and recover or
  revert approved work without expert context.
- Extension surfaces use precise ecosystem language: Plugins are installed
  bundles; Actions, Connections, Skills, MCP, Tools, and Checks are capabilities
  when present. Slash routes and manifest strings stay out of the primary path.
- CI stays fast, split, strict, and green for the release package path.
- The app has a polished blue `p` Dock icon and no obvious stale, unused, or
  confusing UI surfaces left in the primary path.

## M14: Connector and Skill Platform

Status: active.

Goal: make third-party local plugins safe and useful without building a
marketplace too early.

Completed:

- Plugin installation/removal is the bundle lifecycle; connectors, actions,
  skills, MCP servers, tools, and checks are capabilities inside bundles.
- Notion is the reference connector while the generic local connector contract
  matures.
- Plugin runner execution is split by ownership: setup, entrypoint validation,
  subprocess execution, stdout parsing, MCP output protocol scanning, output
  schema, memory-note capture, timeline conversion, proof validation, metadata
  ownership, and handoff forwarding are separately testable.
- Remote-write proof and connector-workflow proof are generic timeline
  contracts, not Notion-specific code paths.
- Connector credentials are stored as metadata in durable storage and secrets in
  the secure local store; clear/remove paths forget runtime secrets and attempt
  full connector cleanup before reporting recoverable failures.
- Service-specific connector help is isolated behind narrow presenters so the
  common plugin, credential, command-input, and proof paths stay generic.
- Capability metadata is reviewed through product copy for connections, skills,
  MCP servers, actions, and checks without exposing manifest paths or raw
  protocol keys in the default plugin manager.

Next:

- Make capability metadata progressively loaded before adding broad catalogs or
  remote transports.
- Keep connector evidence generic in timeline data as more services are added;
  only service copy and service proof labels belong in narrow presenters.
- Prove connector import, local enablement, credential use, revocation, removal,
  retries, receipts, and memory capture with one reference connector before
  adding more services.
- Add a small skill-like instruction layer only if it is bounded,
  user-reviewable, progressively loaded, and stored locally.
- Treat hooks as verification and automation points first, not arbitrary
  always-on automation.

## M15: Cowork Continuity

Goal: make Pith better over time without turning it into a remote server agent.

- Add a local follow-up queue for user-approved next actions.
- Add scheduled work only after approvals, sandbox policy, and receipts
  work headlessly and fail closed.
- Add cross-session recall through bounded memory and session search before any
  local document RAG.
- Keep background work explicit, pausable, and easy to inspect from the app.
- Do not add messaging gateways, remote control channels, or multi-agent
  orchestration until the local single-agent experience is excellent.

## Later

- Developer ID signing and notarization when an Apple Developer account exists.
- Broader connector ecosystem after the local connector contract is proven.
- Native sandbox hardening beyond the current workspace and subprocess
  boundaries.
- Optional bundled Git only if bounded system Git proves insufficient for real
  packaged users.
- Local document RAG only after Web Search, workspace context, memory ranking,
  and session search are reliable in daily use.

## Guardrails

- No hosted model dependency.
- No required Pith login, hosted user session, or subscription gate.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No multi-agent orchestration before the single cowork loop is excellent.
- No marketplace or remote MCP transport until local connector execution is safe
  and useful.
- Keep established ecosystem terms such as plugin, skill, and MCP when they
  describe real interfaces; clarify them instead of renaming them away.
- No cosmetic or line-count-only refactor; split by ownership, contract clarity,
  or user-facing clarity.
- English-only source, docs, commits, branches, and PR text.
