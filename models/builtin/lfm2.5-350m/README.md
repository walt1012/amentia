## Default Model Catalog Entry

This directory tracks the default `LFM2.5-350M` catalog manifest for Amentia.

What belongs in git:

- `model-pack.json`
- small text documentation
- checksums or licensing metadata

What does not belong in git history:

- `LFM2.5-350M-Q4_K_M.gguf`
- converted weight shards
- large runtime binaries

Recommended local layout:

```text
<AMENTIA_DATA_DIR>/
`-- models/
    `-- builtin/
        `-- lfm2.5-350m/
            |-- model-pack.json
            `-- LFM2.5-350M-Q4_K_M.gguf
```

Amentia can download the recommended Q4_K_M GGUF from the catalog manifest into the suggested app data path. The download target stays local and is never tracked in git.

Manifest sizing metadata is intentionally split:

- `model_context_size` is the published model context window from the upstream model documentation.
- `context_size` is Amentia's default llama.cpp runtime window for the lightweight first-use path.
- `max_output_tokens` is Amentia's conservative generation cap, not an intrinsic model limit.

The runtime can also resolve:

- `AMENTIA_MODEL_PACK_MANIFEST`
- `AMENTIA_MODEL_PATH`
- `AMENTIA_LFM_MODEL_PATH` as a legacy alias
- repo-local manifests for development

The repository keeps the manifest so the product can describe the default downloadable model without forcing the large GGUF file into source control.
