# Evidence

This directory is for human-accepted milestone and release proof. Keep it small,
specific, and tied to a real installed-app run.

## M14 Installed-App Proof

The accepted installed-app proof belongs at:

```text
docs/evidence/m14-installed-app-proof.json
```

Create a safe template only when a maintainer is ready to fill it from a real
installed-app run:

```bash
python scripts/installed_app_proof.py --template-output docs/evidence/m14-installed-app-proof.json
```

Validate the filled evidence before committing it:

```bash
python scripts/installed_app_proof.py --evidence docs/evidence/m14-installed-app-proof.json
```

Do not commit placeholder evidence. The validator rejects placeholder text,
non-UTC acceptance times, unknown model IDs, weak cleanup proof, and unexpected
app-owned residue.

## M14 Reference Connector Proof

The accepted Notion reference connector proof belongs at:

```text
docs/evidence/m14-reference-connector-proof.json
```

Create a safe template only when a maintainer is ready to fill it from a real
installed-app run:

```bash
python scripts/reference_connector_proof.py --template-output docs/evidence/m14-reference-connector-proof.json
```

Validate the filled evidence before committing it:

```bash
python scripts/reference_connector_proof.py --evidence docs/evidence/m14-reference-connector-proof.json
```

Do not commit placeholder evidence. The validator rejects placeholder text,
non-UTC acceptance times, and stale credential state.
