use std::env;
use std::path::PathBuf;

pub fn default_plugin_root() -> Option<PathBuf> {
  if let Ok(path) = env::var("PITH_PLUGIN_DIR") {
    return Some(PathBuf::from(path));
  }

  let roots = discovery_roots();
  for root in &roots {
    let candidate = root.join("plugins");
    if candidate.exists() {
      return Some(candidate);
    }
  }

  roots.into_iter().next().map(|root| root.join("plugins"))
}

pub fn configured_plugin_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(path) = env::var("PITH_PLUGIN_DIR") {
    roots.push(PathBuf::from(path));
    if let Ok(local_path) = env::var("PITH_LOCAL_PLUGIN_DIR") {
      let local_root = PathBuf::from(local_path);
      if !roots.contains(&local_root) {
        roots.push(local_root);
      }
    }
    return roots;
  }

  if let Ok(path) = env::var("PITH_LOCAL_PLUGIN_DIR") {
    roots.push(PathBuf::from(path));
  }

  if let Some(default_root) = default_plugin_root() {
    if !roots.contains(&default_root) {
      roots.push(default_root);
    }
  }

  roots
}

pub fn configured_plugin_install_root() -> PathBuf {
  if let Ok(path) = env::var("PITH_LOCAL_PLUGIN_DIR") {
    return PathBuf::from(path);
  }
  if let Ok(path) = env::var("PITH_PLUGIN_DIR") {
    return PathBuf::from(path);
  }
  default_plugin_root()
    .map(|root| root.join("local"))
    .unwrap_or_else(|| PathBuf::from("plugins").join("local"))
}

fn discovery_roots() -> Vec<PathBuf> {
  let mut roots = vec![];

  if let Ok(current_executable) = env::current_exe() {
    if let Some(parent) = current_executable.parent() {
      roots.push(parent.to_path_buf());
    }
  }

  if let Ok(current_directory) = env::current_dir() {
    roots.push(current_directory);
  }

  let mut unique_roots = vec![];
  for root in roots {
    if !unique_roots.contains(&root) {
      unique_roots.push(root);
    }
  }

  unique_roots
}
