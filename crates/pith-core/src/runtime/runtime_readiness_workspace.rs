use pith_protocol::RuntimeReadinessCheck;

use crate::runtime_context::RuntimeContext;

pub(super) fn workspace_check(context: &RuntimeContext) -> RuntimeReadinessCheck {
  let status = if context.workspace_state.is_open() {
    "ready"
  } else {
    "setup_required"
  };

  RuntimeReadinessCheck {
    id: "workspace".to_string(),
    title: "Project".to_string(),
    status: status.to_string(),
    detail: context
      .workspace_state
      .current()
      .map(|workspace| format!("Project tools are ready for {}.", workspace.display_name))
      .unwrap_or_else(|| {
        "Open a project folder to enable files, actions, memory, and approvals.".to_string()
      }),
  }
}

pub(super) fn thread_check(
  thread_ready: bool,
  workspace_thread_count: usize,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "thread".to_string(),
    title: "Session".to_string(),
    status: if thread_ready {
      "ready".to_string()
    } else {
      "setup_required".to_string()
    },
    detail: if thread_ready {
      format!("{workspace_thread_count} session(s) are ready in the current project.")
    } else {
      "Create or resume a session for the current project.".to_string()
    },
  }
}

pub(super) fn first_request_check(
  first_request_sent: bool,
  thread_ready: bool,
) -> RuntimeReadinessCheck {
  RuntimeReadinessCheck {
    id: "firstRequest".to_string(),
    title: "First Request".to_string(),
    status: if first_request_sent {
      "ready".to_string()
    } else if thread_ready {
      "ready_to_send".to_string()
    } else {
      "waiting".to_string()
    },
    detail: if first_request_sent {
      "At least one request has been sent in the current project.".to_string()
    } else if thread_ready {
      "Send one short request to complete first-use setup.".to_string()
    } else {
      "Create or resume a project-bound session before sending the first request.".to_string()
    },
  }
}
