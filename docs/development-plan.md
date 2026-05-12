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

## Milestone 3: Daily Driver Foundation

Status: closed.

M3 made Pith usable daily without restart anxiety or setup confusion:

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
- Done: plugin command registries now expose typed execution contracts, so M4 can add real plugin runners without returning to prompt-template commands.

## Current Milestone: M4 Real Plugin Platform

M4 makes plugins real local capabilities, not prompt templates:

- Commands declare typed execution contracts with driver, entrypoint, support status, inputs, and outputs.
- Built-in commands remain small local workflows; third-party commands must run through bounded, inspectable runners.
- Connectors such as Notion require explicit auth policy, credential storage, permissions, logs, and failure states.
- Plugin execution must respect workspace boundaries, sandbox policy, cancellation, timeouts, and output compaction.
- Plugin UI stays progressive: discover, inspect, enable, authorize, run, and debug without crowding the main flow.
- MCP can be considered only as a local-first connector/runtime option, not as a broad marketplace shortcut.

## Next Order

1. Add plugin contract schemas for typed command input and output envelopes.
2. Add a minimal bounded plugin runner path before adding any new connector surface.
3. Add connector auth lifecycle for one Notion-like connector path only.
4. Keep web search as the only active retrieval layer unless user workflows prove local RAG is needed.
5. Keep Swift and Rust ownership changes tied to real runtime, plugin, model, or sandbox boundaries.

## Not Now

- No hosted model dependency.
- No multi-agent workflows.
- No generic document RAG or local vector database.
- No broad connector marketplace.
- No cosmetic refactor that only moves code around.
- No large UI expansion before plugin execution is real.

## Discipline

- CI is hygiene, not a milestone.
- Remote CI is the source of truth for Rust fmt, clippy, tests, smoke coverage, model manifest validation, and Swift build.
- Local validation tools are optional and should not block progress.
- Keep commits scoped and fix CI from logs, not guesses.
- Split modules only when it clarifies ownership or failure boundaries.
