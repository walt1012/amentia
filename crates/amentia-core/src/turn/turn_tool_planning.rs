use std::path::Path;

use amentia_plugin_host::PluginCatalogEntry;
use amentia_protocol::WorkspaceSummary;

use super::turn_plugin_routing::{
  infer_explicit_plugin_command_route, infer_natural_plugin_command_route,
  ExplicitPluginCommandRoute,
};
use crate::intent_inference::{
  infer_explicit_web_search_intent, infer_fresh_web_search_intent, infer_model_web_search_intent,
  infer_requested_file_path, infer_search_query, infer_shell_command, infer_write_intent,
  WebSearchIntent, WriteIntent,
};

#[derive(Debug)]
pub(crate) enum InitialToolPlan {
  NoWorkspace,
  Write { intent: WriteIntent },
  Shell { command: String },
  PluginCommand { route: ExplicitPluginCommandRoute },
  ReadFile { relative_path: String },
  Search { query: String },
  WebSearch { intent: WebSearchIntent },
  WebSearchCandidate { intent: WebSearchIntent },
  ListWorkspace,
}

pub(crate) fn plan_initial_turn_tool(
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  plugins: &[PluginCatalogEntry],
) -> InitialToolPlan {
  if let Some(intent) = infer_explicit_web_search_intent(message) {
    return InitialToolPlan::WebSearch { intent };
  }

  if let Some(route) = infer_explicit_plugin_command_route(message) {
    return InitialToolPlan::PluginCommand { route };
  }

  if let Some(route) = infer_natural_plugin_command_route(message, plugins) {
    return InitialToolPlan::PluginCommand { route };
  }

  let Some(workspace) = workspace else {
    if let Some(intent) = infer_fresh_web_search_intent(message) {
      return InitialToolPlan::WebSearchCandidate { intent };
    }
    if let Some(intent) = infer_model_web_search_intent(message) {
      return InitialToolPlan::WebSearchCandidate { intent };
    }
    return InitialToolPlan::NoWorkspace;
  };
  let workspace_root = Path::new(&workspace.root_path);

  if let Some(intent) = infer_write_intent(message) {
    return InitialToolPlan::Write { intent };
  }

  if let Some(command) = infer_shell_command(message) {
    return InitialToolPlan::Shell { command };
  }

  if let Some(relative_path) = infer_requested_file_path(message, workspace_root) {
    return InitialToolPlan::ReadFile { relative_path };
  }

  if let Some(intent) = infer_fresh_web_search_intent(message) {
    return InitialToolPlan::WebSearchCandidate { intent };
  }

  if let Some(query) = infer_search_query(message) {
    return InitialToolPlan::Search { query };
  }

  if let Some(intent) = infer_model_web_search_intent(message) {
    return InitialToolPlan::WebSearchCandidate { intent };
  }

  InitialToolPlan::ListWorkspace
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::path::PathBuf;
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[test]
  fn explicit_web_search_wins_without_workspace() {
    let plan = plan_initial_turn_tool("web search latest local model news", None, &[]);

    assert!(matches!(plan, InitialToolPlan::WebSearch { .. }));
  }

  #[test]
  fn no_workspace_plan_stays_light_without_external_intent() {
    let plan = plan_initial_turn_tool("help me understand this workspace", None, &[]);

    assert!(matches!(plan, InitialToolPlan::NoWorkspace));
  }

  #[test]
  fn workspace_read_request_plans_read_file() {
    let workspace_root = unique_temp_workspace("planner-read");
    fs::create_dir_all(&workspace_root).expect("workspace");
    fs::write(workspace_root.join("README.md"), "hello").expect("readme");
    let workspace = WorkspaceSummary {
      root_path: workspace_root.display().to_string(),
      display_name: "amentia".to_string(),
    };
    let plan = plan_initial_turn_tool("read README.md", Some(&workspace), &[]);

    match plan {
      InitialToolPlan::ReadFile { relative_path } => assert_eq!(relative_path, "README.md"),
      _ => panic!("expected read file plan"),
    }
    let _ = fs::remove_dir_all(workspace_root);
  }

  fn unique_temp_workspace(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock")
      .as_nanos();
    std::env::temp_dir().join(format!("amentia-core-{prefix}-{nonce}"))
  }
}
