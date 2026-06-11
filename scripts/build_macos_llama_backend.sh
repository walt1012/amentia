#!/usr/bin/env bash
set -euo pipefail

: "${LLAMA_CPP_REF:?LLAMA_CPP_REF is required}"
: "${LLAMA_BACKEND_CACHE_DIR:?LLAMA_BACKEND_CACHE_DIR is required}"
: "${LLAMA_BINARY:?LLAMA_BINARY is required}"
: "${RUNNER_TEMP:?RUNNER_TEMP is required}"

source_dir="$RUNNER_TEMP/llama.cpp"
build_dir="$RUNNER_TEMP/llama.cpp-build"
build_log="$RUNNER_TEMP/llama-build.log"

retry_network() {
  local attempt=1
  local max_attempts=4
  local delay_seconds=8

  until "$@"; do
    local exit_code=$?
    if (( attempt >= max_attempts )); then
      return "$exit_code"
    fi

    echo "::warning title=llama.cpp fetch retry::Attempt $attempt failed with exit code $exit_code. Retrying in ${delay_seconds}s."
    sleep "$delay_seconds"
    attempt=$((attempt + 1))
    delay_seconds=$((delay_seconds * 2))
  done
}

rm -rf "$source_dir" "$build_dir"
git init "$source_dir"
git -C "$source_dir" remote add origin https://github.com/ggml-org/llama.cpp.git
retry_network git -C "$source_dir" fetch --depth 1 origin "$LLAMA_CPP_REF"
git -C "$source_dir" checkout --detach FETCH_HEAD

cmake \
  -S "$source_dir" \
  -B "$build_dir" \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_OSX_ARCHITECTURES=x86_64 \
  -DBUILD_SHARED_LIBS=OFF

if ! cmake --build "$build_dir" --config Release --parallel 3 > "$build_log" 2>&1; then
  tail -n 160 "$build_log"
  exit 1
fi

backend="$(find "$build_dir" -type f \( -name llama-cli -o -name main \) | head -n 1)"
test -x "$backend"

rm -rf "$LLAMA_BACKEND_CACHE_DIR"
mkdir -p "$LLAMA_BACKEND_CACHE_DIR"
python3 scripts/package_macos_app.py \
  --stage-llama-backend "$backend" \
  --stage-llama-output "$LLAMA_BACKEND_CACHE_DIR"
chmod +x "$LLAMA_BACKEND_CACHE_DIR/$LLAMA_BINARY"

if [[ -n "${PREBUILT_ARTIFACT_DIR:-}" ]]; then
  mkdir -p "$PREBUILT_ARTIFACT_DIR"
  cp -R "$LLAMA_BACKEND_CACHE_DIR"/. "$PREBUILT_ARTIFACT_DIR"/
fi
