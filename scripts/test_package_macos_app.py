#!/usr/bin/env python3
"""Unit checks for macOS packaging helpers that do not require macOS."""

from __future__ import annotations

import stat
import tempfile
import zipfile
import json
from pathlib import Path

from macos_llama_backend import (
  LLAMA_BACKEND_EXECUTABLE_NAME,
  LLAMA_BACKEND_LIB_DIRECTORY_NAME,
  is_packaged_backend_dependency,
  parse_otool_dependencies,
)
from package_macos_app import (
  DAILY_DRIVER_NEXT_ACTION_SOURCE,
  FIRST_APP_OPEN_ACTION_CONTRACT,
  DAILY_DRIVER_PRESENTATION,
  DAILY_DRIVER_STAGE_SOURCE,
  LLAMA_BACKEND_RELATIVE_PARENT,
  assert_macos_icon_packaged,
  assert_png_source_can_drive_app_icon,
  assert_bundled_plugin_connector_workflows,
  assert_packaged_app_copy_is_present,
  assert_zip_entries_are_safe,
  copy_tree_if_present,
  copy_required_llama_backend,
  normalize_source_commit,
  normalize_version,
  parse_lipo_architectures,
  png_dimensions,
  write_package_manifest,
)
from package_contract import (
  DEFAULT_MAX_APP_BUNDLE_BYTES,
  DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
  DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
  LOCAL_EXECUTION_SAFETY_MODES,
  PACKAGE_MANIFEST_SCHEMA_VERSION,
  PITH_ACCOUNT_REQUIRED,
  SANDBOX_CONTRACT,
  SUPPORTED_ARCH,
  assert_size_under_budget,
  package_size_budget,
)


def assert_equal(actual: object, expected: object) -> None:
  if actual != expected:
    raise AssertionError(f"expected {expected!r}, got {actual!r}")


def assert_raises(action, message: str) -> None:
  try:
    action()
  except (RuntimeError, FileNotFoundError):
    return
  raise AssertionError(message)


def write_png_header(path: Path, width: int, height: int) -> None:
  path.write_bytes(
    b"\x89PNG\r\n\x1a\n"
    + b"\x00\x00\x00\rIHDR"
    + width.to_bytes(4, "big")
    + height.to_bytes(4, "big")
    + b"\x08\x06\x00\x00\x00"
  )


def write_icns_header(path: Path, declared_size: int, body: bytes = b"") -> None:
  path.write_bytes(b"icns" + declared_size.to_bytes(4, "big") + body)


def main() -> int:
  assert_equal(normalize_version("0.1.0"), "0.1.0")
  assert_equal(normalize_version("v1.2.3"), "1.2.3")
  assert_raises(
    lambda: normalize_version("12.34"),
    "two-part versions should fail package metadata validation",
  )
  assert_raises(
    lambda: normalize_version("v1.2.3-beta"),
    "prerelease suffixes should stay out of Info.plist versions",
  )
  assert_equal(normalize_source_commit("development"), "development")
  assert_equal(
    normalize_source_commit("ABCDEF0123456789ABCDEF0123456789ABCDEF01"),
    "abcdef0123456789abcdef0123456789abcdef01",
  )
  assert_raises(
    lambda: normalize_source_commit("short"),
    "short source commits should fail package metadata validation",
  )
  with tempfile.TemporaryDirectory(prefix="pith-package-icon-") as root:
    root_path = Path(root)
    png_path = root_path / "icon.png"
    write_png_header(png_path, 1254, 1254)
    assert_equal(png_dimensions(png_path), (1254, 1254))
    assert_png_source_can_drive_app_icon(png_path)
    small_png_path = root_path / "small.png"
    write_png_header(small_png_path, 512, 512)
    assert_raises(
      lambda: assert_png_source_can_drive_app_icon(small_png_path),
      "small icon source should fail macOS icon generation guard",
    )
    invalid_png_path = root_path / "icon.txt"
    invalid_png_path.write_text("not png", encoding="utf-8")
    assert_raises(
      lambda: png_dimensions(invalid_png_path),
      "non-PNG icon source should fail",
    )
    icns_path = root_path / "Pith.icns"
    write_icns_header(icns_path, 12, b"abcd")
    assert_macos_icon_packaged(icns_path)
    broken_icns_path = root_path / "Broken.icns"
    write_icns_header(broken_icns_path, 8, b"abcd")
    assert_raises(
      lambda: assert_macos_icon_packaged(broken_icns_path),
      "invalid ICNS size header should fail",
    )
  with tempfile.TemporaryDirectory(prefix="pith-package-manifest-") as root:
    manifest_path = Path(root) / "PithPackage.json"
    write_package_manifest(
      manifest_path,
      SUPPORTED_ARCH,
      "1.2.3",
      "abcdef0123456789abcdef0123456789abcdef01",
      "ad-hoc",
    )
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert_equal(manifest["schemaVersion"], PACKAGE_MANIFEST_SCHEMA_VERSION)
    assert_equal(
      manifest["sourceCommit"],
      "abcdef0123456789abcdef0123456789abcdef01",
    )
    assert_equal(manifest["pithAccountRequired"], PITH_ACCOUNT_REQUIRED)
    assert_equal(
      manifest["defaultLocalExecutionSafetyMode"],
      DEFAULT_LOCAL_EXECUTION_SAFETY_MODE,
    )
    assert_equal(
      manifest["localExecutionSafetyModes"],
      list(LOCAL_EXECUTION_SAFETY_MODES),
    )
    assert_equal(manifest["distributionTrust"], "ad-hoc-not-notarized")
    assert_equal(manifest["sandboxMode"], SANDBOX_CONTRACT["mode"])
    assert_equal(manifest["sandboxBackend"], SANDBOX_CONTRACT["backend"])
    assert_equal(manifest["sandboxFallback"], SANDBOX_CONTRACT["fallback"])
    assert_equal(manifest["sandboxNetworkDefault"], SANDBOX_CONTRACT["networkDefault"])
    assert_equal(manifest["dailyDriverStageSource"], DAILY_DRIVER_STAGE_SOURCE)
    assert_equal(
      manifest["dailyDriverNextActionSource"],
      DAILY_DRIVER_NEXT_ACTION_SOURCE,
    )
    assert_equal(manifest["dailyDriverPresentation"], DAILY_DRIVER_PRESENTATION)
    assert_equal(
      manifest["firstAppOpenActionContract"],
      FIRST_APP_OPEN_ACTION_CONTRACT,
    )
    assert_equal(
      manifest["sizeBudget"],
      {
        "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES,
        "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
      },
    )

  assert_equal(
    package_size_budget(),
    {
      "maxAppBundleBytes": DEFAULT_MAX_APP_BUNDLE_BYTES,
      "maxZipArtifactBytes": DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
    },
  )
  assert_size_under_budget(1024, 2048, "test package")
  assert_raises(
    lambda: assert_size_under_budget(2049, 2048, "test package"),
    "oversized release packages should fail the size guard",
  )

  assert_equal(
    parse_lipo_architectures("Non-fat file: Pith is architecture: x86_64"),
    {"x86_64"},
  )
  assert_equal(
    parse_lipo_architectures(
      "Architectures in the fat file: Pith are: x86_64 arm64"
    ),
    {"x86_64", "arm64"},
  )
  try:
    parse_lipo_architectures("not a lipo architecture line")
  except RuntimeError:
    pass
  else:
    raise AssertionError("invalid lipo output should fail")
  assert_equal(
    parse_otool_dependencies(
      """/tmp/llama-cli:
\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)
\t@executable_path/lib/libllama.dylib (compatibility version 0.0.0, current version 0.0.0)
"""
    ),
    ["/usr/lib/libSystem.B.dylib", "@executable_path/lib/libllama.dylib"],
  )
  if not is_packaged_backend_dependency("@executable_path/lib/libllama.dylib", True):
    raise AssertionError("backend dependencies should allow packaged executable paths")
  if not is_packaged_backend_dependency("@loader_path/libggml.dylib", False):
    raise AssertionError("dylib dependencies should allow packaged loader paths")
  if is_packaged_backend_dependency("/external/package-manager/lib/libllama.dylib", True):
    raise AssertionError("absolute non-system dependency paths should be rejected")

  notion_manifest = {
    "appConnectors": [{"id": "notion", "service": "notion"}],
    "connectorWorkflows": [
      {
        "id": "notion.create-page",
        "connectorId": "notion",
        "action": "createPage",
        "stages": ["draftPrepared"],
        "statuses": ["prepared"],
      }
    ],
  }
  workflow_capabilities = {
    "command:notion.prepare-page-draft",
    "connector:notion",
    "connector_workflow:notion.create-page",
  }
  with tempfile.TemporaryDirectory(prefix="pith-package-plugin-workflow-") as root:
    plugin_root = Path(root)
    commands_dir = plugin_root / "commands"
    commands_dir.mkdir()
    (commands_dir / "notion.prepare-page-draft.json").write_text(
      json.dumps(
        {
          "execution": {
            "workflowId": "notion.create-page",
            "connectors": ["notion"],
          }
        }
      ),
      encoding="utf-8",
    )
    assert_bundled_plugin_connector_workflows(
      plugin_root,
      notion_manifest,
      workflow_capabilities,
    )
    (commands_dir / "notion.prepare-page-draft.json").write_text(
      json.dumps(
        {
          "execution": {
            "workflowId": "notion.create-page",
            "connectors": ["wrong-connector"],
          }
        }
      ),
      encoding="utf-8",
    )
    assert_raises(
      lambda: assert_bundled_plugin_connector_workflows(
        plugin_root,
        notion_manifest,
        workflow_capabilities,
      ),
      "command workflow must be bound to the declared connector",
    )

  with tempfile.TemporaryDirectory(prefix="pith-package-copy-") as root:
    root_path = Path(root)
    executable = root_path / "Pith.app" / "Contents" / "MacOS" / "Pith"
    executable.parent.mkdir(parents=True)
    executable.write_text(
      "\n".join(
        [
          "Start Pith to restore model choices",
          "paused downloads",
          "selected model choices remain local",
          "to keep resume data",
          "cancel to clear the partial file",
          "from saved resume data",
          "Refresh local model setup if readiness still fails",
          "Download Local Model",
          "Repair Model Setup",
          "Open Anyway",
          "Control-click Pith.app",
          "no Pith account required",
          "local execution mode",
          "package size budget",
        ]
      ),
      encoding="utf-8",
    )
    assert_packaged_app_copy_is_present(root_path / "Pith.app")
    executable.write_text("Start Pith to restore model choices\n", encoding="utf-8")
    assert_raises(
      lambda: assert_packaged_app_copy_is_present(root_path / "Pith.app"),
      "missing packaged recovery copy should fail package validation",
    )

  assert_zip_entries_are_safe(
    Path("Pith-macos-x86_64.zip"),
    [zipfile.ZipInfo("Pith.app/Contents/Resources/models/builtin/model-pack.json")],
  )
  assert_raises(
    lambda: assert_zip_entries_are_safe(
      Path("Pith-macos-x86_64.zip"),
      [zipfile.ZipInfo("Pith.app/Contents/Resources/models/builtin/local.gguf")],
    ),
    "zip model weights should be rejected",
  )
  assert_raises(
    lambda: assert_zip_entries_are_safe(
      Path("Pith-macos-x86_64.zip"),
      [zipfile.ZipInfo("../Pith.app/Contents/Info.plist")],
    ),
    "zip path traversal should be rejected",
  )
  symlink_entry = zipfile.ZipInfo("Pith.app/Contents/Resources/link")
  symlink_entry.external_attr = (stat.S_IFLNK | 0o777) << 16
  assert_raises(
    lambda: assert_zip_entries_are_safe(Path("Pith-macos-x86_64.zip"), [symlink_entry]),
    "zip symlinks should be rejected",
  )

  with tempfile.TemporaryDirectory(prefix="pith-package-resource-copy-") as root:
    root_path = Path(root)
    source = root_path / "source"
    destination = root_path / "destination"
    source.mkdir()
    (source / "pith-plugin.json").write_text("{}", encoding="utf-8")
    (source / "model.gguf").write_text("weight", encoding="utf-8")
    (source / "module.pyc").write_bytes(b"cache")
    pycache = source / "__pycache__"
    pycache.mkdir()
    (pycache / "module.cpython-311.pyc").write_bytes(b"cache")
    copy_tree_if_present(source, destination)
    if not (destination / "pith-plugin.json").is_file():
      raise AssertionError("resource copy should keep plugin metadata")
    for generated_path in (
      destination / "model.gguf",
      destination / "module.pyc",
      destination / "__pycache__",
    ):
      if generated_path.exists():
        raise AssertionError(f"resource copy should exclude {generated_path.name}")

  with tempfile.TemporaryDirectory(prefix="pith-package-test-") as root:
    root_path = Path(root)
    source_backend = root_path / "llama-cli"
    source_backend.write_text("#!/bin/sh\n", encoding="utf-8")
    source_backend.chmod(source_backend.stat().st_mode | stat.S_IXUSR)
    source_lib = root_path / LLAMA_BACKEND_LIB_DIRECTORY_NAME
    source_lib.mkdir()
    (source_lib / "libllama.dylib").write_text("placeholder", encoding="utf-8")
    packaged_backend = copy_required_llama_backend(
      root_path,
      root_path / "Resources",
      source_backend,
    )
    assert_equal(
      packaged_backend,
      root_path / "Resources" / LLAMA_BACKEND_RELATIVE_PARENT / LLAMA_BACKEND_EXECUTABLE_NAME,
    )
    if not packaged_backend.is_file():
      raise AssertionError("packaged llama backend should exist")
    if not (
      packaged_backend.parent / LLAMA_BACKEND_LIB_DIRECTORY_NAME / "libllama.dylib"
    ).is_file():
      raise AssertionError("packaged llama backend should include sibling dylib bundle")

  with tempfile.TemporaryDirectory(prefix="pith-package-missing-backend-") as root:
    try:
      copy_required_llama_backend(Path(root), Path(root) / "Resources", None)
    except FileNotFoundError:
      pass
    else:
      raise AssertionError("missing llama backend should fail packaging")
  print("package helper tests passed")
  return 0


if __name__ == "__main__":
  raise SystemExit(main())
