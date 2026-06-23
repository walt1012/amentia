---
name: Release acceptance receipt
about: Record validated fresh-Mac acceptance before publishing an ad-hoc prerelease.
title: "Release acceptance: <tag>"
labels: release-acceptance
assignees: ""
---

## Candidate

- Tag:
- Release workflow run:
- Source commit:
- Signing mode:
- Downloaded assets folder:

## Validation

- [ ] Generated `manual-acceptance.json` from downloaded assets.
- [ ] Generated and filled `m14-installed-app-proof.json` from the same installed-app run.
- [ ] Completed the fresh-Mac or clean-user install pass.
- [ ] Validated the installed-app proof with `scripts/installed_app_proof.py`.
- [ ] Validated the receipt with `scripts/manual_acceptance_contract.py --installed-app-proof`.
- [ ] Confirmed the receipt matches the candidate tag and DMG checksum.

## Receipt

Paste the validated JSON receipt below.

```json
{}
```

## Installed-App Proof

Paste the validated installed-app proof JSON below.

```json
{}
```

## Decision

- [ ] Accept this build for visible ad-hoc prerelease.
- [ ] Keep this build draft-only and fix issues before publishing.
