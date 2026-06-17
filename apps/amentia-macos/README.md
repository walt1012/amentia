# Amentia macOS App

This package contains the native macOS shell for `Amentia`.

Primary goals:

- native Intel Mac experience
- workspace and thread navigation
- timeline-centered task execution UI
- runtime bridge over `stdio`
- local model setup and recovery without external model APIs

## Source Layout

The app target is intentionally organized by product domain, not by helper type:

- `App`: application shell, top-level view model, shared app models, distribution trust copy, and platform services.
- `Runtime`: stdio bridge, JSON-RPC protocol payloads, runtime launch, readiness, and state mapping.
- `LocalModels`: first-use model setup, catalog, download, verification, activation, and model panel UI.
- `Plugins`: plugin discovery, install state, manager UI, and action planning.
- `Memory`: memory status, note presentation, and memory panel UI.
- `Timeline`: thread timeline presentation, inspector state, composer status, and session actions.
- `Workspace`: workspace search state and UI.
- `Setup`: first-run setup callouts and progress presentation.

New macOS app code should land in the domain that owns the product behavior. Avoid creating one-off
helper files at the target root; if a boundary is unclear, prefer tightening the owning domain model
before extracting another file.

Connector UI should render generic workflow, approval, proof, retry, and evidence concepts first.
Keep service-specific proof copy in narrow presenter helpers or connector attributes so adding the
next connector does not turn timeline presentation into a service-specific switchboard.

## Local Runtime Bridge

For local development, point the app at a built runtime executable with:

```bash
export AMENTIA_RUNTIME_PATH=/absolute/path/to/amentia-runtime-bin
```

The shell will use that executable when the runtime launch action is triggered.

## Packaging

Use the repository-level packaging script for release-shaped artifacts:

```bash
python3 scripts/package_macos_app.py
```

The script builds the Swift shell and `amentia-runtime-bin`, assembles `Amentia.app`, places executables in
`Contents/MacOS`, and includes bundled plugin manifests and model metadata in `Contents/Resources`.
CI packages that app into the x86_64 DMG installer, validates the product metadata, first-use model
metadata, bundled plugin resources, absence of model weights, and installer asset contract before
uploading the user-facing artifact.
