pub(super) struct ReadinessSummaryInput<'a> {
  pub(super) status: &'a str,
  pub(super) model_ready: bool,
  pub(super) workspace_ready: bool,
  pub(super) thread_ready: bool,
  pub(super) web_search_permission_ready: bool,
  pub(super) first_request_sent: bool,
  pub(super) pending_approval_count: usize,
  pub(super) active_turn_count: usize,
  pub(super) running_turn_count: usize,
  pub(super) running_approval_count: usize,
  pub(super) running_plugin_command_count: usize,
  pub(super) running_workspace_search_count: usize,
}

pub(super) fn readiness_summary(input: ReadinessSummaryInput<'_>) -> String {
  let ReadinessSummaryInput {
    status,
    model_ready,
    workspace_ready,
    thread_ready,
    web_search_permission_ready,
    first_request_sent,
    pending_approval_count,
    active_turn_count,
    running_turn_count,
    running_approval_count,
    running_plugin_command_count,
    running_workspace_search_count,
  } = input;

  match status {
    "setup_required" if !model_ready => {
      "Download and select one local model to enable local agent work.".to_string()
    }
    "setup_required" if !workspace_ready => {
      "Open a project so tools, memory, and approvals are scoped safely.".to_string()
    }
    "setup_required" if !thread_ready => {
      "Create or resume a project-bound session before local agent work.".to_string()
    }
    "setup_required" if !web_search_permission_ready => {
      "Enable Web Search so Pith can retrieve current information when needed.".to_string()
    }
    "needs_approval" => {
      format!("Pith is waiting on {pending_approval_count} approval(s) before continuing.")
    }
    "running" => {
      let active_execution_count = active_turn_count
        .saturating_add(running_turn_count)
        .saturating_add(running_approval_count)
        .saturating_add(running_plugin_command_count)
        .saturating_add(running_workspace_search_count);
      format!("Pith is working on {active_execution_count} local request(s).")
    }
    "ready" if !first_request_sent => "Pith is ready for the first local request.".to_string(),
    _ => "Pith is ready: model, project, tools, context, plugins, and connections are controlled."
      .to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn summary_uses_project_language_for_missing_workspace() {
    let summary = readiness_summary(input("setup_required", false, true, true, 0));

    assert_eq!(
      summary,
      "Open a project so tools, memory, and approvals are scoped safely."
    );
  }

  #[test]
  fn summary_uses_session_language_for_missing_thread() {
    let summary = readiness_summary(input("setup_required", true, false, true, 0));

    assert_eq!(
      summary,
      "Create or resume a project-bound session before local agent work."
    );
  }

  #[test]
  fn summary_uses_local_request_language_while_running() {
    let summary = readiness_summary(input("running", true, true, true, 2));

    assert_eq!(summary, "Pith is working on 2 local request(s).");
    assert!(!summary.contains("local execution"));
  }

  #[test]
  fn summary_uses_plugin_and_connection_language_when_ready() {
    let summary = readiness_summary(input("ready", true, true, true, 0));

    assert!(summary.contains("project"));
    assert!(summary.contains("plugins"));
    assert!(summary.contains("connections"));
    assert!(!summary.contains("workspace"));
    assert!(!summary.contains("connectors"));
  }

  fn input(
    status: &'static str,
    workspace_ready: bool,
    thread_ready: bool,
    first_request_sent: bool,
    active_turn_count: usize,
  ) -> ReadinessSummaryInput<'static> {
    ReadinessSummaryInput {
      status,
      model_ready: true,
      workspace_ready,
      thread_ready,
      web_search_permission_ready: true,
      first_request_sent,
      pending_approval_count: 0,
      active_turn_count,
      running_turn_count: 0,
      running_approval_count: 0,
      running_plugin_command_count: 0,
      running_workspace_search_count: 0,
    }
  }
}
