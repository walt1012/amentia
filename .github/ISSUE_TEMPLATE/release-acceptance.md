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
- [ ] Completed the fresh-Mac or clean-user install pass.
- [ ] Validated the receipt with `scripts/manual_acceptance_contract.py`.
- [ ] Confirmed the receipt matches the candidate tag and DMG checksum.

## Receipt

Paste the validated JSON receipt below.

```json
{}
```

## Decision

- [ ] Accept this build for visible ad-hoc prerelease.
- [ ] Keep this build draft-only and fix issues before publishing.
