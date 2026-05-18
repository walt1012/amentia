use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use pith_process::configure_process_group;
use pith_protocol::WorkspaceSummary;

const PLUGIN_RUNNER_TEMP_DIR: &str = ".pith/plugin-runner-tmp";
const PLUGIN_RUNNER_SAFE_PATH: &str = "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";

pub(super) struct PluginRunnerSandbox {
  policy: pith_sandbox::SandboxPolicy,
  plugin_root: PathBuf,
  temporary_root: PathBuf,
}

impl PluginRunnerSandbox {
  pub(super) fn prepare(
    workspace: Option<&WorkspaceSummary>,
    plugin_id: &str,
    plugin_root: &Path,
    allow_network: bool,
  ) -> std::result::Result<Self, (i32, String)> {
    let workspace = workspace.ok_or_else(|| {
      (
        -32054,
        "External plugin commands require an open workspace.".to_string(),
      )
    })?;
    let workspace_root = PathBuf::from(&workspace.root_path)
      .canonicalize()
      .map_err(|error| {
        (
          -32054,
          format!("Workspace root could not be resolved: {error}"),
        )
      })?;
    let plugin_root = plugin_root.canonicalize().map_err(|error| {
      (
        -32054,
        format!("Plugin root could not be resolved: {error}"),
      )
    })?;
    let temporary_root = plugin_runner_temp_root(&workspace_root, plugin_id);
    pith_sandbox::prepare_workspace_temporary_root(&workspace_root, &temporary_root)
      .map_err(|message| (-32054, message))?;

    let policy = pith_sandbox::SandboxPolicy::workspace_read_write(workspace_root)
      .with_temporary_root(temporary_root.clone())
      .with_read_only_root(plugin_root.clone())
      .with_network_access(allow_network);

    Ok(Self {
      policy,
      plugin_root,
      temporary_root,
    })
  }

  pub(super) fn build_command(&self, entrypoint_path: &Path) -> Command {
    let mut command = build_sandboxed_command(entrypoint_path, &self.policy);
    command.env_clear();
    command.current_dir(&self.plugin_root);
    self.apply_environment(&mut command);
    command
  }

  pub(super) fn detail(&self) -> String {
    pith_sandbox::native_sandbox_status(&self.policy).detail
  }

  pub(super) fn attributes(&self) -> HashMap<String, String> {
    let status = pith_sandbox::native_sandbox_status(&self.policy);
    let mut attributes = HashMap::from([
      ("sandboxMode".to_string(), status.mode.clone()),
      ("sandboxBackend".to_string(), status.backend.clone()),
      ("sandboxAvailable".to_string(), status.available.to_string()),
      ("sandboxActive".to_string(), status.active.to_string()),
      (
        "sandboxNetworkAllowed".to_string(),
        status.network_allowed.to_string(),
      ),
      (
        "sandboxNetworkPolicy".to_string(),
        status.network_policy().to_string(),
      ),
      ("sandboxDetail".to_string(), status.detail),
    ]);
    if let Some(temporary_root) = status.temporary_root {
      attributes.insert("sandboxTempRoot".to_string(), temporary_root.clone());
      attributes.insert("sandboxTemporaryRoot".to_string(), temporary_root);
    }
    if !status.writable_roots.is_empty() {
      attributes.insert(
        "sandboxWritableRoots".to_string(),
        status.writable_roots.join("\n"),
      );
    }
    attributes
  }

  fn apply_environment(&self, command: &mut Command) {
    command
      .env("PATH", PLUGIN_RUNNER_SAFE_PATH)
      .env("HOME", &self.temporary_root)
      .env("LANG", "en_US.UTF-8")
      .env("LC_ALL", "en_US.UTF-8")
      .env("TMPDIR", &self.temporary_root)
      .env("TMP", &self.temporary_root)
      .env("TEMP", &self.temporary_root)
      .env("PITH_PLUGIN_SANDBOX_TEMP", &self.temporary_root);
  }
}

#[cfg(target_os = "macos")]
fn build_sandboxed_command(
  entrypoint_path: &Path,
  policy: &pith_sandbox::SandboxPolicy,
) -> Command {
  if pith_sandbox::native_sandbox_available() {
    let profile = pith_sandbox::macos_seatbelt_profile(policy);
    let mut command = Command::new(pith_sandbox::macos_sandbox_exec_path());
    command.arg("-p").arg(profile).arg(entrypoint_path);
    configure_process_group(&mut command);
    return command;
  }

  build_process_command(entrypoint_path)
}

#[cfg(not(target_os = "macos"))]
fn build_sandboxed_command(
  entrypoint_path: &Path,
  _policy: &pith_sandbox::SandboxPolicy,
) -> Command {
  build_process_command(entrypoint_path)
}

fn build_process_command(entrypoint_path: &Path) -> Command {
  let mut command = Command::new(entrypoint_path);
  configure_process_group(&mut command);
  command
}

fn plugin_runner_temp_root(workspace_root: &Path, plugin_id: &str) -> PathBuf {
  workspace_root
    .join(PLUGIN_RUNNER_TEMP_DIR)
    .join(safe_plugin_id_segment(plugin_id))
}

fn safe_plugin_id_segment(plugin_id: &str) -> String {
  let segment = plugin_id
    .chars()
    .map(|character| {
      if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
        character
      } else {
        '_'
      }
    })
    .collect::<String>();
  let segment = segment.trim_matches('.').to_string();
  if segment.is_empty() {
    "plugin".to_string()
  } else {
    segment
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::fs;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[test]
  fn plugin_id_segment_replaces_path_unsafe_characters() {
    assert_eq!(safe_plugin_id_segment("../notion:sync"), "_notion_sync");
    assert_eq!(safe_plugin_id_segment("..."), "plugin");
    assert_eq!(
      safe_plugin_id_segment("com.example.plugin"),
      "com.example.plugin"
    );
    assert_eq!(safe_plugin_id_segment("notion-sync"), "notion-sync");
  }

  #[cfg(unix)]
  #[test]
  fn prepare_removes_runner_temp_symlink_without_following_it() {
    use std::os::unix::fs::symlink;

    let workspace_root = unique_temp_workspace("plugin-runner-symlink-cleanup");
    let plugin_root = workspace_root.join(".agents/plugins/notion");
    let outside = unique_temp_workspace("plugin-runner-outside");
    let temporary_root = plugin_runner_temp_root(&workspace_root, "notion/sync");
    let outside_file = outside.join("keep.txt");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    fs::create_dir_all(temporary_root.parent().expect("temporary parent"))
      .expect("temporary parent");
    fs::create_dir_all(&outside).expect("outside");
    fs::write(&outside_file, "keep").expect("outside file");
    symlink(&outside, &temporary_root).expect("temporary root symlink");
    let workspace = WorkspaceSummary {
      root_path: workspace_root.display().to_string(),
      display_name: "Test Workspace".to_string(),
    };

    PluginRunnerSandbox::prepare(Some(&workspace), "notion/sync", &plugin_root, false)
      .expect("prepare sandbox");

    assert!(temporary_root.is_dir());
    assert!(outside_file.is_file());

    let _ = fs::remove_dir_all(workspace_root);
    let _ = fs::remove_dir_all(outside);
  }

  #[cfg(unix)]
  #[test]
  fn prepare_rejects_runner_temp_parent_symlink_escape() {
    use std::os::unix::fs::symlink;

    let workspace_root = unique_temp_workspace("plugin-runner-parent-symlink");
    let plugin_root = workspace_root.join(".agents/plugins/notion");
    let outside = unique_temp_workspace("plugin-runner-parent-outside");
    let temporary_root = plugin_runner_temp_root(&workspace_root, "notion/sync");
    let temporary_parent = temporary_root.parent().expect("temporary parent");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    fs::create_dir_all(temporary_parent.parent().expect("pith directory")).expect("pith directory");
    fs::create_dir_all(&outside).expect("outside");
    symlink(&outside, temporary_parent).expect("temporary parent symlink");
    let workspace = WorkspaceSummary {
      root_path: workspace_root.display().to_string(),
      display_name: "Test Workspace".to_string(),
    };

    let error = match PluginRunnerSandbox::prepare(
      Some(&workspace),
      "notion/sync",
      &plugin_root,
      false,
    ) {
      Ok(_) => panic!("parent symlink should fail"),
      Err(error) => error,
    };

    assert_eq!(error.0, -32054);
    assert!(error.1.contains("crosses a symlink"));
    assert!(!outside.join("_notion_sync").exists());

    let _ = fs::remove_dir_all(workspace_root);
    let _ = fs::remove_dir_all(outside);
  }

  #[test]
  fn build_command_uses_explicit_safe_environment() {
    let workspace_root = unique_temp_workspace("plugin-runner-env");
    let plugin_root = workspace_root.join(".agents/plugins/env");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    let workspace = WorkspaceSummary {
      root_path: workspace_root.display().to_string(),
      display_name: "Test Workspace".to_string(),
    };
    let sandbox = PluginRunnerSandbox::prepare(Some(&workspace), "env", &plugin_root, false)
      .expect("prepare sandbox");
    let temporary_root = sandbox.temporary_root.to_string_lossy().to_string();
    let command = sandbox.build_command(&plugin_root.join("run"));
    let environment = command
      .get_envs()
      .filter_map(|(key, value)| {
        Some((
          key.to_string_lossy().to_string(),
          value?.to_string_lossy().to_string(),
        ))
      })
      .collect::<HashMap<_, _>>();

    assert_eq!(
      environment.get("PATH").map(String::as_str),
      Some(PLUGIN_RUNNER_SAFE_PATH)
    );
    assert_eq!(
      environment.get("HOME").map(String::as_str),
      Some(temporary_root.as_str())
    );
    assert_eq!(
      environment
        .get("PITH_PLUGIN_SANDBOX_TEMP")
        .map(String::as_str),
      Some(temporary_root.as_str())
    );

    let _ = fs::remove_dir_all(workspace_root);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("pith-core-{prefix}-{nonce}"))
  }
}
