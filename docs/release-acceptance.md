# Release Acceptance

Use this maintainer-only flow before making an ad-hoc prerelease visible.

## Inputs

- A successful release workflow run for the candidate tag.
- The four downloaded release assets in one local folder:
  - `Amentia-<tag>-macos-x86_64.dmg`
  - `Amentia-<tag>-macos-x86_64.dmg.sha256`
  - `README-FIRST.txt`
  - `Amentia-<tag>-release-manifest.json`
- A fresh Mac or clean macOS user profile.

## Generate Receipt

```bash
python3 scripts/manual_acceptance_contract.py \
  --tag <tag> \
  --asset-dir <downloaded-assets> \
  --template-output manual-acceptance.json
```

Fill every field after the real install pass. Do not pre-accept a build before
running the app from the downloaded DMG.

## Required Pass

- Verify the DMG checksum from the download folder.
- Review the release manifest.
- Open the DMG and move `Amentia.app` to Applications.
- Complete the unsigned Gatekeeper path when Developer ID signing is absent.
- Download and activate the default local model.
- Open a workspace.
- Complete one cowork turn.
- Inspect the Web Search receipt.
- Approve one safe local action after reviewing the diff receipt.
- Restart Amentia and confirm recovery.
- Delete a test session and revert one approved session change.
- Run Reset Amentia and confirm no unexpected app-owned residue remains.
- Install, disable, and remove the reference plugin path used for acceptance.
- Confirm no Amentia login is required.

## Validate Receipt

```bash
python3 scripts/manual_acceptance_contract.py \
  --tag <tag> \
  --receipt manual-acceptance.json
```

Publish the validated manual acceptance JSON in a repository-scoped HTTPS
location, such as a GitHub issue created from the release acceptance template.
Use that exact URL as `manual_acceptance_receipt_url`.

## Publish Visible Ad-Hoc Prerelease

Run the `Release` workflow manually:

- `tag`: `<tag>`
- `draft`: `false`
- `prerelease`: `true`
- `publish_untrusted_ad_hoc`: `true`
- `manual_acceptance_confirmed`: `true`
- `manual_acceptance_receipt_url`: repository URL for the validated receipt
- `dry_run`: `false`

If acceptance fails, keep the release draft-only, fix `main`, and cut a new tag.
Do not move a failed public release back to draft through automation.
