#!/usr/bin/env python3
"""Build a signed-ready x86_64 macOS app bundle for Amentia."""

from __future__ import annotations

import argparse
import json
import os
import plistlib
import re
import shutil
import stat
import struct
import subprocess
import sys
import tempfile
import zipfile
import zlib
from pathlib import Path, PurePosixPath

from macos_llama_backend import (
  LLAMA_BACKEND_EXECUTABLE_NAME,
  LLAMA_BACKEND_LIB_DIRECTORY_NAME,
  assert_portable_llama_backend,
  stage_llama_backend,
)
from package_contract import (
  APP_NAME,
  DAILY_DRIVER_CONTRACT,
  DEFAULT_MODEL_ID,
  DEFAULT_MODEL_MANIFEST_RELATIVE_PATH,
  FIRST_APP_OPEN_CONTRACT_ID,
  MINIMUM_SYSTEM_VERSION,
  MODEL_DELIVERY_MODE,
  MODEL_METADATA_BUNDLED,
  MODEL_WEIGHTS_BUNDLED,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  PACKAGE_SIGNING_MODES,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  LOCAL_EXECUTION_SAFETY_MODES,
  AMENTIA_ACCOUNT_REQUIRED,
  PROHIBITED_MODEL_SUFFIXES,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  assert_size_under_budget,
  directory_size_bytes,
  package_size_budget,
  package_distribution_trust,
  validate_package_manifest_contract,
)
from release_identity import normalize_product_version


APP_EXECUTABLE_NAME = APP_NAME
SWIFT_EXECUTABLE_NAME = "AmentiaApp"
RUNTIME_EXECUTABLE_NAME = "amentia-runtime-bin"
APP_ICON_FILE_NAME = f"{APP_NAME}.icns"
APP_ICON_SOURCE_RELATIVE_PATH = Path("docs/brand/amentia-blue-symbol-icon-candidate.png")
APP_ICON_MIN_SOURCE_SIZE = 1024
APP_ICON_MAX_TRANSPARENT_MARGIN_RATIO = 0.04
APP_ICONSET_RENDITIONS = (
  (16, 1),
  (16, 2),
  (32, 1),
  (32, 2),
  (128, 1),
  (128, 2),
  (256, 1),
  (256, 2),
  (512, 1),
  (512, 2),
)
LLAMA_BACKEND_RELATIVE_PARENT = Path("tools/llama.cpp")
DEFAULT_BUNDLE_ID = "app.amentia.Amentia"
DEFAULT_VERSION = "0.1.0"
DEFAULT_SOURCE_COMMIT = (
  os.environ.get("AMENTIA_SOURCE_COMMIT")
  or os.environ.get("GITHUB_SHA")
  or "development"
)
SOURCE_COMMIT_HEX_LENGTH = 40
DAILY_DRIVER_STAGE_SOURCE = DAILY_DRIVER_CONTRACT["stageSource"]
DAILY_DRIVER_NEXT_ACTION_SOURCE = DAILY_DRIVER_CONTRACT["nextActionSource"]
DAILY_DRIVER_PRESENTATION = DAILY_DRIVER_CONTRACT["presentation"]
FIRST_APP_OPEN_ACTION_CONTRACT = FIRST_APP_OPEN_CONTRACT_ID
REQUIRED_ZIP_BASE_ENTRIES = {
  f"{APP_NAME}.app/Contents/Info.plist",
  f"{APP_NAME}.app/Contents/MacOS/{APP_EXECUTABLE_NAME}",
  f"{APP_NAME}.app/Contents/MacOS/{RUNTIME_EXECUTABLE_NAME}",
  f"{APP_NAME}.app/Contents/Resources/{APP_ICON_FILE_NAME}",
  f"{APP_NAME}.app/Contents/Resources/AmentiaPackage.json",
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
  "CFBundleIconFile": APP_ICON_FILE_NAME,
  "CFBundleIdentifier": DEFAULT_BUNDLE_ID,
  "CFBundleInfoDictionaryVersion": "6.0",
  "CFBundleName": APP_NAME,
  "CFBundlePackageType": "APPL",
  "CFBundleSupportedPlatforms": ["MacOSX"],
  "LSApplicationCategoryType": "public.app-category.productivity",
  "LSArchitecturePriority": [SUPPORTED_ARCH],
  "LSMinimumSystemVersion": MINIMUM_SYSTEM_VERSION,
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
    "skill:notion.workspace",
  },
  "review-assistant": {
    "command:review.inspect-diff",
    "skill:review.prompts",
  },
  "shell-recorder": {"command:shell.summarize-session", "hook:shell.recorder"},
  "web-search": {"tool:web_search"},
  "workspace-notes": {
    "command:workspace.capture-note",
    "skill:workspace.notes",
  },
}
REQUIRED_APP_COPY_SNIPPETS = (
  f"Start {APP_NAME} to restore model choices",
  "paused downloads",
  "selected model choices remain local",
  "to keep resume data",
  "cancel to clear the partial file",
  "from saved resume data",
  "Refresh local model setup if readiness still fails",
  "Download Local Model",
  "Refresh Model Setup",
  "Open Anyway",
  f"Control-click {APP_NAME}.app",
  f"no {APP_NAME} account required",
  "action safety mode",
  "package size budget",
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
    help=f"Output directory for {APP_NAME}.app and the zip artifact.",
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
    help=f"Swift build architecture. {APP_NAME} ships x86_64 macOS artifacts.",
  )
  parser.add_argument(
    "--runtime-binary",
    type=Path,
    help="Use an existing amentia-runtime-bin instead of building one.",
  )
  parser.add_argument(
    "--llama-binary",
    type=Path,
    help="Use an existing llama.cpp llama-cli backend for packaged local inference.",
  )
  parser.add_argument(
    "--app-binary",
    type=Path,
    help="Use an existing AmentiaApp executable instead of the default Swift build output.",
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
    default=os.environ.get("AMENTIA_RELEASE_VERSION", DEFAULT_VERSION),
    help="App bundle version, normally derived from the release tag without the leading v.",
  )
  parser.add_argument(
    "--source-commit",
    default=DEFAULT_SOURCE_COMMIT,
    help="Source commit recorded in package metadata.",
  )
  parser.add_argument(
    "--signing-mode",
    default="ad-hoc",
    choices=sorted(PACKAGE_SIGNING_MODES),
    help="Signing state recorded in AmentiaPackage.json.",
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
  package_root = repo_root / "apps" / "amentia-macos"
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
  if signing_mode not in PACKAGE_SIGNING_MODES:
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
    resources_path / "AmentiaPackage.json",
    arch,
    version,
    source_commit,
    signing_mode,
  )
  (contents_path / "PkgInfo").write_text("APPL????\n", encoding="utf-8")

  create_app_icon(repo_root, resources_path / APP_ICON_FILE_NAME)
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


def create_app_icon(repo_root: Path, destination: Path) -> None:
  source = repo_root / APP_ICON_SOURCE_RELATIVE_PATH
  require_file(source, "macOS app icon source")
  assert_png_source_can_drive_app_icon(source)
  for tool in ("sips", "iconutil"):
    if shutil.which(tool) is None:
      raise FileNotFoundError(f"macOS app icon generation requires {tool}")

  destination.parent.mkdir(parents=True, exist_ok=True)
  with tempfile.TemporaryDirectory(prefix="amentia-iconset-") as temp_root:
    iconset_path = Path(temp_root) / "Amentia.iconset"
    iconset_path.mkdir()
    for point_size, scale in APP_ICONSET_RENDITIONS:
      pixel_size = point_size * scale
      suffix = "@2x" if scale == 2 else ""
      output_path = iconset_path / f"icon_{point_size}x{point_size}{suffix}.png"
      run(
        [
          "sips",
          "-z",
          str(pixel_size),
          str(pixel_size),
          str(source),
          "--out",
          str(output_path),
        ],
        repo_root,
      )
    run(["iconutil", "-c", "icns", str(iconset_path), "-o", str(destination)], repo_root)
  assert_macos_icon_packaged(destination)


def assert_png_source_can_drive_app_icon(path: Path) -> None:
  width, height = png_dimensions(path)
  if width < APP_ICON_MIN_SOURCE_SIZE or height < APP_ICON_MIN_SOURCE_SIZE:
    raise RuntimeError(
      "macOS app icon source must be at least "
      f"{APP_ICON_MIN_SOURCE_SIZE}x{APP_ICON_MIN_SOURCE_SIZE}: {path}"
    )
  assert_png_source_has_transparent_corners(path)
  assert_png_source_fills_macos_icon_frame(path)


def png_dimensions(path: Path) -> tuple[int, int]:
  data = path.read_bytes()
  if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n":
    raise RuntimeError(f"macOS app icon source must be a PNG file: {path}")
  return struct.unpack(">II", data[16:24])


def assert_png_source_has_transparent_corners(path: Path) -> None:
  image = decode_rgba_png(path)
  width, height = image["width"], image["height"]
  pixels = image["pixels"]
  corners = [
    pixels[0][0],
    pixels[0][width - 1],
    pixels[height - 1][0],
    pixels[height - 1][width - 1],
  ]
  if any(pixel[3] != 0 for pixel in corners):
    raise RuntimeError(
      "macOS app icon source must use transparent outer corners so the "
      f"installed Dock icon is a rounded app tile: {path}"
    )


def assert_png_source_fills_macos_icon_frame(path: Path) -> None:
  image = decode_rgba_png(path)
  width, height = image["width"], image["height"]
  pixels = image["pixels"]
  alpha_bounds = png_alpha_bounds(pixels, width, height)
  if alpha_bounds is None:
    raise RuntimeError(f"macOS app icon source must contain visible pixels: {path}")

  left, top, right, bottom = alpha_bounds
  max_margin = round(min(width, height) * APP_ICON_MAX_TRANSPARENT_MARGIN_RATIO)
  margins = (left, top, width - right, height - bottom)
  if any(margin > max_margin for margin in margins):
    raise RuntimeError(
      "macOS app icon source must fill the icon frame instead of placing a "
      f"small tile inside transparent padding: {path}: margins {margins}, "
      f"max {max_margin}"
    )


def png_alpha_bounds(
  pixels: list[list[tuple[int, int, int, int]]],
  width: int,
  height: int,
) -> tuple[int, int, int, int] | None:
  left = width
  top = height
  right = 0
  bottom = 0
  for y, row in enumerate(pixels):
    for x, pixel in enumerate(row):
      if pixel[3] == 0:
        continue
      left = min(left, x)
      top = min(top, y)
      right = max(right, x + 1)
      bottom = max(bottom, y + 1)

  if right == 0 or bottom == 0:
    return None
  return left, top, right, bottom


def decode_rgba_png(path: Path) -> dict[str, object]:
  data = path.read_bytes()
  if len(data) < 33 or data[:8] != b"\x89PNG\r\n\x1a\n":
    raise RuntimeError(f"macOS app icon source must be a PNG file: {path}")

  chunks = png_chunks(data, path)
  ihdr = chunks.get("IHDR", [None])[0]
  if ihdr is None:
    raise RuntimeError(f"PNG icon source is missing IHDR: {path}")
  width, height = struct.unpack(">II", ihdr[:8])
  bit_depth = ihdr[8]
  color_type = ihdr[9]
  if bit_depth != 8 or color_type != 6:
    raise RuntimeError(
      "macOS app icon source must be an 8-bit RGBA PNG with alpha transparency: "
      f"{path}"
    )

  idat = b"".join(chunks.get("IDAT", []))
  if not idat:
    raise RuntimeError(f"PNG icon source is missing image data: {path}")
  raw = zlib.decompress(idat)
  return {
    "width": width,
    "height": height,
    "pixels": unfilter_rgba_png(raw, width, height, path),
  }


def png_chunks(data: bytes, path: Path) -> dict[str, list[bytes]]:
  chunks: dict[str, list[bytes]] = {}
  position = 8
  while position + 8 <= len(data):
    length = struct.unpack(">I", data[position:position + 4])[0]
    chunk_type = data[position + 4:position + 8].decode("ascii", errors="replace")
    chunk_start = position + 8
    chunk_end = chunk_start + length
    if chunk_end + 4 > len(data):
      raise RuntimeError(f"PNG chunk exceeds file length: {path}: {chunk_type}")
    chunks.setdefault(chunk_type, []).append(data[chunk_start:chunk_end])
    position = chunk_end + 4
    if chunk_type == "IEND":
      break
  return chunks


def unfilter_rgba_png(
  raw: bytes,
  width: int,
  height: int,
  path: Path,
) -> list[list[tuple[int, int, int, int]]]:
  bytes_per_pixel = 4
  row_length = width * bytes_per_pixel
  expected = height * (1 + row_length)
  if len(raw) != expected:
    raise RuntimeError(
      f"PNG icon source has unexpected image data length: {path}: {len(raw)} != {expected}"
    )

  rows: list[bytearray] = []
  offset = 0
  for _row_index in range(height):
    filter_type = raw[offset]
    offset += 1
    row = bytearray(raw[offset:offset + row_length])
    offset += row_length
    previous = rows[-1] if rows else bytearray(row_length)
    unfilter_png_row(row, previous, filter_type, bytes_per_pixel, path)
    rows.append(row)

  return [
    [
      tuple(row[column:column + 4])  # type: ignore[misc]
      for column in range(0, row_length, bytes_per_pixel)
    ]
    for row in rows
  ]


def unfilter_png_row(
  row: bytearray,
  previous: bytearray,
  filter_type: int,
  bytes_per_pixel: int,
  path: Path,
) -> None:
  for index, value in enumerate(row):
    left = row[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
    up = previous[index]
    upper_left = previous[index - bytes_per_pixel] if index >= bytes_per_pixel else 0
    if filter_type == 0:
      continue
    if filter_type == 1:
      row[index] = (value + left) & 0xFF
    elif filter_type == 2:
      row[index] = (value + up) & 0xFF
    elif filter_type == 3:
      row[index] = (value + ((left + up) // 2)) & 0xFF
    elif filter_type == 4:
      row[index] = (value + paeth_predictor(left, up, upper_left)) & 0xFF
    else:
      raise RuntimeError(f"PNG icon source uses unsupported row filter {filter_type}: {path}")


def paeth_predictor(left: int, up: int, upper_left: int) -> int:
  estimate = left + up - upper_left
  distance_left = abs(estimate - left)
  distance_up = abs(estimate - up)
  distance_upper_left = abs(estimate - upper_left)
  if distance_left <= distance_up and distance_left <= distance_upper_left:
    return left
  if distance_up <= distance_upper_left:
    return up
  return upper_left


def assert_macos_icon_packaged(path: Path) -> None:
  require_file(path, "macOS app icon")
  data = path.read_bytes()
  if len(data) < 8 or data[:4] != b"icns":
    raise RuntimeError(f"macOS app icon must be an ICNS file: {path}")
  declared_size = struct.unpack(">I", data[4:8])[0]
  actual_size = path.stat().st_size
  if declared_size != actual_size:
    raise RuntimeError(
      f"macOS app icon size header must match file size: {path}: "
      f"{declared_size} != {actual_size}"
    )


def validate_swift_package_rules(repo_root: Path) -> None:
  package_path = repo_root / "apps" / "amentia-macos" / "Package.swift"
  text = package_path.read_text(encoding="utf-8")
  required_fragments = {
    "minimum macOS 12 platform": ".macOS(.v12)",
    "AmentiaApp executable product": 'name: "AmentiaApp"',
    "AmentiaApp source target": 'path: "Sources/AmentiaApp"',
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
    "minimumSystemVersion": MINIMUM_SYSTEM_VERSION,
    "architecture": arch,
    "runtimeExecutable": RUNTIME_EXECUTABLE_NAME,
    "backendExecutable": (
      LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME
    ).as_posix(),
    "defaultModelId": DEFAULT_MODEL_ID,
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
    "modelDelivery": MODEL_DELIVERY_MODE,
    "modelWeightsBundled": MODEL_WEIGHTS_BUNDLED,
    "modelMetadataBundled": MODEL_METADATA_BUNDLED,
    "amentiaAccountRequired": AMENTIA_ACCOUNT_REQUIRED,
    "defaultLocalExecutionSafetyMode": DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
    "localExecutionSafetyModes": list(LOCAL_EXECUTION_SAFETY_MODES),
    "dailyDriverStageSource": DAILY_DRIVER_STAGE_SOURCE,
    "dailyDriverNextActionSource": DAILY_DRIVER_NEXT_ACTION_SOURCE,
    "dailyDriverPresentation": DAILY_DRIVER_PRESENTATION,
    "firstAppOpenActionContract": FIRST_APP_OPEN_ACTION_CONTRACT,
    "bundledPluginsIncluded": True,
    "sandboxMode": SANDBOX_CONTRACT["mode"],
    "sandboxBackend": SANDBOX_CONTRACT["backend"],
    "sandboxFallback": SANDBOX_CONTRACT["fallback"],
    "sandboxNetworkDefault": SANDBOX_CONTRACT["networkDefault"],
    "signing": signing_mode,
    "distributionTrust": package_distribution_trust(signing_mode),
    "sizeBudget": package_size_budget(),
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
  ignore = shutil.ignore_patterns(
    "*.gguf",
    "*.bin",
    "*.safetensors",
    "*.pyc",
    "__pycache__",
    ".DS_Store",
  )
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
    app_path / "Contents" / "Resources" / "AmentiaPackage.json",
    app_path / "Contents" / "PkgInfo",
    app_path / "Contents" / "MacOS" / APP_EXECUTABLE_NAME,
    app_path / "Contents" / "MacOS" / RUNTIME_EXECUTABLE_NAME,
    app_path / "Contents" / "Resources" / APP_ICON_FILE_NAME,
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
  assert_macos_icon_packaged(app_path / "Contents" / "Resources" / APP_ICON_FILE_NAME)
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
  assert_llama_backend_launches(app_path)
  assert_llama_backend_dependencies_match_arch(app_path)
  assert_packaged_app_copy_is_present(app_path)
  assert_no_model_weights_are_bundled(app_path)
  assert_packaged_model_manifest_is_downloadable(app_path)
  assert_bundled_plugins_are_package_ready(app_path)
  assert_app_bundle_size_budget(app_path)


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


def assert_llama_backend_launches(app_path: Path) -> None:
  backend = (
    app_path
    / "Contents"
    / "Resources"
    / LLAMA_BACKEND_RELATIVE_PARENT
    / LLAMA_BACKEND_EXECUTABLE_NAME
  )
  completed = subprocess.run(
    [str(backend), "--help"],
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
    timeout=10,
  )
  if completed.returncode != 0:
    raise RuntimeError(
      "Packaged llama.cpp backend failed to launch. "
      f"Exit {completed.returncode}. Output: {completed.stdout[-1000:]}"
    )


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
  manifest_path = app_path / "Contents" / "Resources" / "AmentiaPackage.json"
  manifest = read_json_object(manifest_path)
  validate_package_manifest_contract(
    manifest,
    f"Package manifest: {manifest_path}",
    source_commit=expected_source_commit,
    signing_mode=expected_signing_mode,
    bundle_version=expected_version,
    expected_size_budget=package_size_budget(),
  )
  if expected_arch != SUPPORTED_ARCH:
    raise RuntimeError(f"Packaged app architecture must be {SUPPORTED_ARCH}")
  expected_values = {
    "bundleIdentifier": DEFAULT_BUNDLE_ID,
    "runtimeExecutable": RUNTIME_EXECUTABLE_NAME,
    "backendExecutable": (
      LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME
    ).as_posix(),
    "defaultModelManifest": DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix(),
  }
  for field, expected_value in expected_values.items():
    if manifest.get(field) != expected_value:
      raise RuntimeError(
        f"Package manifest field {field} must be {expected_value!r}: {manifest_path}"
      )

  if required_bool_field(manifest, "modelWeightsBundled", manifest_path) is not MODEL_WEIGHTS_BUNDLED:
    raise RuntimeError("Package manifest must not claim bundled model weights")
  if required_bool_field(manifest, "modelMetadataBundled", manifest_path) is not MODEL_METADATA_BUNDLED:
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
    manifest_path = plugin_root / "amentia-plugin.json"
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
    assert_bundled_plugin_skill_files(plugin_root, manifest, capabilities)
    assert_bundled_plugin_mcp_server_files(plugin_root, manifest, capabilities)


def assert_app_bundle_size_budget(app_path: Path) -> None:
  budget = package_size_budget()
  assert_size_under_budget(
    directory_size_bytes(app_path),
    budget["maxAppBundleBytes"],
    "macOS app bundle",
  )


def assert_zip_size_budget(zip_path: Path) -> None:
  budget = package_size_budget()
  assert_size_under_budget(
    zip_path.stat().st_size,
    budget["maxZipArtifactBytes"],
    "macOS zip artifact",
  )


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
  workflow_connectors = {}
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
    workflow_connectors[workflow_id] = connector_id
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
    if isinstance(workflow_id, str) and workflow_id.strip():
      input_fields = assert_connector_workflow_envelope(
        plugin_root,
        command_id,
        execution,
        "input",
      )
      output_fields = assert_connector_workflow_envelope(
        plugin_root,
        command_id,
        execution,
        "output",
      )
      assert_connector_workflow_field(
        plugin_root,
        command_id,
        "output",
        output_fields,
        "items",
        "timelineItems",
      )
      assert_connector_workflow_field(
        plugin_root,
        command_id,
        "output",
        output_fields,
        "memoryNotes",
        "memoryNotes",
      )
      if command_id.endswith(".publish-page-draft"):
        assert_connector_workflow_field(
          plugin_root,
          command_id,
          "input",
          input_fields,
          "input",
          "text",
        )
        assert_connector_workflow_field(
          plugin_root,
          command_id,
          "input",
          input_fields,
          "connectors",
          "connectorRefs",
        )
    if isinstance(workflow_id, str) and workflow_id.strip() and workflow_id not in workflow_ids:
      raise RuntimeError(
        f"Bundled command {command_id} references undeclared workflow: {workflow_id}"
      )
    connectors = execution.get("connectors", [])
    if (
      isinstance(workflow_id, str)
      and workflow_id.strip()
      and isinstance(connectors, list)
      and workflow_connectors.get(workflow_id) not in connectors
    ):
      raise RuntimeError(
        f"Bundled command {command_id} workflow is not bound to its connector: {workflow_id}"
      )


def assert_connector_workflow_envelope(
  plugin_root: Path,
  command_id: str,
  execution: dict,
  envelope_key: str,
) -> dict[str, str]:
  envelope = execution.get(envelope_key)
  if not isinstance(envelope, dict):
    raise RuntimeError(
      f"Bundled command {command_id} must declare execution.{envelope_key}: {plugin_root}"
    )
  envelope_name = envelope.get("envelope")
  if not isinstance(envelope_name, str) or not envelope_name.strip():
    raise RuntimeError(
      f"Bundled command {command_id} execution.{envelope_key}.envelope is required"
    )
  fields = envelope.get("fields")
  if not isinstance(fields, list) or not fields:
    raise RuntimeError(
      f"Bundled command {command_id} execution.{envelope_key}.fields must be non-empty"
    )
  field_kinds = {}
  for field in fields:
    if not isinstance(field, dict):
      raise RuntimeError(
        f"Bundled command {command_id} execution.{envelope_key}.fields must be objects"
      )
    name = field.get("name")
    kind = field.get("kind")
    if isinstance(name, str) and isinstance(kind, str):
      field_kinds[name] = kind
  return field_kinds


def assert_connector_workflow_field(
  plugin_root: Path,
  command_id: str,
  envelope_key: str,
  fields: dict[str, str],
  name: str,
  kind: str,
) -> None:
  if fields.get(name) != kind:
    raise RuntimeError(
      f"Bundled command {command_id} execution.{envelope_key} must declare "
      f"{name}:{kind}: {plugin_root}"
    )


def assert_safe_capability_identifier(identifier: str, capability: str) -> None:
  if (
    not identifier.strip()
    or identifier in {".", ".."}
    or any(character in identifier for character in "/\\:")
  ):
    raise RuntimeError(f"Bundled plugin capability has unsafe identifier: {capability}")


def assert_bundled_plugin_skill_files(
  plugin_root: Path,
  manifest: dict,
  capabilities: set[str],
) -> None:
  skills = manifest.get("skills", [])
  if not isinstance(skills, list):
    raise RuntimeError(f"Bundled plugin skills must be a list: {plugin_root}")
  declared_skill_ids = set()
  for skill in skills:
    if not isinstance(skill, dict):
      raise RuntimeError(f"Bundled plugin skill entry must be an object: {plugin_root}")
    skill_id = skill.get("id")
    if not isinstance(skill_id, str) or not skill_id.strip():
      raise RuntimeError(f"Bundled plugin skill entry is missing id: {plugin_root}")
    assert_safe_capability_identifier(skill_id, f"skill:{skill_id}")
    declared_skill_ids.add(skill_id)
    if f"skill:{skill_id}" not in capabilities:
      raise RuntimeError(f"Bundled plugin skill is missing capability: skill:{skill_id}")
    skill_path = skill.get("path")
    if not isinstance(skill_path, str) or not skill_path.strip():
      raise RuntimeError(f"Bundled plugin skill entry is missing path: {plugin_root}")
    require_file(
      safe_plugin_resource_path(plugin_root, skill_path),
      f"bundled plugin skill {skill_path}",
    )
  for capability in capabilities:
    if not capability.startswith("skill:"):
      continue
    skill_id = capability.removeprefix("skill:")
    if skill_id not in declared_skill_ids:
      raise RuntimeError(f"Bundled plugin is missing skill declaration: {capability}")


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
  assert_zip_size_budget(zip_path)
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
    f"{APP_NAME}.app/Contents/Resources/{DEFAULT_MODEL_MANIFEST_RELATIVE_PATH.as_posix()}"
  )
  entries.add(
    f"{APP_NAME}.app/Contents/Resources/"
    f"{(LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME).as_posix()}"
  )
  for plugin_id in REQUIRED_BUNDLED_PLUGIN_CAPABILITIES:
    entries.add(
      f"{APP_NAME}.app/Contents/Resources/plugins/bundled/{plugin_id}/amentia-plugin.json"
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
      app_binary = args.app_binary or repo_root / "apps" / "amentia-macos" / ".build" / args.configuration / SWIFT_EXECUTABLE_NAME
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
