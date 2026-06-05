# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: native `Pith` app, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- Setup: first use downloads and verifies a GGUF model in-app, defaulting to
  `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer. Generic local document
  RAG waits until the daily cowork loop is excellent.
- Extensions: plugins and connectors must be real local capabilities, not prompt
  templates or marketplace theater.
- Delivery: users install a downloadable macOS package from GitHub Releases.

## Product Contract

Learn from Codex and Claude Code at durable boundaries: workspace context,
bounded tools, Web Search, approvals, sandbox visibility, session continuity,
reviewable evidence, and MCP-style local connectors.

Pith should stay intentionally different where it matters: local-first
inference, no account requirement, small-model constraints, cowork-first tasks,
and a lightweight package that downloads model weights after install.

The daily loop is:

1. Understand the workspace and request.
2. Retrieve only useful context.
3. Choose a bounded tool or connector.
4. Explain the action with a compact receipt.
5. Ask before writes or external effects.
6. Execute, show proof, remember useful state, and continue.

## Architecture

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, routing, notifications, request
  supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, local execution safety,
  context, memory use, and plugin execution.
- `crates/pith-tools`: bounded workspace tools, shell, Web Search, compaction,
  and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health,
  bounded inference, and failure wording.
- `crates/pith-memory`: memory meaning, ranking, summaries, and context
  selection.
- `crates/pith-storage`: durable records for threads, workspace state,
  approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries,
  connector metadata, and bundle lifecycle.

Memory owns meaning and ranking. Storage owns durable records. Connector
evidence stays generic at protocol and timeline layers; service-specific detail
belongs in connector attributes and narrow presenter adapters.

## Current Stage

Milestones 1-9 are closed. Implementation history belongs in git history, not
this roadmap.

Active milestone: **M12 Public Release**.

Ready foundations:

- First-use model setup, verified activation, bounded local inference, and
  runtime recovery are in place.
- Workspace-safe tools, sandbox diagnostics, bounded subprocess execution, Web
  Search retrieval, and compact receipts are in place.
- First-run readiness, Web Search permission recovery, plugin recovery, and
  sandbox recovery now stay visible from the main cowork loop.
- Plugin registry, connector credentials, local execution gates, approvals,
  retries, runner memory capture, and Notion as the reference connector are in
  place.
- x86_64 DMG packaging, unsigned install guidance, package-size budgets,
  release manifests, release copy validation, and packaged smoke coverage are in
  place.
- Mounted-DMG smoke now emits a first-run receipt that release manifests embed
  and validate, so release proof names the user path it exercised.
- Release notes and `README-FIRST.txt` tell users where to inspect that
  packaged first-run proof before opening the app.
- Release state planning now prevents accidental public-release rollback to
  draft; visible withdrawal stays a deliberate maintainer action.
- Release publishing now reads the final GitHub Release back and validates tag,
  title, draft/prerelease state, exact public installer assets, non-empty
  uploads, and tag-scoped download URLs.
- Release publishing is staged: plan release state, upload draft assets,
  download and rehearse those assets, then apply final visibility and validate
  the GitHub Release.
- Release planning writes the final draft/prerelease state and trust path into
  the workflow summary before upload, so maintainers see what will become
  visible before assets move.
- Release plans now include the source commit, successful CI run, and release
  workflow run before assets upload, keeping the first tag path auditable.
- Existing GitHub Releases are checked for non-contract assets before upload,
  keeping reruns from carrying stale packages or model payloads into a user
  download page.
- Release workflow dispatch can run a dry-run build that validates and
  rehearses the same DMG, checksum, install guide, manifest, and release plan
  without creating or updating a GitHub Release.
- Downloaded release rehearsals can validate a whole asset directory against
  the same installer contract used before upload, then summarize first-run
  proof, checksum command, signing mode, source commit, and first app-open
  checks.
- First app-open copy now matches the release rehearsal path: choose Map
  Workspace, Plan Next Step, or type a short cowork request.
- First app-open actions now avoid fake single-choice behavior: readiness shows
  state and sends only an existing draft, while starter prompt selection stays
  in the visible cowork callout.
- Release notes, `README-FIRST.txt`, and downloaded-release rehearsal now share
  the same first app-open action copy contract.
- CI now checks that Swift first-open copy stays aligned with the release copy
  contract.
- Release manifests now include the first app-open action contract, and packaged
  smoke proof scope names the first cowork request explicitly.
- `PithPackage.json` now carries the same first app-open action contract, so the
  installed app can surface it in distribution trust copy.
- Swift first-open and distribution-trust copy now share the same presenter
  contract instead of duplicated action strings.
- Packaged smoke receipts now embed the same package metadata proof that release
  manifests validate against `PithPackage.json`.
- Downloaded-release rehearsals now require the same app package proof and smoke
  package metadata before declaring a downloaded installer set ready.
- Release notes, install guides, and rehearsal summaries now name the app
  package metadata, smoke package metadata, checksum command, and Gatekeeper
  trust path users or maintainers must inspect.
- Final GitHub Release validation now checks the published release body as well
  as the tag state and installer asset set.
- Published release body validation now also matches the signing mode, draft
  state, and untrusted ad-hoc Gatekeeper copy.
- CI macOS packaging now runs the same downloaded-installer rehearsal before
  uploading the internal installer artifact, then writes the summary to the job.
- Release workflows now also write downloaded-release rehearsal summaries to
  the run summary before uploading the internal rehearsal artifact.
- Downloaded release rehearsals now reject extra assets by default; internal CI
  package directories must opt in before allowing build-only extras.
- Release manifests now carry the Gatekeeper guidance that downloaded-release
  rehearsals display and validate.
- Packaged smoke receipts now group checks into user-journey stages so release
  rehearsals show what the installer actually proved.
- Downloaded-release rehearsal summaries now include the manual prerelease
  acceptance path for a fresh unsigned DMG.
- Release rehearsal summaries now include the publish decision boundary:
  automated rehearsal can pass while visible ad-hoc prerelease still requires
  manual acceptance.
- CI change detection now treats release rehearsal contract changes as package
  impacting and reruns all lanes when the classifier itself changes.
- Workflow policy tests now enforce the release staging order so assets cannot
  become visible before downloaded-asset rehearsal passes.
- CI lanes are change-aware, policy-tested, and split so validation and package
  assembly stay fast without hiding release-impacting changes.

Latest review decisions:

- Keep Web Search as retrieval for now; do not build generic local RAG yet.
- Keep connector expansion narrow until one local cowork loop is excellent.
- Do not bundle Git, model weights, package-manager payloads, extra
  architectures, or unused runtimes.
- Package resources must exclude generated caches, bytecode, and model weights.
- Keep development planning concise; move completed detail to history, tests, or
  release notes.
- Treat M12 as the active lane. New work should close release proof gaps, not
  expand product scope before the first usable prerelease.

## M10: Cowork Daily Driver

Goal: make one local cowork loop feel complete in the packaged app.

Build now:

- Packaged first-run rehearsal: prove the unsigned DMG path from download,
  checksum, Gatekeeper guidance, launch, model download, workspace open, Web
  Search, approval, safe execution, proof, and runtime recovery.
- Download rehearsal: after assets are downloaded from GitHub Release, validate
  the directory and produce one compact summary before manual app opening,
  including what the first screen should guide the user to do.
- First app-open guidance: keep the header, readiness strip, and starter prompt
  choices aligned; readiness must not silently choose a prompt for the user.
- Publish proof: verify the final GitHub Release page state and asset set after
  upload, including asset readiness and user download URLs, not just before it.
- Receipts: every meaningful tool decision has a compact, actionable receipt.
- Recovery proof: readiness chips, timeline warnings, inspector detail, release
  copy, and package metadata all point to the same local fix.
- UI restraint: admin detail stays progressive and never crowds the cowork loop.

Exit when:

- A fresh unsigned DMG install can download the default model, open a workspace,
  run a cowork turn, use Web Search, request approval, execute safely, show
  proof, and recover from runtime failure.
- Runtime readiness, app copy, package metadata, release notes, and smoke tests
  all describe the same daily-driver contract.
- The main surface remains calm, with evidence available on demand.

## M11: Connector Platform

Goal: make third-party local connectors safe and useful without building a
marketplace too early.

- Keep Notion as the reference contract.
- Generalize connector permission recovery beyond Web Search only after the
  Notion path proves credentials, approvals, retries, proof, memory capture, and
  timeline evidence stay generic.
- Add import/distribution only after connector secrets can be installed, used,
  revoked, and forgotten safely.
- Treat hooks as verification and automation points first, not arbitrary
  always-on automation.

## M12: Public Release

Goal: ship a usable macOS installer from GitHub Releases.

- Public assets stay limited to DMG, checksum, `README-FIRST.txt`, and release
  manifest.
- Use release dry-run before the first visible prerelease to inspect the exact
  installer assets, release plan, and rehearsal summary without mutating GitHub
  Releases.
- Treat release rehearsal as a publish gate, not a substitute for manual
  first-launch acceptance on a fresh Mac.
- Run one full ad-hoc prerelease rehearsal: download from GitHub Release, verify
  checksum, open DMG, handle Gatekeeper, download the default model, open a
  workspace, run a cowork turn, and inspect proof.
- Developer ID notarization is optional later; unsigned prereleases must clearly
  explain Gatekeeper manual approval.
- No bundled model weights, package-manager payloads, extra architectures, or
  unused runtimes.

## Guardrails

- No hosted model dependency.
- No required Pith login, account, hosted user session, or subscription gate.
- No generic local vector database before Web Search and workspace context are
  reliable.
- No multi-agent orchestration before the single cowork loop is excellent.
- No marketplace or remote MCP transport until local connector execution is safe
  and useful.
- No bundled Git runtime until bounded system Git proves insufficient for real
  packaged users.
- No cosmetic refactor that only moves code around.
- English-only source, docs, commits, branches, and PR text.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage,
  model manifest validation, and macOS app packaging.
