# Scripts

Repository scripts are product infrastructure, not a second application layer.
Keep them small, deterministic, and covered by Python contract tests when they
define release, CI, package, model, or workflow behavior.

## Groups

- CI policy: `ci_changes.py`, `check_english_policy.py`, and
  `validate_workflows.py` keep change lanes, repository text, and GitHub
  Actions structure predictable.
- macOS packaging: `package_macos_app.py`, `create_macos_dmg.py`,
  `package_contract.py`, `installer_artifact_contract.py`,
  `macos_llama_backend.py`, `smoke_launch_macos_app.py`, and
  `validate_macos_distribution.py` build and validate the x86_64 app bundle,
  DMG, package metadata, bundled resources, app launch smoke, and optional
  Developer ID distribution shape.
- Release: `release_identity.py`, `release_text.py`,
  `release_copy_contract.py`, `release_artifacts.py`, `release_readiness.py`,
  `release_state.py`, `release_rehearsal_contract.py`, `release_publish_contract.py`, and
  `manual_acceptance_contract.py` enforce the four public assets, dry-run-first
  release rehearsal, draft/visible safety, downloaded-release rehearsal, and
  fresh-Mac manual acceptance gate.
- Receipt helpers stay lightweight: `receipt_fields.py` only removes
  duplicated JSON and field checks used by release acceptance scripts.
- Runtime and model checks: `runtime_smoke_test.py` and
  `validate_model_pack.py` cover the runtime protocol surface and curated local
  model metadata.
- Tests: `test_*.py` files are lightweight contract tests for the scripts above
  and should stay runnable without Rust or Swift toolchains.

## Release Rules

- Public release assets are exactly the DMG, checksum, `README-FIRST.txt`, and
  release manifest.
- Internal workflow artifacts may contain executable build products or
  rehearsal files, but they are not user-facing release downloads.
- Tag-push and ordinary manual release runs default to dry-run behavior.
- Without Developer ID credentials, a visible ad-hoc prerelease requires a
  validated fresh-Mac manual acceptance receipt.
- Maintainers should use `docs/release-acceptance.md` and the release
  acceptance issue template to create the receipt URL used by the release
  workflow.
- Do not publish model weights, package-manager payloads, stale assets, or extra
  archives on the GitHub Release page.

## Local Use

Python script checks may be run locally when Python is available, but GitHub
Actions remains the source of truth. Local Rust, Swift, and macOS signing
toolchains are optional and should not block development.
