# Amentia Development Plan

## North Star

Amentia is a small, strong, local-first macOS cowork agent for real daily work.

- Product: native `Amentia` app, macOS 12+, `x86_64` only.
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

Amentia learns from Codex and Claude Code at durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox visibility, session continuity,
reviewable evidence, and MCP-style local connections.

Amentia also learns from Hermes Agent, but only where it supports the local macOS
cowork goal:

- Keep the core narrow and move optional capability to plugins, tools,
  connectors, MCP, checks, and skills.
- Make execution observable, interruptible, resumable, and receipt-backed.
- Keep memory and skills bounded, curated, and progressively loaded.
- Prefer edge expansion over core bloat.
- Do not copy server-first messaging sprawl, multi-agent orchestration, or
  provider complexity before the single local cowork loop is excellent.

Amentia stays intentionally different where it matters: local-first inference, no
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

- `apps/amentia-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/amentia-runtime-bin`: JSON-RPC process, routing, notifications, request
  supervision, and lock boundaries.
- `crates/amentia-core`: orchestration, turn lifecycle, context selection, memory
  use, plugin execution, and local execution safety.
- `crates/amentia-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/amentia-sandbox`: native sandbox policy and diagnostics.
- `crates/amentia-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/amentia-memory`: memory meaning, ranking, summaries, and context
  selection.
- `crates/amentia-storage`: durable records for threads, workspace state,
  approvals, workspace change ledger, memory notes, and plugin state.
- `crates/amentia-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records. Connector
evidence stays generic at protocol and timeline layers; service-specific detail
belongs in connector attributes and narrow presenter adapters. Refactor only
when ownership, contracts, or user-facing clarity improve; do not split or merge
files cosmetically.

## Current State

Active milestone: **M14 Connector and Skill Platform**.

Implemented foundations:

- Local model setup has the core path in place: in-app download, verification,
  activation, pause, resume, cancel, recovery, backend launch probing,
  file-backed bounded inference, in-app model self-check, Reset Amentia,
  curated model choice, and one active model.
- Cowork loop foundations are in place: project-scoped tools, Web Search
  retrieval, approvals, sandbox diagnostics, bounded subprocesses, human
  receipts, session delete, and session-level change preview/revert.
- Extension baseline: local plugin registry, installation lifecycle, actions,
  connections, skills, MCP servers, tools, checks, credentials, retries, generic
  proof surfaces, and Notion as the reference connection.
- Product language: normal setup, model, project, session, readiness, timeline,
  inspector, plugin, connection, permission, and local-data paths avoid raw
  protocol names, paths, IDs, hashes, and manifest details by default.
- Primary window foundations are in place: native sidebar density, calm timeline
  cards, focused composer, readiness, first-run setup, session sidebar
  ownership, model management, project search, plugin management, inspector
  sections, settings surfaces, and subtle state-driven motion without fixed dark
  styling.
- Release baseline: unsigned x86_64 DMG, concise GitHub Release assets, install
  guidance, checksum, manifest, package smoke proof, manual acceptance receipt,
  transparent-corner macOS icon packaging, and split CI as the source of truth.

Current proof gaps:

- Installed-app model deployment is the first gate. A fresh installed DMG must
  download, verify, activate, self-check, and invoke a local model before more
  connector breadth is added.
- User-owned cleanup is the second gate. Session delete, session revert, plugin
  removal, connection credential clearing, and Reset Amentia must leave no
  app-owned residue outside the intended support, cache, preference, saved-state,
  and local credential locations.
- Product clarity is the third gate. Default UI should stay human and calm; raw
  protocol names, manifest strings, hashes, internal paths, and schema details
  belong only in diagnostics or reveal-on-demand proof.

Current constraints:

- Installed-app blockers beat platform expansion: first-run model download and
  activation, packaged backend launch, human default UI, visible session
  deletion, and Reset Amentia must stay reliable before more M14 connector surface
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
- `Amentia` is the product, app, package, runtime, crate, environment variable,
  plugin manifest, and GitHub repository namespace.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

Status: packaging baseline complete; Amentia needs a fresh installed-app
acceptance pass before the next visible public tag.

Completed:

- The release exposes exactly the DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- The GitHub Release page stays concise; detailed install, Gatekeeper,
  verification, and package metadata stay in `README-FIRST.txt` and the release
  manifest.

Required evidence:

- A fresh unsigned install completes the daily cowork loop without hosted model
  dependency or manual model import.
- The GitHub Release exposes exactly the four public assets and final publish
  validation passes against the live release.
- Release plan, release manifest, `README-FIRST.txt`, release notes, packaged
  smoke proof, manual acceptance receipt, and downloaded-asset rehearsal must
  describe the same user path for each visible release.
- CI stays fast, split, strict, and understandable enough to block release
  contract drift.

## M13: Product Quality and Identity

Status: baseline established; installed-app proof remains active.

Goal: make the shipped app feel intentional, maintainable, and worthy of daily
use before expanding the platform surface.

Completed baseline:

- Clean the installed app experience: human UI copy, first-run model setup,
  light-mode/system appearance support, and no internal wording in the normal
  cowork path.
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
- Finish product identity around the approved Amentia lockup reference, derive
  the Dock icon mark from that source, keep SVG previews byte-aligned with PNG
  assets, require transparent outer icon corners, and ship through the native
  macOS icon package contract.
- Keep extension management understandable: plugin installation is the bundle
  workflow, while capabilities are grouped as Actions, Connections, Skills,
  MCP, Tools, and Checks.
- Keep plugin management clean enough for real users: confirmation dialogs show
  what a plugin can do, what access it needs, and what Amentia will store without
  exposing source paths or manifest capability strings.
- Keep timeline and inspector evidence product-first: domain presenters own copy
  and proof for runtime, model, session, plugin, connection, and action
  surfaces; paths and protocol fields stay secondary.
- Keep model manager details product-first: default summaries describe context and
  response limits, while runner, path, and package details stay diagnostic.
- Keep reset and session deletion copy explicit: Amentia data, chat history,
  activity cards, saved connections, and project folders are described in user
  terms before destructive actions run.
- Keep support diagnostics available without letting them dominate the default
  inspector path.
- Treat Amentia as a fresh app identity: app-owned data lives under `Amentia`,
  Reset Amentia deletes only Amentia-owned files, and legacy product data is not
  migrated or cleaned implicitly.
- Keep release and settings copy product-first: internal package filenames and
  schema details stay out of normal user-facing explanations.

Remaining quality bar:

- A fresh install can download, verify, activate, re-download, recover, and
  invoke a local model without expert context; readiness must fail early if the
  packaged backend cannot launch, and generation must use bounded file-backed
  prompt input with an automatic short self-check after model selection plus a
  visible manual check from first-use setup and the model manager.
- Users can delete a session, review or revert session changes, and Reset Amentia
  from visible UI without learning hidden menus; Reset Amentia must remove
  app-owned folders, paused downloads, preferences, caches, saved app state, and
  local connector tokens or keys.
- The default path avoids raw runtime, manifest, checksum, ID, and backend
  wording; advanced diagnostics remain available only when useful.
- Amentia can run the cowork loop, use Web Search, manage sessions, and recover or
  revert approved work without expert context.
- Extension surfaces use precise ecosystem language: Plugins are installed
  bundles; Actions, Connections, Skills, MCP, Tools, and Checks are capabilities
  when present. Slash routes and manifest strings stay out of the primary path.
- CI stays fast, split, strict, and green for the release package path.
- The app has a polished blue Dock icon and no obvious stale, unused, or
  confusing UI surfaces left in the primary path.
- The approved Dock icon uses a 2048 px PNG master with a byte-matched SVG
  preview wrapper and native macOS icon packaging checks.

## M14: Connector and Skill Platform

Status: active.

Goal: make third-party local plugins safe and useful without building a
marketplace too early.

Completed baseline:

- Plugins are installed bundles. Actions, Connections, Skills, MCP servers,
  Tools, and Checks are capabilities inside those bundles.
- The plugin host validates manifests, keeps skill paths and MCP commands inside
  bundle roots, bounds runner output, supervises registry loading, and fails
  closed when setup, credentials, or local secrets are missing.
- Notion is the reference connection. Its authorization metadata, local token or
  key handling, missing-secret state, cleanup, timeline proof, retries, and
  receipts exercise the generic connector contract.
- Executable connector workflow commands declare explicit input and output
  envelopes so third-party bundles do not depend on implicit runner defaults.
- Connector evidence stays generic in protocol and timeline data. Service names,
  access scopes, auth type, proof labels, and repair copy are translated by
  narrow presenters.
- Skills are bounded, read-only context packs with query selection, strict
  budgets, explicit `skill:<id>` capabilities, reviewable receipts, and a plugin
  disable path for revocation. Legacy `prompt_pack:<id>` entries may remain only
  as compatibility aliases for declared skills.
- Checks are verification surfaces with product-facing trigger copy and the same
  plugin disable path, not arbitrary always-on automation.
- Plugin manager surfaces expose capability meaning progressively. Source paths,
  manifest keys, raw event names, routes, hashes, and storage details stay out of
  the default path.
- Plugin action, connection, workflow, skill, and setup details use product
  labels instead of log-style field names in default summaries.

Exit gates:

- Reference connector proof: install or refresh the Notion bundle, authorize it,
  run one useful action or workflow, show a receipt, clear the credential, remove
  the plugin, verify no stale local credential state remains, and capture the
  result with `scripts/reference_connector_proof.py`. Store the accepted
  evidence under `docs/evidence/m14-reference-connector-proof.json` only after a
  real installed-app run. Keep the evidence workflow documented in
  `docs/evidence/README.md`. The proof must reject placeholders, use a UTC
  acceptance timestamp, and explicitly cover storage and local credential handle
  cleanup.
- Installed-app proof: in a packaged app, verify model download, activation,
  local inference, Web Search, session delete, session revert, Reset Amentia, and
  plugin install/disable/remove before adding more connector surface area.
- Runner proof: actions, connector workflows, skills, MCP metadata, and checks
  must have bounded output, timeout or cancellation behavior where execution is
  possible, product-facing errors, and CI coverage for failure paths.
- Product clarity proof: plugin, timeline, model, session, and settings surfaces
  must remain understandable to a non-developer by default, with technical
  detail available only through reveal or diagnostics paths.
- Scope decision: after the Notion proof is stable, either close M14 and move to
  M15 or add exactly one more connector to test whether the generic contract
  truly holds. Do not start a broad catalog first.

Immediate next work:

1. Prove fresh installed-app model deployment end to end from the DMG.
2. Capture the Notion reference connector proof in the evidence file above.
3. Continue the product clarity pass on advanced settings and installed-app
   proof surfaces.
4. Close M14 only after the installed-app and connector proofs are green.

## M15: Cowork Continuity

Goal: make Amentia better over time without turning it into a remote server agent.

- Start only after the M14 exit gates are green in a packaged app.
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
- No required Amentia login, hosted user session, or subscription gate.
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
