# Pith Development Plan

## North Star

Pith is a small, strong, local-first macOS cowork agent for real daily work.

- Product: `Pith`, macOS 12+, `x86_64` only.
- Purpose: cowork first; coding is one workflow, not the boundary.
- Intelligence: local model by default, no required hosted model API, one active
  local model at a time.
- First use: in-app verified GGUF download, defaulting to `LFM2.5-350M`.
- Retrieval: Web Search is the active retrieval layer; no generic local document
  RAG until the cowork loop is excellent.
- Plugins: real local capabilities and connectors, not prompt templates.
- Delivery: users install a downloadable macOS app package; CI proves the app
  path, but the app experience is the product.

## Product Shape

Learn from Codex and Claude Code at the durable boundaries: workspace context,
bounded file and shell tools, Web Search retrieval, reviewable diffs, approvals,
sandbox status, session continuity, and MCP-style local connectors.

Pith should differ intentionally: local-first inference, cowork-first tasks,
small-model constraints, no required hosted model API, and no marketplace shell
before a real connector workflow is excellent.

## Architecture Boundaries

- `apps/pith-macos`: native UI, setup, timeline, approvals, model manager, and
  app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, request routing, notifications,
  request supervision, and lock boundaries.
- `crates/pith-core`: orchestration, turn lifecycle, permissions, context,
  memory use, and plugin execution.
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

Milestones 1-8 are closed. Keep implementation detail in git history, not in
this roadmap.

Closed capabilities:

- First-use model setup, resumable downloads, verified activation, runtime
  recovery, bounded model/shell work, workspace-safe tools, Web Search,
  sandbox diagnostics, compact context packing, and progressive UI surfaces.
- Plugin registry, inspect-before-install, enable/disable, connector auth,
  bounded runners, one-shot MCP stdio commands, permission gates, approval
  gates, output envelopes, retry flows, and runner memory capture.
- Practical cowork loop: bounded Plan/Act/Observe execution, Web Search
  retrieval, connector-backed plugin commands, approval pause/resume, safe
  review-summary writes, saved artifacts, and structured handoff metadata.
- Package proof: x86_64 app bundle, internal DMG workflow, mounted-DMG smoke,
  release-state safety, native sandbox fallback disclosure, and unsigned
  distribution path with optional Developer ID upgrade later.

## Closed Milestone: M8 Release Candidate

Goal: prove Pith works as a real downloadable macOS app for non-developer users.

Done:

- Release assets are DMG, basename-only SHA-256 checksum, `README-FIRST.txt`,
  and a machine-readable release manifest.
- Package metadata, release sidecars, install copy, app trust copy, distribution
  validation, and workflow policy now agree on model delivery, signing mode,
  source commit, sandbox fallback, and workflow proof.
- Packaged smoke covers first-use model metadata, app-owned model activation,
  workspace opening, Web Search source snapshots, approval-gated writes,
  connector draft/publish proof, runtime restart, and recovery of readiness
  state.
- Package gates reject bundled model weights, unsafe zip entries, path
  traversal, symlink leakage, non-`x86_64` outputs, and untracked release asset
  drift.

Keep M8 healthy by preserving these release gates:

- A release-candidate workflow proves the downloadable DMG, checksum, install
  guide, manifest, source commit, first-run model path, workspace path, Web
  Search evidence, approval-gated write path, connector smoke, and runtime
  recovery together in CI.
- Release builds run the remote model catalog audit before publishing so the
  first-use download path is checked against current upstream metadata.
- The release manifest remains enough for a user or maintainer to verify what
  was built, from which source commit, by which workflow run, and with which
  trust and sandbox posture.
- Packaged smoke stays focused on real user journeys. Add UI automation only
  when the app has a stable UI automation harness.

## Current Milestone: M9 Cowork Connectors

Goal: make Pith useful for real non-code cowork tasks without turning the app
into a marketplace shell or a generic RAG product.

Current review:

- Aligned: Notion now has a real create-page workflow contract: draft, inspect,
  approval, publish, proof, retry, command coverage, and bounded loop budget.
- Aligned: Notion setup and publish input now guide the local integration token,
  shared parent-page requirement, and approval-before-write behavior.
- Aligned: Notion retry distinguishes preserved-input retries from editable
  missing-parent recovery, so users can repair the publish input instead of
  looping the same failed command.
- Aligned: connector workflow rules now have a reusable bundled-contract check,
  not only a Notion-specific smoke path.
- Aligned: Web Search remains the retrieval layer; saved artifacts and memory
  remain context aids, not a local document RAG product.
- Risk: a second connector should not start until the Notion path is boringly
  reliable through packaged smoke and user-facing recovery.

M9 exit criteria:

- Notion setup clearly explains local integration tokens, required scopes, and
  why OAuth is not claimed yet.
- Notion create-page works as one daily cowork loop: prepare a draft, inspect
  the remote write, request approval, publish, show page proof, capture memory,
  and offer retry without losing input.
- Connector workflow contracts are reusable: manifest workflow, command
  bindings, output envelopes, proof attributes, step budget, and CI contract
  checks all travel together.
- The app keeps connector UI progressive: show readiness, repair, workflow,
  and proof when needed; avoid an always-open admin surface.
- Packaged smoke proves the connector path, Web Search evidence, workspace
  approval, runtime recovery, and unsigned DMG install path together.

Next development order:

- Keep tightening Notion recovery, proof, and packaged smoke before adding
  breadth.
- Start a second connector only after the Notion workflow stays green and the
  second connector has a real cowork use case with inspect, approval, proof, and
  retry.

## Next Milestone: M10 Connector Breadth

Goal: add one more useful cowork connector without turning Pith into a
marketplace shell.

Candidate scope:

- Pick one connector with a narrow daily workflow, not a broad platform clone.
- Reuse M9 contracts exactly: local credential, manifest workflow, bounded
  runner, inspect-before-write, approval, proof, retry, and packaged smoke.
- Keep the native UI small; expand surfaces only where the workflow needs setup,
  action, or proof.

## Guardrails

- No hosted model dependency.
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
