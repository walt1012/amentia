use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub(crate) fn resolve_workspace_path(
  workspace_root: &Path,
  relative_path: &str,
  allow_directory: bool,
) -> Result<PathBuf> {
  let workspace_root = canonical_workspace_root(workspace_root)?;
  let candidate = workspace_root.join(relative_path);
  let resolved = fs::canonicalize(&candidate)
    .with_context(|| format!("failed to resolve workspace path {}", candidate.display()))?;

  if !resolved.starts_with(&workspace_root) {
    bail!("workspace path escapes the selected workspace");
  }

  let metadata = fs::metadata(&resolved)
    .with_context(|| format!("failed to read metadata for {}", resolved.display()))?;

  if metadata.is_dir() && !allow_directory {
    bail!("workspace path points to a directory");
  }

  Ok(resolved)
}

pub(crate) fn relative_path_string(workspace_root: &Path, target: &Path) -> Result<String> {
  let relative = target
    .strip_prefix(workspace_root)
    .with_context(|| format!("failed to relativize {}", target.display()))?;

  if relative.as_os_str().is_empty() {
    return Ok(".".to_string());
  }

  Ok(relative.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn canonical_workspace_root(workspace_root: &Path) -> Result<PathBuf> {
  fs::canonicalize(workspace_root).with_context(|| {
    format!(
      "failed to resolve workspace root {}",
      workspace_root.display()
    )
  })
}

pub(crate) fn validate_workspace_write_target(workspace_root: &Path, target: &Path) -> Result<()> {
  match fs::symlink_metadata(target) {
    Ok(metadata) => {
      if metadata.file_type().is_symlink() {
        bail!("workspace path points to a symlink");
      }
      validate_existing_workspace_path(workspace_root, target)?;
      validate_no_symlink_components(workspace_root, target)?;
    }
    Err(error) if error.kind() == ErrorKind::NotFound => {
      if let Some(parent) = target.parent() {
        validate_workspace_write_parent(workspace_root, parent)?;
      }
    }
    Err(error) => {
      return Err(error)
        .with_context(|| format!("failed to read metadata for {}", target.display()));
    }
  }

  Ok(())
}

pub(crate) fn validate_workspace_write_parent(workspace_root: &Path, parent: &Path) -> Result<()> {
  let existing_parent = nearest_existing_ancestor(parent).with_context(|| {
    format!(
      "failed to locate an existing parent for {}",
      parent.display()
    )
  })?;
  validate_existing_workspace_path(workspace_root, &existing_parent)?;
  validate_no_symlink_components(workspace_root, &existing_parent)
}

pub(crate) fn sanitize_relative_path(relative_path: &str) -> Result<String> {
  let path = Path::new(relative_path);
  if path.is_absolute() {
    bail!("workspace path must be relative");
  }

  let mut sanitized = PathBuf::new();
  for component in path.components() {
    match component {
      std::path::Component::CurDir => {}
      std::path::Component::Normal(segment) => sanitized.push(segment),
      _ => bail!("workspace path must stay inside the selected workspace"),
    }
  }

  if sanitized.as_os_str().is_empty() {
    bail!("workspace path must not be empty");
  }

  Ok(sanitized.to_string_lossy().replace('\\', "/"))
}

fn validate_existing_workspace_path(workspace_root: &Path, path: &Path) -> Result<()> {
  let resolved = fs::canonicalize(path)
    .with_context(|| format!("failed to resolve workspace path {}", path.display()))?;
  if !resolved.starts_with(workspace_root) {
    bail!("workspace path escapes the selected workspace");
  }

  Ok(())
}

fn validate_no_symlink_components(workspace_root: &Path, path: &Path) -> Result<()> {
  let relative = path
    .strip_prefix(workspace_root)
    .with_context(|| format!("failed to relativize {}", path.display()))?;
  let mut candidate = workspace_root.to_path_buf();

  for component in relative.components() {
    let std::path::Component::Normal(segment) = component else {
      continue;
    };
    candidate.push(segment);
    match fs::symlink_metadata(&candidate) {
      Ok(metadata) if metadata.file_type().is_symlink() => {
        bail!("workspace path crosses a symlink");
      }
      Ok(_) => {}
      Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
      Err(error) => {
        return Err(error)
          .with_context(|| format!("failed to read metadata for {}", candidate.display()));
      }
    }
  }

  Ok(())
}

fn nearest_existing_ancestor(path: &Path) -> Option<PathBuf> {
  let mut candidate = path.to_path_buf();
  loop {
    if candidate.exists() {
      return Some(candidate);
    }
    if !candidate.pop() {
      return None;
    }
  }
}
