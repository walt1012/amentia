use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use pith_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, TimelineItem, TurnStartResult,
  WorkspaceSummary,
};
use pith_tools::read_file;

use super::context_compaction::{merge_context_pack_attributes, pack_memory_context, ContextPack};
use super::request_params::parse_required_params;
use super::{approvals_for_thread, refresh_thread_summary_note, RuntimeContext};

pub(super) fn handle_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match parse_required_params::<PluginCommandRunParams>(&request, "plugin/commandRun")
  {
    Ok(params) => params,
    Err(response) => return response,
  };

  let Some(command) = build_command_registry(&context.plugins)
    .into_iter()
    .find(|command| command.command_id == params.command_id)
  else {
    return JsonRpcResponse::error(request.id, -32052, "Plugin command not found");
  };

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  let workspace = thread
    .workspace
    .clone()
    .or_else(|| context.workspace.clone());
  let command_input = params
    .input
    .as_deref()
    .map(str::trim)
    .filter(|input| !input.is_empty());
  let memory_query = command_input
    .map(|input| {
      format!(
        "{} {} {} {}",
        command.title, command.description, command.prompt, input
      )
    })
    .unwrap_or_else(|| {
      format!(
        "{} {} {}",
        command.title, command.description, command.prompt
      )
    });
  let context_pack = pack_memory_context(
    &context.model_runtime,
    &context.memory_notes,
    workspace.as_ref().map(|entry| entry.display_name.as_str()),
    &memory_query,
  );
  let command_item =
    build_plugin_command_timeline_item(&command, workspace.as_ref(), command_input, &context_pack);
  if let Some(result) = execute_builtin_plugin_command(
    context,
    &params.thread_id,
    &command,
    workspace.clone(),
    command_input,
    command_item.clone(),
  ) {
    return match result {
      Ok(result) => JsonRpcResponse::success(request.id, &result),
      Err((code, message)) => JsonRpcResponse::error(request.id, code, message),
    };
  }

  JsonRpcResponse::error(
    request.id,
    -32053,
    format!(
      "Plugin command `{}` requires an explicit execution contract.",
      command.command_id
    ),
  )
}

fn build_plugin_command_timeline_item(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  context_pack: &ContextPack,
) -> TimelineItem {
  let mut attributes = HashMap::from([
    ("commandId".to_string(), command.command_id.clone()),
    ("pluginId".to_string(), command.plugin_id.clone()),
    (
      "pluginDisplayName".to_string(),
      command.plugin_display_name.clone(),
    ),
    ("sourcePath".to_string(), command.source_path.clone()),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  if let Some(input) = input {
    attributes.insert("commandInput".to_string(), input.to_string());
  }
  if let Some(execution_kind) = command.execution_kind.as_ref() {
    attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  merge_context_pack_attributes(&mut attributes, context_pack);

  let workspace_label = workspace
    .map(|entry| entry.display_name.clone())
    .unwrap_or_else(|| "No Workspace".to_string());
  let mut content = format!(
    "Run {} from {} in {}.\n{}",
    command.title, command.plugin_display_name, workspace_label, command.description
  );
  if let Some(input) = input {
    content.push_str(&format!("\nCommand input: {input}"));
  }

  TimelineItem {
    kind: "pluginCommand".to_string(),
    title: command.title.clone(),
    content,
    attributes: Some(attributes),
  }
}

fn execute_builtin_plugin_command(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<&str>,
  command_item: TimelineItem,
) -> Option<std::result::Result<TurnStartResult, (i32, String)>> {
  let execution_kind = command.execution_kind.as_deref()?;
  let result_content = match execution_kind {
    "builtin.workspaceReadmeNote" => {
      build_workspace_readme_note_result(command, workspace.as_ref(), input)
    }
    "builtin.shellSessionSummary" => {
      build_shell_session_summary_result(context, workspace.as_ref())
    }
    "builtin.reviewDiffSummary" => build_review_diff_summary_result(command, workspace.as_ref()),
    _ => return None,
  };

  let result_item =
    build_plugin_result_timeline_item(command, execution_kind, result_content.clone());
  let assistant_item = TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed through {}.\n\n{}",
      command.title, command.plugin_display_name, result_content
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
    ])),
  };
  let items = vec![command_item, result_item, assistant_item];
  let result = complete_plugin_command_items(context, thread_id, workspace, items);

  Some(result.and_then(|mut result| {
    match maybe_capture_plugin_command_memory(context, thread_id, command, input, &result.items) {
      Ok(Some(memory_item)) => {
        if let Some(thread) = context
          .threads
          .iter_mut()
          .find(|thread| thread.summary.id == thread_id)
        {
          thread.items.push(memory_item.clone());
        }
        result.items.push(memory_item);
        context
          .persist_runtime_state()
          .map_err(|error| (-32010, error.to_string()))?;
        refresh_thread_summary_note(context, thread_id)
          .map_err(|error| (-32012, error.to_string()))?;
      }
      Ok(None) => {}
      Err(error) => {
        let warning_item = build_plugin_command_memory_warning_item(command, error.to_string());
        if let Some(thread) = context
          .threads
          .iter_mut()
          .find(|thread| thread.summary.id == thread_id)
        {
          thread.items.push(warning_item.clone());
        }
        result.items.push(warning_item);
        context
          .persist_runtime_state()
          .map_err(|error| (-32010, error.to_string()))?;
      }
    }
    Ok(result)
  }))
}

fn complete_plugin_command_items(
  context: &mut RuntimeContext,
  requested_thread_id: &str,
  workspace: Option<WorkspaceSummary>,
  items: Vec<TimelineItem>,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let (thread_id, turn_id) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == requested_thread_id)
    else {
      return Err((-32004, "Thread not found".to_string()));
    };

    if thread.workspace.is_none() {
      thread.workspace = workspace.clone();
    }
    thread.turn_count += 1;
    let turn_id = format!("{}-turn-{}", thread.summary.id, thread.turn_count);
    thread.summary.status = match &thread.workspace {
      Some(workspace) => format!(
        "{} plugin command(s) in {}",
        thread.turn_count, workspace.display_name
      ),
      None => format!("{} plugin command(s)", thread.turn_count),
    };
    thread.items.extend(items.clone());
    thread.summary.status = "Ready".to_string();
    (thread.summary.id.clone(), turn_id)
  };

  context
    .persist_runtime_state()
    .map_err(|error| (-32010, error.to_string()))?;
  refresh_thread_summary_note(context, &thread_id).map_err(|error| (-32012, error.to_string()))?;

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals: approvals_for_thread(context, requested_thread_id),
    active_turn_id: None,
  })
}

fn build_plugin_result_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  content: String,
) -> TimelineItem {
  TimelineItem {
    kind: "pluginResult".to_string(),
    title: format!("{} Result", command.title),
    content,
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
      ("sourcePath".to_string(), command.source_path.clone()),
    ])),
  }
}

fn build_workspace_readme_note_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
) -> String {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return "This command cannot read workspace files because its plugin does not declare `file.read`."
      .to_string();
  }
  let Some(workspace) = workspace else {
    return "Open a workspace before capturing a workspace note.".to_string();
  };

  match read_file(Path::new(&workspace.root_path), "README.md", 4096) {
    Ok(result) => {
      let summary = compact_text_preview(&result.content, 10, 900);
      let input_summary = input
        .map(|value| format!("\nOperator input: {}", value.trim()))
        .unwrap_or_default();
      format!(
        "Workspace note candidate from README.md in {}.{}\n\n{}",
        workspace.display_name, input_summary, summary
      )
    }
    Err(error) => format!(
      "Could not capture a README-based note in {}: {}",
      workspace.display_name, error
    ),
  }
}

fn build_shell_session_summary_result(
  context: &RuntimeContext,
  workspace: Option<&WorkspaceSummary>,
) -> String {
  let workspace_label = workspace
    .map(|workspace| workspace.display_name.as_str())
    .unwrap_or("the current workspace");
  let shell_notes = context
    .memory_notes
    .iter()
    .filter(|note| {
      note.source == "plugin.shell-recorder"
        || note.tags.iter().any(|tag| tag == "shell" || tag == "hook")
    })
    .rev()
    .take(5)
    .map(|note| {
      format!(
        "- {}: {}",
        note.title,
        compact_text_preview(&note.body, 2, 220)
      )
    })
    .collect::<Vec<_>>();

  if shell_notes.is_empty() {
    return format!(
      "No shell completion notes are recorded for {} yet. Enable Shell Recorder and approve shell commands to build this timeline.",
      workspace_label
    );
  }

  format!(
    "Recent shell activity for {}:\n{}",
    workspace_label,
    shell_notes.join("\n")
  )
}

fn build_review_diff_summary_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
) -> String {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return "This command cannot inspect the workspace because its plugin does not declare `file.read`."
      .to_string();
  }
  let Some(workspace) = workspace else {
    return "Open a workspace before inspecting the current diff.".to_string();
  };
  let workspace_root = Path::new(&workspace.root_path);
  let stat = git_workspace_output(workspace_root, &["diff", "--stat"]);
  let names = git_workspace_output(workspace_root, &["diff", "--name-only"]);

  match (stat, names) {
    (Some(stat), Some(names)) if !stat.trim().is_empty() || !names.trim().is_empty() => {
      format!(
        "Current diff snapshot for {}.\n\nChanged files:\n{}\n\nDiff stat:\n{}\n\nReview focus:\n- Check behavioral regressions first.\n- Verify missing tests around changed paths.\n- Inspect risky file writes before approving follow-up changes.",
        workspace.display_name,
        compact_text_preview(&names, 20, 900),
        compact_text_preview(&stat, 20, 1200)
      )
    }
    (Some(_), Some(_)) => format!(
      "No active git diff was detected in {}. The review command is ready once files change.",
      workspace.display_name
    ),
    _ => format!(
      "Could not read a git diff in {}. Ensure the workspace is a git repository and git is available.",
      workspace.display_name
    ),
  }
}

fn git_workspace_output(workspace_root: &Path, args: &[&str]) -> Option<String> {
  let output = Command::new("git")
    .arg("-C")
    .arg(workspace_root)
    .args(args)
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn compact_text_preview(content: &str, max_lines: usize, max_chars: usize) -> String {
  let mut preview = content
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .take(max_lines)
    .collect::<Vec<_>>()
    .join("\n");
  if preview.is_empty() {
    preview = "No content available.".to_string();
  }
  if preview.chars().count() > max_chars {
    preview = preview.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
  }
  preview
}

fn maybe_capture_plugin_command_memory(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  input: Option<&str>,
  items: &[TimelineItem],
) -> Result<Option<TimelineItem>> {
  let Some(note_title) = command.memory_note_title.as_ref() else {
    return Ok(None);
  };
  let Some(assistant_message) = items
    .iter()
    .rev()
    .find(|item| item.kind == "assistantMessage")
  else {
    return Ok(None);
  };
  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == thread_id)
  else {
    return Ok(None);
  };
  let Some(workspace) = thread
    .workspace
    .as_ref()
    .or(context.workspace.as_ref())
    .cloned()
  else {
    return Ok(None);
  };

  let note_body =
    build_plugin_command_memory_body(command, &workspace, input, &assistant_message.content);
  let note_source = command
    .memory_note_source
    .clone()
    .unwrap_or_else(|| format!("plugin.{}", command.plugin_id));
  let note_tags = plugin_command_memory_tags(command);
  let note = context.create_memory_note(
    note_title.clone(),
    note_body,
    workspace.display_name.clone(),
    note_source,
    note_tags,
  )?;

  Ok(Some(TimelineItem {
    kind: "system".to_string(),
    title: "Memory Note Saved".to_string(),
    content: format!(
      "Saved workspace memory note \"{}\" from {}.",
      note.title, command.title
    ),
    attributes: Some(HashMap::from([
      ("memoryNoteId".to_string(), note.id),
      ("memoryNoteTitle".to_string(), note.title),
      ("memoryScope".to_string(), note.scope),
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
    ])),
  }))
}

fn build_plugin_command_memory_body(
  command: &HostPluginCommandEntry,
  workspace: &WorkspaceSummary,
  input: Option<&str>,
  assistant_content: &str,
) -> String {
  let mut body = format!(
    "Plugin: {} ({})\nCommand: {} ({})\nWorkspace: {} at {}.",
    command.plugin_display_name,
    command.plugin_id,
    command.title,
    command.command_id,
    workspace.display_name,
    workspace.root_path
  );
  if let Some(input) = input {
    body.push_str(&format!("\nCommand input: {input}"));
  }
  body.push_str("\n\nCommand result:\n");
  body.push_str(assistant_content.trim());
  body
}

fn plugin_command_memory_tags(command: &HostPluginCommandEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "command".to_string(),
    command.plugin_id.clone(),
    command.command_id.clone(),
  ];
  for tag in &command.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

fn build_plugin_command_memory_warning_item(
  command: &HostPluginCommandEntry,
  error_message: String,
) -> TimelineItem {
  TimelineItem {
    kind: "warning".to_string(),
    title: "Plugin Memory Capture Failed".to_string(),
    content: format!(
      "{} could not save its workspace memory note. {}",
      command.title, error_message
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
    ])),
  }
}
