## Built-in Model Pack

This directory tracks the `LFM2.5-350M` pack manifest for Pith.

What belongs in git:

- `model-pack.json`
- small text documentation
- checksums or licensing metadata

What does not belong in git history:

- `LFM2.5-350M.gguf`
- converted weight shards
- large runtime binaries

Recommended local layout:

```text
<PITH_DATA_DIR>/
`-- models/
    `-- builtin/
        `-- lfm2.5-350m/
            |-- model-pack.json
            `-- LFM2.5-350M.gguf
```

The runtime can also resolve:

- `PITH_MODEL_PACK_MANIFEST`
- `PITH_LFM_MODEL_PATH`
- repo-local manifests for development

The repository keeps the manifest so the product can describe the built-in pack without forcing the large GGUF file into source control.
