# Pith Development Plan

## Mission

Pith is a small, strong, local-first macOS agent app for controlled local work.

It should feel native, focused, recoverable, and capable. It must not become a generic chatbot, web wrapper, terminal skin, hosted-model frontend, or feature zoo.

## Rules

- Product: `Pith`, macOS 12+, `x86_64` only.
- Intelligence: local model by default, no required external model API.
- First use: in-app model download, defaulting to `LFM2.5-350M`.
- Runtime: one active local model at a time.
- Repository: English-only source, docs, commits, branches, and PR text.
- Foundation: free and open source.

## Ownership

- `apps/pith-macos`: native UI, setup, approvals, timeline, model manager, inspector, and app-facing state.
- `crates/pith-runtime-bin`: JSON-RPC process, request routing, request supervision, notifications, and runtime lock boundaries.
- `crates/pith-core`: orchestration, request reducers, turn lifecycle, readiness, permissions, context packing, memory usage, and plugin baselines.
- `crates/pith-protocol`: typed wire contract.
- `crates/pith-tools`: bounded workspace tools, shell, web search, output compaction, and path safety.
- `crates/pith-sandbox`: native sandbox policy and diagnostics.
- `crates/pith-model-runtime`: local model discovery, validation, health, bounded inference, and failure wording.
- `crates/pith-memory`: memory semantics, note ranking, summaries, and context selection.
- `crates/pith-storage`: durable persistence for threads, workspace state, approvals, memory notes, and plugin state.
- `crates/pith-plugin-host`: manifests, discovery, validation, registries, connector metadata, and bundle lifecycle.

Memory and storage do not conflict: memory owns meaning and ranking; storage owns durable records.

## Review Snapshot

- Keep: short runtime locks, bounded subprocesses, symlink-safe workspace tools, verified local model activation, progressive inspector surfaces, shared cancellable app task slots.
- Watch: do not split Swift files for cosmetic size; create coordinators/stores only when ownership actually changes.
- Watch: keep `pith-core` modular enough for Milestone 4 plugin execution seams.
- Watch: plugin commands are built-in execution contracts today, not third-party plugin processes.
- Watch: CI checks internal model metadata; release work must re-check upstream URLs, sizes, checksums, and licenses.

## Retrieval And Context

- Web search is the current retrieval layer for fresh public information.
- Web search must stay default-available, permissioned, bounded, cancellable, and separate from sandbox enforcement.
- Current routing can be deterministic or explicit while tiny local models mature; later tool planning can let the model choose retrieval when reliable.
- Memory ranking is compact, attributed context selection for known notes, not generic document RAG.
- Do not add local vector stores, embedding services, generic file RAG, or document indexing until usage proves the need.

## Current Milestone: M3 Daily Driver Foundation

M3 ends when Pith is usable daily without restart anxiety or setup confusion:

- Fresh install can select, download, verify, activate, and use one local model.
- Download pause, continue, cancel, failure recovery, and partial-file cleanup are reliable.
- Runtime exit, timeout, cancellation, and model generation failure recover without app restart.
- Shell, workspace tools, model generation, web search, and helpers are bounded and inspectable.
- Native sandbox diagnostics show backend, active state, writable roots, temp root, and network policy.
- Workspace boundaries stay safe across symlinks, shell temp files, writes, search, and approvals.
- Timeline and inspector stay calm in the normal ready state.
- Context remains compact and explainable for tiny local models.
- Code is clean enough to start real plugin execution without another broad refactor.

## M3 Status

- Done: model setup is first-use guided, catalog-based, checksum verified, resumable, cancellable, and single-active-model only.
- Done: runtime requests use short locks, supervised execution lanes, bounded subprocesses, cancellable turns and approvals, relaunch recovery, panic cleanup, and stable timeline attribution.
- Done: shell, workspace tools, model inference, web search, git helpers, sandbox temp paths, and output artifacts are bounded and inspectable.
- Done: workspace writes, diffs, reads, listings, search, shell temp routing, and artifact cleanup respect symlink-safe boundaries.
- Done: web search is the current retrieval layer; generic local RAG and vector indexing remain out of scope.
- Done: local planner and summarizer prompts use compact prompt envelopes with attribution for tiny model context limits.
- Done: model release metadata can be audited against Hugging Face size, checksum, and license headers without making CI network-fragile.
- Done: inspector and setup surfaces are progressive enough for daily use without expanding every control by default.
- Active: final M3 readiness audit before moving into Milestone 4 plugin execution.

## Next Order

1. Model setup hardening: first-use guidance, activation recovery, corrupt model handling, release metadata review.
2. Runtime recovery: diagnostics, cancellation, pending request cleanup, relaunch flow.
3. Sandbox diagnostics: native vs process-only wording, temp routing, network policy, timeline attributes.
4. Context and retrieval: compact observations, memory attribution, web search shaping, no pseudo-RAG drift.
5. Swift ownership: keep `AppViewModel` as facade; move ownership only into real domain stores/coordinators.

## Milestone 4: Real Plugin Platform

- Add real plugin execution contracts, not more prompt templates.
- Add typed capability inputs and outputs.
- Add third-party auth and connectors such as Notion.
- Consider MCP only if it fits Pith's local-first, small, native direction.
- Keep plugin state, logs, failures, and outputs visible without polluting the main flow.

## Not Now

- No hosted model dependency.
- No multi-agent workflows.
- No generic document RAG or local vector database.
- No broad connector marketplace.
- No cosmetic refactor that only moves code around.
- No large UI expansion before M3 is stable.

## Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage, model manifest validation, and Swift build.
- Local validation tools are optional and should not block progress.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when it clarifies ownership or failure boundaries.
