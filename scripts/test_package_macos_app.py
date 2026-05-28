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
  DAILY_DRIVER_PRESENTATION,
  DAILY_DRIVER_STAGE_SOURCE,
  DEFAULT_MAX_APP_BUNDLE_BYTES,
  DEFAULT_MAX_ZIP_ARTIFACT_BYTES,
  LLAMA_BACKEND_RELATIVE_PARENT,
  assert_size_under_budget,
  assert_bundled_plugin_connector_workflows,
  assert_packaged_app_copy_is_present,
  assert_zip_entries_are_safe,
  copy_required_llama_backend,
  package_size_budget,
  normalize_source_commit,
  normalize_version,
  parse_lipo_architectures,
  write_package_manifest,
)


def assert_equal(actual: object, expected: object) -> None:
  if actual != expected:
    raise AssertionError(f"expected {expected!r}, got {actual!r}")


def assert_raises(action, message: str) -> None:
  try:
    action()
  except RuntimeError:
    return
  raise AssertionError(message)


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
  with tempfile.TemporaryDirectory(prefix="pith-package-manifest-") as root:
    manifest_path = Path(root) / "PithPackage.json"
    write_package_manifest(
      manifest_path,
      "x86_64",
      "1.2.3",
      "abcdef0123456789abcdef0123456789abcdef01",
      "ad-hoc",
    )
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    assert_equal(manifest["schemaVersion"], 1)
    assert_equal(
      manifest["sourceCommit"],
      "abcdef0123456789abcdef0123456789abcdef01",
    )
    assert_equal(manifest["sandboxMode"], "workspaceReadWrite")
    assert_equal(manifest["sandboxBackend"], "runtime-detected")
    assert_equal(manifest["sandboxFallback"], "processOnlyWhenNativeUnavailable")
    assert_equal(manifest["sandboxNetworkDefault"], "disabled")
    assert_equal(manifest["dailyDriverStageSource"], DAILY_DRIVER_STAGE_SOURCE)
    assert_equal(
      manifest["dailyDriverNextActionSource"],
      DAILY_DRIVER_NEXT_ACTION_SOURCE,
    )
    assert_equal(manifest["dailyDriverPresentation"], DAILY_DRIVER_PRESENTATION)
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
          "Recovery: launch the runtime",
          "paused downloads",
          "selected model state are read from local storage",
          "to keep resume data",
          "cancel to clear the partial file",
          "from saved resume data",
          "reinstall metadata if readiness still fails",
          "Open Anyway",
          "Control-click Pith.app",
          "package size budget",
        ]
      ),
      encoding="utf-8",
    )
    assert_packaged_app_copy_is_present(root_path / "Pith.app")
    executable.write_text("Recovery: launch the runtime\n", encoding="utf-8")
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
