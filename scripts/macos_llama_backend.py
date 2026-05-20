#!/usr/bin/env python3
"""Stage a portable llama.cpp backend for the packaged macOS app."""

from __future__ import annotations

import shutil
import stat
import subprocess
from pathlib import Path


LLAMA_BACKEND_EXECUTABLE_NAME = "llama-cli"
LLAMA_BACKEND_LIB_DIRECTORY_NAME = "lib"
SYSTEM_DYLIB_PREFIXES = ("/usr/lib/", "/System/Library/")
FORBIDDEN_DYLIB_PREFIXES = ("/usr/local/", "/opt/homebrew/", "/opt/local/")
FORBIDDEN_DYLIB_MARKERS = ("/Cellar/", "/Homebrew/")


def stage_llama_backend(source_backend: Path, target_directory: Path) -> Path:
  require_file(source_backend, "llama.cpp backend")
  if source_backend.is_symlink():
    raise RuntimeError(f"Packaged llama.cpp backend must not be a symlink: {source_backend}")

  reset_directory(target_directory)
  target_directory.mkdir(parents=True, exist_ok=True)
  packaged_backend = target_directory / LLAMA_BACKEND_EXECUTABLE_NAME
  copy_executable(source_backend, packaged_backend)
  copy_existing_llama_lib_bundle(source_backend, target_directory)

  if shutil.which("otool") is None:
    return packaged_backend

  dependency_sources = collect_macho_dependency_sources(source_backend)
  copy_llama_dependency_sources(dependency_sources, target_directory)
  rewrite_llama_dependency_paths(source_backend, packaged_backend, dependency_sources)
  assert_portable_llama_backend(target_directory)
  return packaged_backend


def assert_portable_llama_backend(target_directory: Path) -> None:
  if shutil.which("otool") is None:
    return
  backend = target_directory / LLAMA_BACKEND_EXECUTABLE_NAME
  assert_portable_macho_dependencies(backend, target_directory, is_backend=True)
  lib_directory = target_directory / LLAMA_BACKEND_LIB_DIRECTORY_NAME
  if not lib_directory.is_dir():
    return
  for dylib in sorted(lib_directory.glob("*.dylib")):
    assert_portable_macho_dependencies(dylib, target_directory, is_backend=False)


def parse_otool_dependencies(output: str) -> list[str]:
  dependencies: list[str] = []
  for line in output.splitlines()[1:]:
    stripped = line.strip()
    if not stripped:
      continue
    dependencies.append(stripped.split(" (", 1)[0])
  return dependencies


def is_forbidden_runtime_dependency(dependency: str) -> bool:
  return dependency.startswith(FORBIDDEN_DYLIB_PREFIXES) or any(
    marker in dependency for marker in FORBIDDEN_DYLIB_MARKERS
  )


def require_file(path: Path, label: str) -> None:
  if not path.is_file():
    raise FileNotFoundError(f"Missing {label}: {path}")


def reset_directory(path: Path) -> None:
  if path.exists():
    shutil.rmtree(path)


def copy_executable(source: Path, destination: Path) -> None:
  require_file(source, "executable")
  shutil.copy2(source, destination)
  current_mode = destination.stat().st_mode
  destination.chmod(current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def assert_copy_source_has_no_symlinks(source: Path) -> None:
  if source.is_symlink():
    raise RuntimeError(f"Packaged resource source must not be a symlink: {source}")
  symlink = next((path for path in source.rglob("*") if path.is_symlink()), None)
  if symlink is not None:
    raise RuntimeError(f"Packaged resources must not contain symlinks: {symlink}")


def copy_existing_llama_lib_bundle(source_backend: Path, target_directory: Path) -> None:
  source_lib_directory = source_backend.parent / LLAMA_BACKEND_LIB_DIRECTORY_NAME
  if not source_lib_directory.is_dir():
    return
  assert_copy_source_has_no_symlinks(source_lib_directory)
  target_lib_directory = target_directory / LLAMA_BACKEND_LIB_DIRECTORY_NAME
  shutil.copytree(source_lib_directory, target_lib_directory, dirs_exist_ok=True)


def collect_macho_dependency_sources(source_backend: Path) -> dict[str, Path]:
  dependency_sources: dict[str, Path] = {}
  queue = [source_backend]
  visited: set[Path] = set()
  while queue:
    binary = queue.pop(0).resolve()
    if binary in visited:
      continue
    visited.add(binary)
    for dependency in macho_runtime_dependencies(binary):
      if is_system_dylib(dependency):
        continue
      dependency_path = resolve_macho_dependency(binary, source_backend, dependency)
      if dependency_path is None:
        raise RuntimeError(
          f"Could not resolve llama.cpp runtime dependency {dependency!r} from {binary}"
        )
      if dependency_path.is_symlink():
        dependency_path = dependency_path.resolve()
      require_file(dependency_path, f"llama.cpp dependency {dependency}")
      name = dependency_path.name
      previous = dependency_sources.get(name)
      if previous is not None and previous.resolve() != dependency_path.resolve():
        raise RuntimeError(
          f"Conflicting llama.cpp dependency names: {previous} and {dependency_path}"
        )
      if previous is None:
        dependency_sources[name] = dependency_path
        queue.append(dependency_path)
  return dependency_sources


def macho_runtime_dependencies(binary: Path) -> list[str]:
  dependencies = parse_otool_dependencies(capture(["otool", "-L", str(binary)], binary.parent))
  if binary.suffix == ".dylib" and dependencies:
    return dependencies[1:]
  return dependencies


def is_system_dylib(dependency: str) -> bool:
  return dependency.startswith(SYSTEM_DYLIB_PREFIXES)


def resolve_macho_dependency(binary: Path, main_executable: Path, dependency: str) -> Path | None:
  if dependency.startswith("@loader_path/"):
    return (binary.parent / dependency.removeprefix("@loader_path/")).resolve()
  if dependency.startswith("@executable_path/"):
    return (main_executable.parent / dependency.removeprefix("@executable_path/")).resolve()
  if dependency.startswith("@rpath/"):
    suffix = dependency.removeprefix("@rpath/")
    for rpath in macho_rpaths(binary, main_executable):
      candidate = (rpath / suffix).resolve()
      if candidate.is_file():
        return candidate
    return None
  if dependency.startswith("@"):
    return None
  return Path(dependency).resolve()


def macho_rpaths(binary: Path, main_executable: Path) -> list[Path]:
  output = capture(["otool", "-l", str(binary)], binary.parent)
  rpaths: list[Path] = [binary.parent, binary.parent / LLAMA_BACKEND_LIB_DIRECTORY_NAME]
  for line in output.splitlines():
    stripped = line.strip()
    if not stripped.startswith("path "):
      continue
    raw_path = stripped.removeprefix("path ").split(" (offset", 1)[0]
    resolved = resolve_macho_rpath(binary, main_executable, raw_path)
    if resolved not in rpaths:
      rpaths.append(resolved)
  return rpaths


def resolve_macho_rpath(binary: Path, main_executable: Path, rpath: str) -> Path:
  if rpath.startswith("@loader_path/"):
    return (binary.parent / rpath.removeprefix("@loader_path/")).resolve()
  if rpath.startswith("@executable_path/"):
    return (main_executable.parent / rpath.removeprefix("@executable_path/")).resolve()
  return Path(rpath).resolve()


def copy_llama_dependency_sources(
  dependency_sources: dict[str, Path],
  target_directory: Path,
) -> None:
  if not dependency_sources:
    return
  lib_directory = target_directory / LLAMA_BACKEND_LIB_DIRECTORY_NAME
  lib_directory.mkdir(parents=True, exist_ok=True)
  for name, source in sorted(dependency_sources.items()):
    target = lib_directory / name
    shutil.copy2(source, target)
    target.chmod(target.stat().st_mode | stat.S_IRUSR | stat.S_IXUSR)


def rewrite_llama_dependency_paths(
  source_backend: Path,
  packaged_backend: Path,
  dependency_sources: dict[str, Path],
) -> None:
  if not dependency_sources:
    return
  if shutil.which("install_name_tool") is None:
    raise RuntimeError("install_name_tool is required to package llama.cpp dylibs")

  rewrite_dependency_references(
    source_backend,
    packaged_backend,
    source_backend,
    dependency_sources,
    "@executable_path/lib",
  )
  lib_directory = packaged_backend.parent / LLAMA_BACKEND_LIB_DIRECTORY_NAME
  for name, source in sorted(dependency_sources.items()):
    target = lib_directory / name
    run(["install_name_tool", "-id", f"@loader_path/{name}", str(target)], target.parent)
    rewrite_dependency_references(
      source,
      target,
      source_backend,
      dependency_sources,
      "@loader_path",
    )


def rewrite_dependency_references(
  source_binary: Path,
  target_binary: Path,
  main_executable: Path,
  dependency_sources: dict[str, Path],
  target_prefix: str,
) -> None:
  for dependency in macho_runtime_dependencies(source_binary):
    dependency_path = resolve_macho_dependency(source_binary, main_executable, dependency)
    if dependency_path is None:
      continue
    if dependency_path.name not in dependency_sources:
      continue
    run(
      [
        "install_name_tool",
        "-change",
        dependency,
        f"{target_prefix}/{dependency_path.name}",
        str(target_binary),
      ],
      target_binary.parent,
    )


def assert_portable_macho_dependencies(
  binary: Path,
  backend_directory: Path,
  is_backend: bool,
) -> None:
  dependencies = parse_otool_dependencies(capture(["otool", "-L", str(binary)], binary.parent))
  for dependency in dependencies:
    if is_system_dylib(dependency):
      continue
    if is_forbidden_runtime_dependency(dependency):
      raise RuntimeError(f"Packaged llama.cpp dependency is not portable: {binary}: {dependency}")
    if is_backend and dependency.startswith("@executable_path/lib/"):
      target = backend_directory / LLAMA_BACKEND_LIB_DIRECTORY_NAME / Path(dependency).name
      require_file(target, f"packaged llama.cpp dependency {dependency}")
      continue
    if not is_backend and dependency.startswith("@loader_path/"):
      target = binary.parent / dependency.removeprefix("@loader_path/")
      require_file(target, f"packaged llama.cpp dependency {dependency}")
      continue
    raise RuntimeError(f"Packaged llama.cpp dependency is not portable: {binary}: {dependency}")


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


def capture(command: list[str], cwd: Path) -> str:
  completed = subprocess.run(
    command,
    cwd=cwd,
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.STDOUT,
  )
  if completed.returncode != 0:
    output = completed.stdout.strip()
    detail = f": {output}" if output else ""
    raise RuntimeError(
      f"command failed with status {completed.returncode}: {' '.join(command)}{detail}"
    )
  return completed.stdout.strip()
