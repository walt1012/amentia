#!/usr/bin/env python3
"""Build a signed-ready x86_64 macOS app bundle for Pith."""

from __future__ import annotations

import argparse
import json
import os
import plistlib
import shutil
import stat
import subprocess
import sys
from pathlib import Path


APP_NAME = "Pith"
APP_EXECUTABLE_NAME = "Pith"
SWIFT_EXECUTABLE_NAME = "PithApp"
RUNTIME_EXECUTABLE_NAME = "pith-runtime-bin"
DEFAULT_BUNDLE_ID = "app.pith.Pith"
DEFAULT_VERSION = "0.1.0"
SUPPORTED_ARCH = "x86_64"
PROHIBITED_MODEL_SUFFIXES = {".gguf", ".bin", ".safetensors"}
DEFAULT_MODEL_ID = "lfm2.5-350m"
DEFAULT_MODEL_MANIFEST_RELATIVE_PATH = Path(
  "models/builtin/lfm2.5-350m/model-pack.json"
)
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
  "CFBundleShortVersionString": DEFAULT_VERSION,
  "CFBundleSupportedPlatforms": ["MacOSX"],
  "CFBundleVersion": DEFAULT_VERSION,
  "LSApplicationCategoryType": "public.app-category.developer-tools",
  "LSArchitecturePriority": [SUPPORTED_ARCH],
  "LSMinimumSystemVersion": "12.0",
  "NSHighResolutionCapable": True,
  "NSPrincipalClass": "NSApplication",
  "NSSupportsAutomaticTermination": True,
  "NSSupportsSuddenTermination": True,
}
REQUIRED_BUNDLED_PLUGIN_CAPABILITIES = {
  "notion-connector": {"command:notion.prepare-page-draft"},
  "review-assistant": {"command:review.inspect-diff"},
  "shell-recorder": {"command:shell.summarize-session", "hook:shell.recorder"},
  "web-search": {"tool:web_search"},
  "workspace-notes": {"command:workspace.capture-note"},
}


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
  arch: str,
  skip_ad_hoc_sign: bool,
  no_zip: bool,
) -> Path | None:
  validate_swift_package_rules(repo_root)

  app_path = dist_dir / f"{APP_NAME}.app"
  contents_path = app_path / "Contents"
  macos_path = contents_path / "MacOS"
  resources_path = contents_path / "Resources"

  reset_directory(app_path)
  macos_path.mkdir(parents=True)
  resources_path.mkdir(parents=True)

  write_info_plist(contents_path / "Info.plist")
  write_package_manifest(resources_path / "PithPackage.json", arch)
  (contents_path / "PkgInfo").write_text("APPL????\n", encoding="utf-8")

  copy_executable(app_binary, macos_path / APP_EXECUTABLE_NAME)
  copy_executable(runtime_binary, macos_path / RUNTIME_EXECUTABLE_NAME)
  copy_tree_if_present(repo_root / "models", resources_path / "models")
  copy_tree_if_present(repo_root / "plugins", resources_path / "plugins")
  copy_llama_backend_if_present(repo_root, resources_path)

  validate_app_bundle(app_path, arch)
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


def write_info_plist(path: Path) -> None:
  with path.open("wb") as file:
    plistlib.dump(REQUIRED_INFO_PLIST_VALUES, file, sort_keys=True)


def write_package_manifest(path: Path, arch: str) -> None:
  manifest = {
    "appName": APP_NAME,
    "bundleIdentifier": DEFAULT_BUNDLE_ID,
    "bundleVersion": DEFAULT_VERSION,
    "minimumSystemVersion": "12.0",
    "architecture": arch,
    "runtimeExecutable": RUNTIME_EXECUTABLE_NAME,
    "defaultModelId": DEFAULT_MODEL_ID,
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
    "modelDelivery": "in-app-download",
    "modelWeightsBundled": False,
    "modelMetadataBundled": True,
    "bundledPluginsIncluded": True,
    "signing": "ad-hoc when codesign is available",
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
  ignore = shutil.ignore_patterns("*.gguf", "*.bin", "*.safetensors", ".DS_Store")
  shutil.copytree(source, destination, ignore=ignore)


def copy_llama_backend_if_present(repo_root: Path, macos_path: Path) -> None:
  candidates = [
    repo_root / "third_party" / "llama.cpp" / "llama-cli",
    repo_root / "tools" / "llama.cpp" / "llama-cli",
  ]
  for candidate in candidates:
    if not candidate.is_file():
      continue
    relative_parent = candidate.parent.relative_to(repo_root)
    target_directory = macos_path / relative_parent
    target_directory.mkdir(parents=True, exist_ok=True)
    copy_executable(candidate, target_directory / candidate.name)
    return


def validate_app_bundle(app_path: Path, expected_arch: str) -> None:
  required_paths = [
    app_path / "Contents" / "Info.plist",
    app_path / "Contents" / "Resources" / "PithPackage.json",
    app_path / "Contents" / "PkgInfo",
    app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME,
    app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME,
    app_path / "Contents" / "Resources" / DEFAULT_MODEL_MANIFEST_RELATIVE_PATH,
    app_path / "Contents" / "Resources" / "plugins" / "bundled",
  ]
  for path in required_paths:
    if not path.exists():
      raise FileNotFoundError(f"Packaged app is missing {path}")

  assert_executable(app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME)
  assert_executable(app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME)
  assert_info_plist_matches_product_rules(app_path)
  assert_pkg_info_matches_app_bundle(app_path)
  assert_package_manifest_matches_bundle(app_path, expected_arch)
  assert_only_x86_64_if_lipo_is_available(app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME)
  assert_only_x86_64_if_lipo_is_available(app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME)
  assert_no_model_weights_are_bundled(app_path)
  assert_packaged_model_manifest_is_downloadable(app_path)
  assert_bundled_plugins_are_package_ready(app_path)


def assert_info_plist_matches_product_rules(app_path: Path) -> None:
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


def assert_pkg_info_matches_app_bundle(app_path: Path) -> None:
  pkg_info_path = app_path / "Contents" / "PkgInfo"
  value = pkg_info_path.read_text(encoding="utf-8")
  if value != "APPL????\n":
    raise RuntimeError(f"PkgInfo must identify a macOS application bundle: {pkg_info_path}")


def assert_package_manifest_matches_bundle(app_path: Path, expected_arch: str) -> None:
  manifest_path = app_path / "Contents" / "Resources" / "PithPackage.json"
  manifest = read_json_object(manifest_path)
  expected_values = {
    "appName": APP_NAME,
    "bundleIdentifier": DEFAULT_BUNDLE_ID,
    "bundleVersion": DEFAULT_VERSION,
    "minimumSystemVersion": "12.0",
    "architecture": expected_arch,
    "runtimeExecutable": RUNTIME_EXECUTABLE_NAME,
    "defaultModelId": DEFAULT_MODEL_ID,
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
    "modelDelivery": "in-app-download",
    "signing": "ad-hoc when codesign is available",
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
    assert_bundled_plugin_skill_files(plugin_root, manifest)


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
  if "x86_64" not in output:
    raise RuntimeError(f"Packaged binary is not x86_64: {path}: {output}")
  if "arm64" in output:
    raise RuntimeError(f"Packaged binary must not include arm64: {path}: {output}")


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
      "--options",
      "runtime",
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


def main() -> int:
  args = parse_args()
  repo_root = args.repo_root.resolve()
  dist_dir = (repo_root / args.dist_dir).resolve()
  dist_dir.mkdir(parents=True, exist_ok=True)

  try:
    if args.skip_build:
      app_binary = repo_root / "apps" / "pith-macos" / ".build" / args.configuration / SWIFT_EXECUTABLE_NAME
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
      args.arch,
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
