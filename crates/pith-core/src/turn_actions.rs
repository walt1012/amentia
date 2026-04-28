use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::{generate_diff, list_directory, read_file, search_files, shell_sandbox_summary};

use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::intent_inference::{
  self, infer_requested_file_path, infer_search_query, infer_shell_command, infer_write_intent,
};
use crate::local_responses::{
  build_plan_item, format_directory_result, format_file_result, format_search_result,
  summarize_directory_result, summarize_file_result, summarize_search_result,
};
use crate::plugin_permissions::{
  build_permission_denied_items, permission_is_granted,
};
use crate::runtime_context::{
  PendingApproval, PreparedTurnAction, PreparedTurnSnapshot, RuntimeContext,
  TurnStartExecutionOutput,
};

pub(crate) fn prepare_turn_action(
  context: &mut RuntimeContext,
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  permission_sources: &HashMap<String, Vec<String>>,
) -> PreparedTurnAction {
  let Some(workspace) = workspace else {
    return PreparedTurnAction::NoWorkspace;
  };
  let workspace_root = Path::new(&workspace.root_path);

  if let Some(intent) = infer_write_intent(message) {
    let approval_id =
      permission_is_granted(permission_sources, "file.write").then(|| reserve_approval_id(context));
    return PreparedTurnAction::Write {
      intent,
      approval_id,
    };
  }

  if let Some(command) = infer_shell_command(message) {
    let approval_id =
      permission_is_granted(permission_sources, "shell.exec").then(|| reserve_approval_id(context));
    return PreparedTurnAction::Shell {
      command,
      approval_id,
    };
  }

  if let Some(relative_path) = infer_requested_file_path(message, workspace_root) {
    return PreparedTurnAction::ReadFile { relative_path };
  }

  if let Some(query) = infer_search_query(message) {
    return PreparedTurnAction::Search { query };
  }

  PreparedTurnAction::ListWorkspace
}

fn reserve_approval_id(context: &mut RuntimeContext) -> String {
  let approval_id = format!("approval-{}", context.next_approval_number);
  context.next_approval_number += 1;
  approval_id
}

pub(crate) fn execute_prepared_turn_snapshot(
  snapshot: PreparedTurnSnapshot,
) -> TurnStartExecutionOutput {
  let mut items = vec![TimelineItem {
    kind: "userMessage".to_string(),
    title: "User".to_string(),
    content: snapshot.display_message.clone(),
    attributes: None,
  }];
  let mut pending_active_turn = None;
  let mut pending_approval = None;

  match (&snapshot.workspace, &snapshot.action) {
    (
      Some(workspace),
      PreparedTurnAction::Write {
        intent,
        approval_id,
      },
    ) => {
      execute_write_turn(
        &snapshot,
        workspace,
        intent,
        approval_id,
        &mut items,
        &mut pending_approval,
      );
    }
    (
      Some(workspace),
      PreparedTurnAction::Shell {
        command,
        approval_id,
      },
    ) => {
      execute_shell_turn(
        &snapshot,
        workspace,
        command,
        approval_id,
        &mut items,
        &mut pending_approval,
      );
    }
    (Some(workspace), PreparedTurnAction::ReadFile { relative_path }) => {
      execute_read_turn(
        &snapshot,
        workspace,
        relative_path,
        &mut items,
        &mut pending_active_turn,
      );
    }
    (Some(workspace), PreparedTurnAction::Search { query }) => {
      execute_search_turn(
        &snapshot,
        workspace,
        query,
        &mut items,
        &mut pending_active_turn,
      );
    }
    (Some(workspace), PreparedTurnAction::ListWorkspace) => {
      execute_list_turn(&snapshot, workspace, &mut items, &mut pending_active_turn);
    }
    _ => execute_no_workspace_turn(&snapshot, &mut items),
  }

  TurnStartExecutionOutput {
    thread_id: snapshot.thread_id,
    turn_id: snapshot.turn_id,
    items,
    pending_approval,
    pending_active_turn,
  }
}

fn execute_write_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  intent: &intent_inference::WriteIntent,
  approval_id: &Option<String>,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if approval_id.is_some() {
      format!(
        "Request approval before writing {} in {}.",
        intent.relative_path, workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before writing {} in {}.",
        intent.relative_path, workspace.display_name
      )
    },
  ));
  let Some(approval_id) = approval_id else {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.write",
      "prepare a file write",
      &workspace.display_name,
      HashMap::from([("relativePath".to_string(), intent.relative_path.clone())]),
    ));
    return;
  };

  let approval = PendingApproval {
    id: approval_id.clone(),
    thread_id: snapshot.thread_id.clone(),
    action: "write_file".to_string(),
    title: format!("Write {}", intent.relative_path),
    relative_path: intent.relative_path.clone(),
    content: Some(intent.content.clone()),
    command: None,
  };
  *pending_approval = Some(approval.clone());

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "generate_diff".to_string(),
    content: intent.relative_path.clone(),
    attributes: None,
  });
  match generate_diff(
    Path::new(&workspace.root_path),
    &intent.relative_path,
    &intent.content,
  ) {
    Ok(diff) => {
      items.push(TimelineItem {
        kind: "diffArtifact".to_string(),
        title: "Diff Preview".to_string(),
        content: diff,
        attributes: Some(HashMap::from([
          ("action".to_string(), "write_file".to_string()),
          ("relativePath".to_string(), intent.relative_path.clone()),
        ])),
      });
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "generate_diff failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
    }
  }
  items.push(TimelineItem {
    kind: "approvalRequested".to_string(),
    title: "Approval Requested".to_string(),
    content: format!(
      "Pith wants to write {} in {}.",
      intent.relative_path, workspace.display_name
    ),
    attributes: Some(HashMap::from([
      ("approvalId".to_string(), approval.id.clone()),
      ("action".to_string(), approval.action.clone()),
      ("relativePath".to_string(), approval.relative_path.clone()),
    ])),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "Pith prepared a write for {} and is waiting for your approval.",
      intent.relative_path
    ),
    attributes: None,
  });
}

fn execute_shell_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  command: &str,
  approval_id: &Option<String>,
  items: &mut Vec<TimelineItem>,
  pending_approval: &mut Option<PendingApproval>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if approval_id.is_some() {
      format!(
        "Request approval before running a shell command in {}.",
        workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before running a shell command in {}.",
        workspace.display_name
      )
    },
  ));
  let Some(approval_id) = approval_id else {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "shell.exec",
      "run a shell command",
      &workspace.display_name,
      HashMap::from([("command".to_string(), command.to_string())]),
    ));
    return;
  };

  let sandbox = shell_sandbox_summary(Path::new(&workspace.root_path));
  let approval = PendingApproval {
    id: approval_id.clone(),
    thread_id: snapshot.thread_id.clone(),
    action: "run_shell".to_string(),
    title: "Run Shell Command".to_string(),
    relative_path: ".".to_string(),
    content: None,
    command: Some(command.to_string()),
  };
  *pending_approval = Some(approval.clone());

  items.push(TimelineItem {
    kind: "approvalRequested".to_string(),
    title: "Approval Requested".to_string(),
    content: format!(
      "Pith wants to run this shell command in {}:\n{}\n\n{}",
      workspace.display_name,
      command,
      sandbox.display_line()
    ),
    attributes: Some({
      let mut attributes = sandbox.attributes();
      attributes.extend(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("action".to_string(), approval.action.clone()),
        ("command".to_string(), command.to_string()),
      ]));
      attributes
    }),
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: "Pith is waiting for your approval before running the shell command.".to_string(),
    attributes: None,
  });
}

fn execute_read_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  relative_path: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Inspect {} in {} with the built-in read_file tool.",
        relative_path, workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before inspecting {} in {}.",
        relative_path, workspace.display_name
      )
    },
  ));
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "inspect a file",
      &workspace.display_name,
      HashMap::from([("relativePath".to_string(), relative_path.to_string())]),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "read_file".to_string(),
    content: relative_path.to_string(),
    attributes: None,
  });

  match read_file(Path::new(&workspace.root_path), relative_path, 4096) {
    Ok(result) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "read_file result".to_string(),
        content: format_file_result(&result),
        attributes: None,
      });
      let (summary, summary_attributes) = summarize_file_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        &result,
      );
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "read_file failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not inspect that file in {}. Try another path inside the workspace.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}

fn execute_search_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  query: &str,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Search {} for matches to \"{}\" with the built-in search_files tool.",
        workspace.display_name, query
      )
    } else {
      format!(
        "Check plugin permissions before searching {} for \"{}\".",
        workspace.display_name, query
      )
    },
  ));
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "search files",
      &workspace.display_name,
      HashMap::from([("query".to_string(), query.to_string())]),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "search_files".to_string(),
    content: query.to_string(),
    attributes: None,
  });

  match search_files(Path::new(&workspace.root_path), query, 12) {
    Ok(matches) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "search_files result".to_string(),
        content: format_search_result(query, &matches),
        attributes: None,
      });
      let (summary, summary_attributes) = summarize_search_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        query,
        &matches,
      );
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "search_files failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not search {} yet. Try a shorter query or re-open the workspace.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}

fn execute_list_turn(
  snapshot: &PreparedTurnSnapshot,
  workspace: &WorkspaceSummary,
  items: &mut Vec<TimelineItem>,
  pending_active_turn: &mut Option<ActiveTurn>,
) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    Some(workspace),
    if permission_is_granted(&snapshot.permission_sources, "file.read") {
      format!(
        "Inspect the root of {} with the built-in list_directory tool.",
        workspace.display_name
      )
    } else {
      format!(
        "Check plugin permissions before inspecting the root of {}.",
        workspace.display_name
      )
    },
  ));
  if !permission_is_granted(&snapshot.permission_sources, "file.read") {
    items.extend(build_permission_denied_items(
      &snapshot.permission_sources,
      "file.read",
      "inspect the workspace",
      &workspace.display_name,
      HashMap::new(),
    ));
    return;
  }

  items.push(TimelineItem {
    kind: "toolStart".to_string(),
    title: "list_directory".to_string(),
    content: ".".to_string(),
    attributes: None,
  });

  match list_directory(Path::new(&workspace.root_path), None, 24) {
    Ok(entries) => {
      items.push(TimelineItem {
        kind: "toolResult".to_string(),
        title: "list_directory result".to_string(),
        content: format_directory_result(&entries),
        attributes: None,
      });
      let (summary, summary_attributes) = summarize_directory_result(
        &snapshot.model_runtime,
        &snapshot.memory_notes,
        &snapshot.thread_title,
        &workspace.display_name,
        &entries,
      );
      *pending_active_turn = start_streaming_assistant_turn(
        &snapshot.thread_id,
        &snapshot.turn_id,
        items,
        summary,
        summary_attributes,
      );
    }
    Err(error) => {
      items.push(TimelineItem {
        kind: "warning".to_string(),
        title: "list_directory failed".to_string(),
        content: error.to_string(),
        attributes: None,
      });
      items.push(TimelineItem {
        kind: "assistantMessage".to_string(),
        title: "Assistant".to_string(),
        content: format!(
          "Pith could not inspect the root of {} yet. Re-open the workspace and try again.",
          workspace.display_name
        ),
        attributes: None,
      });
    }
  }
}

fn execute_no_workspace_turn(snapshot: &PreparedTurnSnapshot, items: &mut Vec<TimelineItem>) {
  items.push(build_plan_item(
    &snapshot.model_runtime,
    &snapshot.memory_notes,
    &snapshot.message,
    None,
    "Wait for a workspace before running filesystem tools.".to_string(),
  ));
  items.push(TimelineItem {
    kind: "warning".to_string(),
    title: "Workspace Required".to_string(),
    content: "Open a workspace before asking Pith to inspect files.".to_string(),
    attributes: None,
  });
  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "Pith received your message in {}, but project tools need an opened workspace first.",
      snapshot.thread_title
    ),
    attributes: None,
  });
}
