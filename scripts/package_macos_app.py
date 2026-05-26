#!/usr/bin/env python3
"""Build a signed-ready x86_64 macOS app bundle for Pith."""

from __future__ import annotations

import argparse
import json
import os
import plistlib
import re
import shutil
import stat
import subprocess
import sys
import zipfile
from pathlib import Path, PurePosixPath

from macos_llama_backend import (
  LLAMA_BACKEND_EXECUTABLE_NAME,
  LLAMA_BACKEND_LIB_DIRECTORY_NAME,
  assert_portable_llama_backend,
  stage_llama_backend,
)
from release_identity import normalize_product_version


APP_NAME = "Pith"
APP_EXECUTABLE_NAME = "Pith"
SWIFT_EXECUTABLE_NAME = "PithApp"
RUNTIME_EXECUTABLE_NAME = "pith-runtime-bin"
LLAMA_BACKEND_RELATIVE_PARENT = Path("tools/llama.cpp")
DEFAULT_BUNDLE_ID = "app.pith.Pith"
DEFAULT_VERSION = "0.1.0"
PACKAGE_MANIFEST_SCHEMA_VERSION = 1
DEFAULT_SOURCE_COMMIT = (
  os.environ.get("PITH_SOURCE_COMMIT")
  or os.environ.get("GITHUB_SHA")
  or "development"
)
SUPPORTED_ARCH = "x86_64"
SOURCE_COMMIT_HEX_LENGTH = 40
SIGNING_MODES = {"unsigned", "ad-hoc", "developer-id"}
PROHIBITED_MODEL_SUFFIXES = {".gguf", ".bin", ".safetensors"}
DEFAULT_MODEL_ID = "lfm2.5-350m"
DEFAULT_MODEL_MANIFEST_RELATIVE_PATH = Path(
  "models/builtin/lfm2.5-350m/model-pack.json"
)
REQUIRED_ZIP_BASE_ENTRIES = {
  "Pith.app/Contents/Info.plist",
  "Pith.app/Contents/MacOS/Pith",
  "Pith.app/Contents/MacOS/pith-runtime-bin",
  "Pith.app/Contents/Resources/PithPackage.json",
}
REQUIRED_PACKAGED_MODEL_FIELDS = {
  "id",
  "display_name",
  "file_name",
  "context_size",
  "model_context_size",
  "max_output_tokens",
  "backend",
  "homepage",
  "download_url",
  "sha256",
  "size_bytes",
  "license",
}
REQUIRED_INFO_PLIST_VALUES = {
  "CFBundleDevelopmentRegion": "en",
  "CFBundleDisplayName": APP_NAME,
  "CFBundleExecutable": APP_EXECUTABLE_NAME,
  "CFBundleIdentifier": DEFAULT_BUNDLE_ID,
  "CFBundleInfoDictionaryVersion": "6.0",
  "CFBundleName": APP_NAME,
  "CFBundlePackageType": "APPL",
  "CFBundleSupportedPlatforms": ["MacOSX"],
  "LSApplicationCategoryType": "public.app-category.productivity",
  "LSArchitecturePriority": [SUPPORTED_ARCH],
  "LSMinimumSystemVersion": "12.0",
  "NSHighResolutionCapable": True,
  "NSPrincipalClass": "NSApplication",
  "NSSupportsAutomaticTermination": True,
  "NSSupportsSuddenTermination": True,
}
REQUIRED_BUNDLED_PLUGIN_CAPABILITIES = {
  "notion-connector": {
    "command:notion.prepare-page-draft",
    "command:notion.inspect-page-write",
    "command:notion.publish-page-draft",
    "connector:notion",
    "connector_workflow:notion.create-page",
    "mcp_server:notion",
  },
  "review-assistant": {"command:review.inspect-diff"},
  "shell-recorder": {"command:shell.summarize-session", "hook:shell.recorder"},
  "web-search": {"tool:web_search"},
  "workspace-notes": {"command:workspace.capture-note"},
}
REQUIRED_APP_COPY_SNIPPETS = (
  "Recovery: launch the runtime",
  "paused downloads",
  "selected model state are read from local storage",
  "to keep resume data",
  "cancel to clear the partial file",
  "from saved resume data",
  "reinstall metadata if readiness still fails",
  "Open Anyway",
  "Control-click Pith.app",
)


def run(command: list[str], cwd: Path) -> str:
  print(f"+ {' '.join(command)}", flush=True)
  completed = subprocess.run(
    command,
    cwd=cwd,
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
  )
  if completed.stdout:
    print(completed.stdout, end="")
  if completed.returncode != 0:
    raise RuntimeError(f"command failed with status {completed.returncode}: {' '.join(command)}")
  return completed.stdout.strip()


def parse_args() -> argparse.Namespace:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument(
    "--repo-root",
    type=Path,
    default=Path(__file__).resolve().parents[1],
    help="Repository root. Defaults to the parent of the scripts directory.",
  )
  parser.add_argument(
    "--dist-dir",
    type=Path,
    default=Path("artifacts/macos"),
    help="Output directory for Pith.app and the zip artifact.",
  )
  parser.add_argument(
    "--configuration",
    default="release",
    choices=("debug", "release"),
    help="Swift and Rust build configuration.",
  )
  parser.add_argument(
    "--arch",
    default=SUPPORTED_ARCH,
    choices=(SUPPORTED_ARCH,),
    help="Swift build architecture. Pith ships x86_64 macOS artifacts.",
  )
  parser.add_argument(
    "--runtime-binary",
    type=Path,
    help="Use an existing pith-runtime-bin instead of building one.",
  )
  parser.add_argument(
    "--llama-binary",
    type=Path,
    help="Use an existing llama.cpp llama-cli backend for packaged local inference.",
  )
  parser.add_argument(
    "--app-binary",
    type=Path,
    help="Use an existing PithApp executable instead of the default Swift build output.",
  )
  parser.add_argument(
    "--skip-build",
    action="store_true",
    help="Package already-built Swift and runtime binaries.",
  )
  parser.add_argument(
    "--no-zip",
    action="store_true",
    help="Leave only the app bundle in the dist directory.",
  )
  parser.add_argument(
    "--skip-ad-hoc-sign",
    action="store_true",
    help="Skip free ad-hoc codesign verification. CI should keep this enabled.",
  )
  parser.add_argument(
    "--version",
    default=os.environ.get("PITH_RELEASE_VERSION", DEFAULT_VERSION),
    help="App bundle version, normally derived from the release tag without the leading v.",
  )
  parser.add_argument(
    "--source-commit",
    default=DEFAULT_SOURCE_COMMIT,
    help="Source commit recorded in PithPackage.json.",
  )
  parser.add_argument(
    "--signing-mode",
    default="ad-hoc",
    choices=sorted(SIGNING_MODES),
    help="Signing state recorded in PithPackage.json.",
  )
  parser.add_argument(
    "--stage-llama-backend",
    type=Path,
    help="Stage a portable llama.cpp backend directory and exit.",
  )
  parser.add_argument(
    "--stage-llama-output",
    type=Path,
    help="Output directory for --stage-llama-backend.",
  )
  return parser.parse_args()


def build_swift_app(repo_root: Path, configuration: str, arch: str) -> Path:
  package_root = repo_root / "apps" / "pith-macos"
  swift_command = [
    "swift",
    "build",
    "--package-path",
    str(package_root),
    "-c",
    configuration,
    "--arch",
    arch,
  ]
  run(swift_command, repo_root)
  bin_path = run(swift_command + ["--show-bin-path"], repo_root)
  executable = Path(bin_path) / SWIFT_EXECUTABLE_NAME
  require_file(executable, "Swift app executable")
  return executable


def build_runtime(repo_root: Path, configuration: str) -> Path:
  command = ["cargo", "build", "-p", RUNTIME_EXECUTABLE_NAME]
  target_dir = repo_root / "target" / "debug"
  if configuration == "release":
    command.append("--release")
    target_dir = repo_root / "target" / "release"

  run(command, repo_root)
  executable = target_dir / RUNTIME_EXECUTABLE_NAME
  require_file(executable, "runtime executable")
  return executable


def package_app(
  repo_root: Path,
  dist_dir: Path,
  app_binary: Path,
  runtime_binary: Path,
  llama_binary: Path | None,
  arch: str,
  version: str,
  source_commit: str,
  signing_mode: str,
  skip_ad_hoc_sign: bool,
  no_zip: bool,
) -> Path | None:
  validate_swift_package_rules(repo_root)
  version = normalize_version(version)
  source_commit = normalize_source_commit(source_commit)
  if signing_mode not in SIGNING_MODES:
    raise RuntimeError(f"Unsupported signing mode: {signing_mode}")
  if signing_mode == "ad-hoc" and skip_ad_hoc_sign:
    signing_mode = "unsigned"

  app_path = dist_dir / f"{APP_NAME}.app"
  contents_path = app_path / "Contents"
  macos_path = contents_path / "MacOS"
  resources_path = contents_path / "Resources"

  reset_directory(app_path)
  macos_path.mkdir(parents=True)
  resources_path.mkdir(parents=True)

  write_info_plist(contents_path / "Info.plist", version)
  write_package_manifest(
    resources_path / "PithPackage.json",
    arch,
    version,
    source_commit,
    signing_mode,
  )
  (contents_path / "PkgInfo").write_text("APPL????\n", encoding="utf-8")

  copy_executable(app_binary, macos_path / APP_EXECUTABLE_NAME)
  copy_executable(runtime_binary, macos_path / RUNTIME_EXECUTABLE_NAME)
  copy_tree_if_present(repo_root / "models", resources_path / "models")
  copy_tree_if_present(repo_root / "plugins", resources_path / "plugins")
  copy_required_llama_backend(repo_root, resources_path, llama_binary)

  validate_app_bundle(app_path, arch, version, source_commit, signing_mode)
  if not skip_ad_hoc_sign:
    sign_app_bundle_if_available(app_path)
    validate_app_signature_if_available(app_path)
  if no_zip:
    return None

  zip_path = dist_dir / f"{APP_NAME}-macos-x86_64.zip"
  create_zip(app_path, zip_path)
  return zip_path


def reset_directory(path: Path) -> None:
  if path.exists():
    shutil.rmtree(path)
  path.mkdir(parents=True)


def require_file(path: Path, label: str) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Missing {label}: {path}")


def validate_swift_package_rules(repo_root: Path) -> None:
  package_path = repo_root / "apps" / "pith-macos" / "Package.swift"
  text = package_path.read_text(encoding="utf-8")
  required_fragments = {
    "minimum macOS 12 platform": ".macOS(.v12)",
    "PithApp executable product": 'name: "PithApp"',
    "PithApp source target": 'path: "Sources/PithApp"',
  }
  for label, fragment in required_fragments.items():
    if fragment not in text:
      raise RuntimeError(f"Swift package is missing {label}: {package_path}")


def normalize_version(version: str) -> str:
  return normalize_product_version(version)


def normalize_source_commit(source_commit: str) -> str:
  normalized = source_commit.strip().lower()
  if normalized == "development":
    return normalized
  if len(normalized) != SOURCE_COMMIT_HEX_LENGTH:
    raise RuntimeError("Source commit must be a full SHA-1 hash or development")
  if any(character not in "0123456789abcdef" for character in normalized):
    raise RuntimeError("Source commit must be lowercase hex or development")
  return normalized


def write_info_plist(path: Path, version: str) -> None:
  info = {
    **REQUIRED_INFO_PLIST_VALUES,
    "CFBundleShortVersionString": version,
    "CFBundleVersion": version,
  }
  with path.open("wb") as file:
    plistlib.dump(info, file, sort_keys=True)


def write_package_manifest(
  path: Path,
  arch: str,
  version: str,
  source_commit: str,
  signing_mode: str,
) -> None:
  source_commit = normalize_source_commit(source_commit)
  manifest = {
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "appName": APP_NAME,
    "bundleIdentifier": DEFAULT_BUNDLE_ID,
    "bundleVersion": version,
    "sourceCommit": source_commit,
    "minimumSystemVersion": "12.0",
    "architecture": arch,
    "runtimeExecutable": RUNTIME_EXECUTABLE_NAME,
    "backendExecutable": (
      LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME
    ).as_posix(),
    "defaultModelId": DEFAULT_MODEL_ID,
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
    "modelDelivery": "in-app-download",
    "modelWeightsBundled": False,
    "modelMetadataBundled": True,
    "bundledPluginsIncluded": True,
    "sandboxMode": "workspaceReadWrite",
    "sandboxBackend": "runtime-detected",
    "sandboxFallback": "processOnlyWhenNativeUnavailable",
    "sandboxNetworkDefault": "disabled",
    "signing": signing_mode,
  }
  path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def copy_executable(source: Path, destination: Path) -> None:
  require_file(source, "executable")
  shutil.copy2(source, destination)
  current_mode = destination.stat().st_mode
  destination.chmod(current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def copy_tree_if_present(source: Path, destination: Path) -> None:
  if not source.exists():
    return
  assert_copy_source_has_no_symlinks(source)
  ignore = shutil.ignore_patterns("*.gguf", "*.bin", "*.safetensors", ".DS_Store")
  shutil.copytree(source, destination, ignore=ignore)


def assert_copy_source_has_no_symlinks(source: Path) -> None:
  if source.is_symlink():
    raise RuntimeError(f"Packaged resource source must not be a symlink: {source}")
  symlink = next((path for path in source.rglob("*") if path.is_symlink()), None)
  if symlink is not None:
    raise RuntimeError(f"Packaged resources must not contain symlinks: {symlink}")


def copy_required_llama_backend(
  repo_root: Path,
  resources_path: Path,
  provided_backend: Path | None,
) -> Path:
  candidates = [
    provided_backend,
    repo_root / "third_party" / "llama.cpp" / "llama-cli",
    repo_root / "tools" / "llama.cpp" / "llama-cli",
  ]
  for candidate in candidates:
    if candidate is None:
      continue
    if not candidate.is_file():
      continue
    target_directory = resources_path / LLAMA_BACKEND_RELATIVE_PARENT
    return stage_llama_backend(candidate, target_directory)

  searched = ", ".join(str(candidate) for candidate in candidates if candidate is not None)
  raise FileNotFoundError(
    "Missing llama.cpp backend for packaged local inference. "
    f"Pass --llama-binary or place llama-cli at one of: {searched}"
  )


def validate_app_bundle(
  app_path: Path,
  expected_arch: str,
  expected_version: str,
  expected_source_commit: str,
  expected_signing_mode: str,
) -> None:
  required_paths = [
    app_path / "Contents" / "Info.plist",
    app_path / "Contents" / "Resources" / "PithPackage.json",
    app_path / "Contents" / "PkgInfo",
    app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME,
    app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME,
    app_path / "Contents" / "Resources" / LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME,
    app_path / "Contents" / "Resources" / DEFAULT_MODEL_MANIFEST_RELATIVE_PATH,
    app_path / "Contents" / "Resources" / "plugins" / "bundled",
  ]
  for path in required_paths:
    if not path.exists():
      raise FileNotFoundError(f"Packaged app is missing {path}")

  assert_executable(app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME)
  assert_executable(app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME)
  assert_executable(
    app_path
    / "Contents"
    / "Resources"
    / LLAMA_BACKEND_RELATIVE_PARENT
    / LLAMA_BACKEND_EXECUTABLE_NAME
  )
  assert_info_plist_matches_product_rules(app_path, expected_version)
  assert_pkg_info_matches_app_bundle(app_path)
  assert_package_manifest_matches_bundle(
    app_path,
    expected_arch,
    expected_version,
    expected_source_commit,
    expected_signing_mode,
  )
  assert_only_x86_64_if_lipo_is_available(app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME)
  assert_only_x86_64_if_lipo_is_available(app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME)
  assert_only_x86_64_if_lipo_is_available(
    app_path
    / "Contents"
    / "Resources"
    / LLAMA_BACKEND_RELATIVE_PARENT
    / LLAMA_BACKEND_EXECUTABLE_NAME
  )
  assert_portable_llama_backend(
    app_path / "Contents" / "Resources" / LLAMA_BACKEND_RELATIVE_PARENT
  )
  assert_llama_backend_dependencies_match_arch(app_path)
  assert_packaged_app_copy_is_present(app_path)
  assert_no_model_weights_are_bundled(app_path)
  assert_packaged_model_manifest_is_downloadable(app_path)
  assert_bundled_plugins_are_package_ready(app_path)


def assert_llama_backend_dependencies_match_arch(app_path: Path) -> None:
  lib_directory = (
    app_path
    / "Contents"
    / "Resources"
    / LLAMA_BACKEND_RELATIVE_PARENT
    / LLAMA_BACKEND_LIB_DIRECTORY_NAME
  )
  if not lib_directory.is_dir():
    return
  for dylib in sorted(lib_directory.glob("*.dylib")):
    assert_only_x86_64_if_lipo_is_available(dylib)


def assert_info_plist_matches_product_rules(app_path: Path, expected_version: str) -> None:
  info_path = app_path / "Contents" / "Info.plist"
  with info_path.open("rb") as file:
    info = plistlib.load(file)

  if not isinstance(info, dict):
    raise RuntimeError(f"Info.plist must decode to a dictionary: {info_path}")
  for field, expected_value in REQUIRED_INFO_PLIST_VALUES.items():
    if info.get(field) != expected_value:
      raise RuntimeError(
        f"Info.plist field {field} must be {expected_value!r}: {info_path}"
      )
  for field in ("CFBundleShortVersionString", "CFBundleVersion"):
    if info.get(field) != expected_version:
      raise RuntimeError(
        f"Info.plist field {field} must be {expected_version!r}: {info_path}"
      )


def assert_pkg_info_matches_app_bundle(app_path: Path) -> None:
  pkg_info_path = app_path / "Contents" / "PkgInfo"
  value = pkg_info_path.read_text(encoding="utf-8")
  if value != "APPL????\n":
    raise RuntimeError(f"PkgInfo must identify a macOS application bundle: {pkg_info_path}")


def assert_package_manifest_matches_bundle(
  app_path: Path,
  expected_arch: str,
  expected_version: str,
  expected_source_commit: str,
  expected_signing_mode: str,
) -> None:
  manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
  manifest = read_json_object(manifest_path)
  expected_values = {
    "schemaVersion": PACKAGE_MANIFEST_SCHEMA_VERSION,
    "appName": APP_NAME,
    "bundleIdentifier": DEFAULT_BUNDLE_ID,
    "bundleVersion": expected_version,
    "sourceCommit": expected_source_commit,
    "minimumSystemVersion": "12.0",
    "architecture": expected_arch,
    "runtimeExecutable": RUNTIME_EXECUTABLE_NAME,
    "backendExecutable": (
      LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME
    ).as_posix(),
    "defaultModelId": DEFAULT_MODEL_ID,
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
    "modelDelivery": "in-app-download",
    "sandboxMode": "workspaceReadWrite",
    "sandboxBackend": "runtime-detected",
    "sandboxFallback": "processOnlyWhenNativeUnavailable",
    "sandboxNetworkDefault": "disabled",
    "signing": expected_signing_mode,
  }
  for field, expected_value in expected_values.items():
    if manifest.get(field) != expected_value:
      raise RuntimeError(
        f"Package manifest field {field} must be {expected_value!r}: {manifest_path}"
      )

  if required_bool_field(manifest, "modelWeightsBundled", manifest_path) is not False:
    raise RuntimeError("Package manifest must not claim bundled model weights")
  if not required_bool_field(manifest, "modelMetadataBundled", manifest_path):
    raise RuntimeError("Package manifest must include bundled model metadata")
  if not required_bool_field(manifest, "bundledPluginsIncluded", manifest_path):
    raise RuntimeError("Package manifest must include bundled plugins")

  model_manifest_path = (
    app_path
    / "Contents"
    / "Resources"
    / required_string_field(manifest, "defaultModelManifest", manifest_path)
  )
  require_file(model_manifest_path, "packaged default model manifest")


def assert_packaged_model_manifest_is_downloadable(app_path: Path) -> None:
  manifest_path = (
    app_path / "Contents" / "Resources" / DEFAULT_MODEL_MANIFEST_RELATIVE_PATH
  )
  manifest = read_json_object(manifest_path)
  missing_fields = sorted(REQUIRED_PACKAGED_MODEL_FIELDS.difference(manifest.keys()))
  if missing_fields:
    raise RuntimeError(
      f"Packaged model manifest is missing fields: {', '.join(missing_fields)}"
    )
  if manifest.get("id") != DEFAULT_MODEL_ID:
    raise RuntimeError(f"Packaged default model id must be {DEFAULT_MODEL_ID}")
  if manifest.get("backend") != "llama.cpp":
    raise RuntimeError("Packaged default model backend must be llama.cpp")

  file_name = required_string_field(manifest, "file_name", manifest_path)
  if not file_name.lower().endswith(".gguf"):
    raise RuntimeError("Packaged default model file_name must point to a GGUF file")
  if (manifest_path.parent / file_name).exists():
    raise RuntimeError("Packaged default model weights must be downloaded after first launch")

  download_url = required_string_field(manifest, "download_url", manifest_path)
  homepage = required_string_field(manifest, "homepage", manifest_path)
  if not download_url.startswith("https://"):
    raise RuntimeError("Packaged default model download_url must use HTTPS")
  if not homepage.startswith("https://"):
    raise RuntimeError("Packaged default model homepage must use HTTPS")

  sha256 = required_string_field(manifest, "sha256", manifest_path)
  if len(sha256) != 64 or any(character not in "0123456789abcdef" for character in sha256):
    raise RuntimeError("Packaged default model sha256 must be lowercase hex")

  size_bytes = required_positive_int_field(manifest, "size_bytes", manifest_path)
  context_size = required_positive_int_field(manifest, "context_size", manifest_path)
  model_context_size = required_positive_int_field(
    manifest, "model_context_size", manifest_path
  )
  max_output_tokens = required_positive_int_field(
    manifest, "max_output_tokens", manifest_path
  )
  if size_bytes <= 100_000_000:
    raise RuntimeError("Packaged default model size_bytes is unexpectedly small")
  if context_size > model_context_size:
    raise RuntimeError("Packaged default model context_size exceeds model_context_size")
  if max_output_tokens > context_size:
    raise RuntimeError("Packaged default model max_output_tokens exceeds context_size")


def assert_packaged_app_copy_is_present(app_path: Path) -> None:
  executable_path = app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME
  text = packaged_executable_text(executable_path)
  missing = [
    snippet
    for snippet in REQUIRED_APP_COPY_SNIPPETS
    if snippet not in text
  ]
  if missing:
    raise RuntimeError(
      "Packaged app executable is missing required user-facing copy: "
      + ", ".join(missing)
    )


def packaged_executable_text(executable_path: Path) -> str:
  require_file(executable_path, "packaged app executable")
  try:
    result = subprocess.run(
      ["strings", str(executable_path)],
      check=True,
      capture_output=True,
      text=True,
    )
    return result.stdout
  except (FileNotFoundError, subprocess.CalledProcessError, UnicodeDecodeError):
    return executable_path.read_bytes().decode("utf-8", errors="ignore")


def assert_bundled_plugins_are_package_ready(app_path: Path) -> None:
  bundled_root = app_path / "Contents" / "Resources" / "plugins" / "bundled"
  for plugin_id, required_capabilities in REQUIRED_BUNDLED_PLUGIN_CAPABILITIES.items():
    plugin_root = bundled_root / plugin_id
    manifest_path = plugin_root / "pith-plugin.json"
    require_file(manifest_path, f"bundled plugin manifest for {plugin_id}")
    manifest = read_json_object(manifest_path)
    if manifest.get("name") != plugin_id:
      raise RuntimeError(f"Bundled plugin manifest name mismatch: {manifest_path}")
    capabilities = plugin_capabilities(manifest_path, manifest)
    missing_capabilities = required_capabilities - capabilities
    if missing_capabilities:
      missing = ", ".join(sorted(missing_capabilities))
      raise RuntimeError(f"Bundled plugin {plugin_id} is missing capabilities: {missing}")
    assert_bundled_plugin_capability_files(plugin_root, capabilities)
    assert_bundled_plugin_connector_workflows(plugin_root, manifest, capabilities)
    assert_bundled_plugin_skill_files(plugin_root, manifest)
    assert_bundled_plugin_mcp_server_files(plugin_root, manifest, capabilities)


def plugin_capabilities(manifest_path: Path, manifest: dict) -> set[str]:
  capabilities = manifest.get("capabilities")
  if not isinstance(capabilities, list) or not capabilities:
    raise RuntimeError(f"Bundled plugin manifest must declare capabilities: {manifest_path}")
  invalid_capability = next(
    (
      capability
      for capability in capabilities
      if not isinstance(capability, str) or not capability.strip()
    ),
    None,
  )
  if invalid_capability is not None:
    raise RuntimeError(f"Bundled plugin manifest has an invalid capability: {manifest_path}")
  return set(capabilities)


def assert_bundled_plugin_capability_files(plugin_root: Path, capabilities: set[str]) -> None:
  for capability in capabilities:
    if capability.startswith("command:"):
      command_id = capability.removeprefix("command:")
      assert_safe_capability_identifier(command_id, capability)
      command_path = plugin_root / "commands" / f"{command_id}.json"
      require_file(command_path, f"bundled plugin command {command_id}")
      read_json_object(command_path)
    elif capability.startswith("hook:"):
      hook_id = capability.removeprefix("hook:")
      assert_safe_capability_identifier(hook_id, capability)
      hook_path = plugin_root / "hooks" / f"{hook_id}.json"
      require_file(hook_path, f"bundled plugin hook {hook_id}")
      read_json_object(hook_path)


def assert_bundled_plugin_connector_workflows(
  plugin_root: Path,
  manifest: dict,
  capabilities: set[str],
) -> None:
  workflows = manifest.get("connectorWorkflows", [])
  if not isinstance(workflows, list):
    raise RuntimeError(f"Bundled plugin connectorWorkflows must be a list: {plugin_root}")

  connector_ids = {
    connector.get("id")
    for connector in manifest.get("appConnectors", [])
    if isinstance(connector, dict) and isinstance(connector.get("id"), str)
  }
  workflow_ids = set()
  for workflow in workflows:
    if not isinstance(workflow, dict):
      raise RuntimeError(f"Bundled connector workflow must be an object: {plugin_root}")
    workflow_id = workflow.get("id")
    connector_id = workflow.get("connectorId")
    action = workflow.get("action")
    stages = workflow.get("stages")
    statuses = workflow.get("statuses")
    if not isinstance(workflow_id, str) or not workflow_id.strip():
      raise RuntimeError(f"Bundled connector workflow is missing id: {plugin_root}")
    assert_safe_capability_identifier(workflow_id, f"connector_workflow:{workflow_id}")
    if connector_id not in connector_ids:
      raise RuntimeError(
        f"Bundled connector workflow references an undeclared connector: {workflow_id}"
      )
    if not isinstance(action, str) or not action.strip():
      raise RuntimeError(f"Bundled connector workflow is missing action: {workflow_id}")
    if not isinstance(stages, list) or not stages:
      raise RuntimeError(f"Bundled connector workflow is missing stages: {workflow_id}")
    if not isinstance(statuses, list) or not statuses:
      raise RuntimeError(f"Bundled connector workflow is missing statuses: {workflow_id}")
    workflow_ids.add(workflow_id)
    if f"connector_workflow:{workflow_id}" not in capabilities:
      raise RuntimeError(
        f"Bundled connector workflow is missing capability: {workflow_id}"
      )

  for capability in capabilities:
    if capability.startswith("connector_workflow:"):
      workflow_id = capability.removeprefix("connector_workflow:")
      if workflow_id not in workflow_ids:
        raise RuntimeError(
          f"Bundled connector workflow capability has no declaration: {capability}"
        )

  for capability in capabilities:
    if not capability.startswith("command:"):
      continue
    command_id = capability.removeprefix("command:")
    command_path = plugin_root / "commands" / f"{command_id}.json"
    if not command_path.exists():
      continue
    command = read_json_object(command_path)
    execution = command.get("execution", {})
    if not isinstance(execution, dict):
      continue
    workflow_id = execution.get("workflowId")
    if isinstance(workflow_id, str) and workflow_id.strip() and workflow_id not in workflow_ids:
      raise RuntimeError(
        f"Bundled command {command_id} references undeclared workflow: {workflow_id}"
      )


def assert_safe_capability_identifier(identifier: str, capability: str) -> None:
  if (
    not identifier.strip()
    or identifier in {".", ".."}
    or any(character in identifier for character in "/\\:")
  ):
    raise RuntimeError(f"Bundled plugin capability has unsafe identifier: {capability}")


def assert_bundled_plugin_skill_files(plugin_root: Path, manifest: dict) -> None:
  skills = manifest.get("skills", [])
  if not isinstance(skills, list):
    raise RuntimeError(f"Bundled plugin skills must be a list: {plugin_root}")
  for skill in skills:
    if not isinstance(skill, dict):
      raise RuntimeError(f"Bundled plugin skill entry must be an object: {plugin_root}")
    skill_path = skill.get("path")
    if not isinstance(skill_path, str) or not skill_path.strip():
      raise RuntimeError(f"Bundled plugin skill entry is missing path: {plugin_root}")
    require_file(
      safe_plugin_resource_path(plugin_root, skill_path),
      f"bundled plugin skill {skill_path}",
    )


def assert_bundled_plugin_mcp_server_files(
  plugin_root: Path,
  manifest: dict,
  capabilities: set[str],
) -> None:
  mcp_servers = manifest.get("mcpServers", [])
  if not isinstance(mcp_servers, list):
    raise RuntimeError(f"Bundled plugin mcpServers must be a list: {plugin_root}")
  declared_server_ids = set()
  for server in mcp_servers:
    if not isinstance(server, dict):
      raise RuntimeError(f"Bundled plugin MCP server entry must be an object: {plugin_root}")
    server_id = server.get("id")
    if not isinstance(server_id, str) or not server_id.strip():
      raise RuntimeError(f"Bundled plugin MCP server entry is missing id: {plugin_root}")
    declared_server_ids.add(server_id)
    command = server.get("command")
    if command is None:
      continue
    if not isinstance(command, str) or not command.strip():
      raise RuntimeError(f"Bundled plugin MCP server command is invalid: {plugin_root}")
    require_file(
      safe_plugin_resource_path(plugin_root, command),
      f"bundled plugin MCP server command {server_id}",
    )
    assert_executable(safe_plugin_resource_path(plugin_root, command))

  for capability in capabilities:
    if not capability.startswith("mcp_server:"):
      continue
    server_id = capability.removeprefix("mcp_server:")
    if server_id not in declared_server_ids:
      raise RuntimeError(f"Bundled plugin is missing MCP server declaration: {capability}")


def safe_plugin_resource_path(plugin_root: Path, relative_path: str) -> Path:
  if "\\" in relative_path:
    raise RuntimeError(f"Bundled plugin resource path must use forward slashes: {relative_path}")
  candidate = Path(relative_path)
  if candidate.is_absolute() or any(part in {"", ".", ".."} for part in candidate.parts):
    raise RuntimeError(f"Bundled plugin resource path must stay inside plugin: {relative_path}")
  resolved_root = plugin_root.resolve()
  resolved_candidate = (plugin_root / candidate).resolve()
  try:
    resolved_candidate.relative_to(resolved_root)
  except ValueError as error:
    raise RuntimeError(
      f"Bundled plugin resource path resolved outside plugin: {relative_path}"
    ) from error
  return resolved_candidate


def read_json_object(path: Path) -> dict:
  try:
    value = json.loads(path.read_text(encoding="utf-8"))
  except json.JSONDecodeError as error:
    raise RuntimeError(f"Packaged JSON is invalid: {path}: {error}") from error
  if not isinstance(value, dict):
    raise RuntimeError(f"Packaged JSON must be an object: {path}")
  return value


def required_string_field(manifest: dict, field: str, path: Path) -> str:
  value = manifest.get(field)
  if not isinstance(value, str) or not value.strip():
    raise RuntimeError(f"Packaged JSON field must be a non-empty string: {path}: {field}")
  return value


def required_positive_int_field(manifest: dict, field: str, path: Path) -> int:
  value = manifest.get(field)
  if not isinstance(value, int) or value <= 0:
    raise RuntimeError(f"Packaged JSON field must be a positive integer: {path}: {field}")
  return value


def required_bool_field(manifest: dict, field: str, path: Path) -> bool:
  value = manifest.get(field)
  if not isinstance(value, bool):
    raise RuntimeError(f"Packaged JSON field must be a boolean: {path}: {field}")
  return value


def assert_no_model_weights_are_bundled(app_path: Path) -> None:
  for path in app_path.rglob("*"):
    if path.is_file() and path.suffix.lower() in PROHIBITED_MODEL_SUFFIXES:
      raise RuntimeError(f"Model weight files must stay out of the app bundle: {path}")


def assert_executable(path: Path) -> None:
  if not os.access(path, os.X_OK):
    raise PermissionError(f"Packaged executable is not executable: {path}")


def assert_only_x86_64_if_lipo_is_available(path: Path) -> None:
  if shutil.which("lipo") is None:
    return
  output = run(["lipo", "-info", str(path)], path.parent)
  architectures = parse_lipo_architectures(output)
  if architectures != {SUPPORTED_ARCH}:
    actual = ", ".join(sorted(architectures)) or "unknown"
    raise RuntimeError(
      f"Packaged binary must contain only {SUPPORTED_ARCH}: {path}: {actual}: {output}"
    )


def parse_lipo_architectures(output: str) -> set[str]:
  normalized = " ".join(output.strip().split())
  for marker in (" is architecture: ", " are: "):
    if marker in normalized:
      architectures = normalized.rsplit(marker, 1)[1]
      return {
        architecture
        for architecture in re.split(r"\s+", architectures.strip())
        if architecture
      }
  raise RuntimeError(f"Could not parse lipo architecture output: {output}")


def sign_app_bundle_if_available(app_path: Path) -> None:
  if shutil.which("codesign") is None:
    print("codesign not found; skipping ad-hoc signing validation.")
    return

  run(
    [
      "codesign",
      "--force",
      "--deep",
      "--sign",
      "-",
      str(app_path),
    ],
    app_path.parent,
  )


def validate_app_signature_if_available(app_path: Path) -> None:
  if shutil.which("codesign") is None:
    return

  run(
    [
      "codesign",
      "--verify",
      "--deep",
      "--strict",
      "--verbose=2",
      str(app_path),
    ],
    app_path.parent,
  )


def create_zip(app_path: Path, zip_path: Path) -> None:
  if zip_path.exists():
    zip_path.unlink()
  if shutil.which("ditto") is not None:
    run(
      [
        "ditto",
        "-c",
        "-k",
        "--sequesterRsrc",
        "--keepParent",
        str(app_path.name),
        str(zip_path),
      ],
      app_path.parent,
    )
    assert_zip_artifact(zip_path)
    return

  shutil.make_archive(str(zip_path.with_suffix("")), "zip", app_path.parent, app_path.name)
  assert_zip_artifact(zip_path)


def assert_zip_artifact(zip_path: Path) -> None:
  require_file(zip_path, "macOS zip artifact")
  if zip_path.suffix != ".zip":
    raise RuntimeError(f"macOS package artifact must be a zip file: {zip_path}")
  if zip_path.stat().st_size <= 0:
    raise RuntimeError(f"macOS package artifact is empty: {zip_path}")
  with zipfile.ZipFile(zip_path) as archive:
    infos = archive.infolist()

  assert_zip_entries_are_safe(zip_path, infos)
  names = {info.filename for info in infos}
  missing_entries = sorted(required_zip_entries().difference(names))
  if missing_entries:
    raise RuntimeError(
      f"macOS package artifact is missing entries: {', '.join(missing_entries)}"
    )


def required_zip_entries() -> set[str]:
  entries = set(REQUIRED_ZIP_BASE_ENTRIES)
  entries.add(
    f"Pith.app/Contents/Resources/{DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix()}"
  )
  entries.add(
    "Pith.app/Contents/Resources/"
    f"{(LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME).as_posix()}"
  )
  for plugin_id in REQUIRED_BUNDLED_PLUGIN_CAPABILITIES:
    entries.add(
      f"Pith.app/Contents/Resources/plugins/bundled/{plugin_id}/pith-plugin.json"
    )
  return entries


def assert_zip_entries_are_safe(zip_path: Path, infos: list[zipfile.ZipInfo]) -> None:
  for info in infos:
    normalized_name = info.filename.rstrip("/")
    if not normalized_name:
      continue
    if "\\" in normalized_name:
      raise RuntimeError(f"macOS zip artifact contains a backslash path: {info.filename}")
    entry_path = PurePosixPath(normalized_name)
    if entry_path.is_absolute() or any(part in {"", ".", ".."} for part in entry_path.parts):
      raise RuntimeError(f"macOS zip artifact contains an unsafe path: {info.filename}")
    if entry_path.suffix.lower() in PROHIBITED_MODEL_SUFFIXES:
      raise RuntimeError(f"Model weight files must stay out of the zip artifact: {info.filename}")
    if stat.S_IFMT(info.external_attr >> 16) == stat.S_IFLNK:
      raise RuntimeError(f"macOS zip artifact must not contain symlinks: {info.filename}")


def main() -> int:
  args = parse_args()
  repo_root = args.repo_root.resolve()

  try:
    if args.stage_llama_backend:
      output_directory = (
        args.stage_llama_output.resolve()
        if args.stage_llama_output
        else repo_root / "artifacts" / "llama-backend"
      )
      staged_backend = stage_llama_backend(args.stage_llama_backend.resolve(), output_directory)
      print(f"Staged llama.cpp backend: {staged_backend}")
      return 0

    dist_dir = (repo_root / args.dist_dir).resolve()
    dist_dir.mkdir(parents=True, exist_ok=True)

    if args.skip_build:
      app_binary = args.app_binary or repo_root / "apps" / "pith-macos" / ".build" / args.configuration / SWIFT_EXECUTABLE_NAME
      runtime_binary = args.runtime_binary or repo_root / "target" / args.configuration / RUNTIME_EXECUTABLE_NAME
      require_file(app_binary, "Swift app executable")
      require_file(runtime_binary, "runtime executable")
    else:
      app_binary = build_swift_app(repo_root, args.configuration, args.arch)
      runtime_binary = args.runtime_binary or build_runtime(repo_root, args.configuration)

    zip_path = package_app(
      repo_root,
      dist_dir,
      app_binary,
      runtime_binary,
      args.llama_binary.resolve() if args.llama_binary else None,
      args.arch,
      args.version,
      args.source_commit,
      args.signing_mode,
      args.skip_ad_hoc_sign,
      args.no_zip,
    )
  except Exception as error:
    print(f"macOS packaging failed: {error}", file=sys.stderr)
    return 1

  app_path = dist_dir / f"{APP_NAME}.app"
  print(f"Packaged app: {app_path}")
  if zip_path is not None:
    print(f"Packaged artifact: {zip_path}")
  return 0


if __name__ == "__main__":
  sys.exit(main())
