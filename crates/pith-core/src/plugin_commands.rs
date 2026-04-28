use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use pith_memory::MemoryNote;
use pith_plugin_host::{build_command_registry, PluginCommandEntry as HostPluginCommandEntry};
use pith_protocol::{
  JsonRpcRequest, JsonRpcResponse, PluginCommandRunParams, TimelineItem, TurnStartResult,
  WorkspaceSummary,
};
use pith_tools::read_file;

use super::approval_state::approvals_for_thread;
use super::context_compaction::{merge_context_pack_attributes, pack_memory_context, ContextPack};
use super::request_params::parse_required_params;
use super::thread_summary::refresh_thread_summary_note;
use super::RuntimeContext;

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const GIT_COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub struct PreparedPluginCommandRun {
  request_id: serde_json::Value,
  snapshot: PluginCommandSnapshot,
}

#[derive(Debug)]
pub struct CompletedPluginCommandRun {
  request_id: serde_json::Value,
  output: std::result::Result<PluginCommandOutput, (i32, String)>,
}

#[derive(Debug)]
struct PluginCommandSnapshot {
  thread_id: String,
  command: HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<String>,
  command_item: TimelineItem,
  memory_notes: Vec<MemoryNote>,
}

#[derive(Debug)]
struct PluginCommandOutput {
  thread_id: String,
  command: HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<String>,
  items: Vec<TimelineItem>,
}

struct GitCommandOutput {
  stdout: Vec<u8>,
  success: bool,
  timed_out: bool,
}

pub(super) fn handle_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let prepared = match prepare_plugin_command_run(context, request) {
    Ok(prepared) => prepared,
    Err(response) => return response,
  };
  let completed = execute_prepared_plugin_command_run(prepared);
  complete_prepared_plugin_command_run(context, completed)
}

pub fn prepare_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> std::result::Result<PreparedPluginCommandRun, JsonRpcResponse> {
  let params = parse_required_params::<PluginCommandRunParams>(&request, "plugin/commandRun")?;

  let Some(command) = build_command_registry(&context.plugins)
    .into_iter()
    .find(|command| command.command_id == params.command_id)
  else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32052,
      "Plugin command not found",
    ));
  };
  if !is_supported_builtin_execution(command.execution_kind.as_deref()) {
    return Err(JsonRpcResponse::error(
      request.id,
      -32053,
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        command.command_id
      ),
    ));
  }

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return Err(JsonRpcResponse::error(
      request.id,
      -32004,
      "Thread not found",
    ));
  };

  let workspace = thread
    .workspace
    .clone()
    .or_else(|| context.workspace.clone());
  let input = params
    .input
    .as_deref()
    .map(str::trim)
    .filter(|input| !input.is_empty())
    .map(str::to_string);
  let memory_query = input
    .as_deref()
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
  let command_item = build_plugin_command_timeline_item(
    &command,
    workspace.as_ref(),
    input.as_deref(),
    &context_pack,
  );

  Ok(PreparedPluginCommandRun {
    request_id: request.id,
    snapshot: PluginCommandSnapshot {
      thread_id: params.thread_id,
      command,
      workspace,
      input,
      command_item,
      memory_notes: context.memory_notes.clone(),
    },
  })
}

pub fn execute_prepared_plugin_command_run(
  prepared: PreparedPluginCommandRun,
) -> CompletedPluginCommandRun {
  CompletedPluginCommandRun {
    request_id: prepared.request_id,
    output: execute_plugin_command_snapshot(prepared.snapshot),
  }
}

pub fn complete_prepared_plugin_command_run(
  context: &mut RuntimeContext,
  completed: CompletedPluginCommandRun,
) -> JsonRpcResponse {
  match completed.output {
    Ok(output) => match complete_plugin_command_items(context, output) {
      Ok(result) => JsonRpcResponse::success(completed.request_id, &result),
      Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
    },
    Err((code, message)) => JsonRpcResponse::error(completed.request_id, code, message),
  }
}

fn is_supported_builtin_execution(execution_kind: Option<&str>) -> bool {
  matches!(
    execution_kind,
    Some(
      "builtin.workspaceReadmeNote" | "builtin.shellSessionSummary" | "builtin.reviewDiffSummary"
    )
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

fn execute_plugin_command_snapshot(
  snapshot: PluginCommandSnapshot,
) -> std::result::Result<PluginCommandOutput, (i32, String)> {
  let execution_kind = snapshot.command.execution_kind.as_deref().ok_or_else(|| {
    (
      -32053,
      format!(
        "Plugin command `{}` requires an explicit execution contract.",
        snapshot.command.command_id
      ),
    )
  })?;
  let result_content = match execution_kind {
    "builtin.workspaceReadmeNote" => build_workspace_readme_note_result(
      &snapshot.command,
      snapshot.workspace.as_ref(),
      snapshot.input.as_deref(),
    ),
    "builtin.shellSessionSummary" => {
      build_shell_session_summary_result(&snapshot.memory_notes, snapshot.workspace.as_ref())
    }
    "builtin.reviewDiffSummary" => {
      build_review_diff_summary_result(&snapshot.command, snapshot.workspace.as_ref())
    }
    _ => {
      return Err((
        -32053,
        format!(
          "Plugin command `{}` requires an explicit execution contract.",
          snapshot.command.command_id
        ),
      ))
    }
  };

  let result_item =
    build_plugin_result_timeline_item(&snapshot.command, execution_kind, result_content.clone());
  let assistant_item = TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed through {}.\n\n{}",
      snapshot.command.title, snapshot.command.plugin_display_name, result_content
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), snapshot.command.plugin_id.clone()),
      ("commandId".to_string(), snapshot.command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
    ])),
  };
  Ok(PluginCommandOutput {
    thread_id: snapshot.thread_id,
    command: snapshot.command,
    workspace: snapshot.workspace,
    input: snapshot.input,
    items: vec![snapshot.command_item, result_item, assistant_item],
  })
}

fn complete_plugin_command_items(
  context: &mut RuntimeContext,
  output: PluginCommandOutput,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let PluginCommandOutput {
    thread_id: requested_thread_id,
    command,
    workspace,
    input,
    mut items,
  } = output;
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

  match maybe_capture_plugin_command_memory(
    context,
    &thread_id,
    &command,
    input.as_deref(),
    workspace.as_ref(),
    &items,
  ) {
    Ok(Some(memory_item)) => {
      if let Some(thread) = context
        .threads
        .iter_mut()
        .find(|thread| thread.summary.id == thread_id)
      {
        thread.items.push(memory_item.clone());
      }
      items.push(memory_item);
      context
        .persist_runtime_state()
        .map_err(|error| (-32010, error.to_string()))?;
      refresh_thread_summary_note(context, &thread_id)
        .map_err(|error| (-32012, error.to_string()))?;
    }
    Ok(None) => {}
    Err(error) => {
      let warning_item = build_plugin_command_memory_warning_item(&command, error.to_string());
      if let Some(thread) = context
        .threads
        .iter_mut()
        .find(|thread| thread.summary.id == thread_id)
      {
        thread.items.push(warning_item.clone());
      }
      items.push(warning_item);
      context
        .persist_runtime_state()
        .map_err(|error| (-32010, error.to_string()))?;
    }
  }

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals: approvals_for_thread(context, &requested_thread_id),
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
  memory_notes: &[MemoryNote],
  workspace: Option<&WorkspaceSummary>,
) -> String {
  let workspace_label = workspace
    .map(|workspace| workspace.display_name.as_str())
    .unwrap_or("the current workspace");
  let shell_notes = memory_notes
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
  let output = run_git_workspace_command(workspace_root, args).ok()?;
  if output.timed_out || !output.success {
    return None;
  }
  Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git_workspace_command(workspace_root: &Path, args: &[&str]) -> Result<GitCommandOutput> {
  let mut child = Command::new("git")
    .arg("-C")
    .arg(workspace_root)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()?;
  let Some(mut stdout) = child.stdout.take() else {
    bail!("git stdout pipe was unavailable");
  };
  let stdout_reader = thread::spawn(move || {
    let mut buffer = vec![];
    let _ = stdout.read_to_end(&mut buffer);
    buffer
  });

  let started_at = Instant::now();
  let (success, timed_out) = loop {
    if let Some(status) = child.try_wait()? {
      break (status.success(), false);
    }

    if started_at.elapsed() >= GIT_COMMAND_TIMEOUT {
      let _ = child.kill();
      let _ = child.wait();
      break (false, true);
    }

    thread::sleep(GIT_COMMAND_POLL_INTERVAL);
  };

  let stdout = stdout_reader.join().unwrap_or_default();
  Ok(GitCommandOutput {
    stdout,
    success,
    timed_out,
  })
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
  workspace: Option<&WorkspaceSummary>,
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
  let workspace = workspace
    .cloned()
    .or_else(|| {
      context
        .threads
        .iter()
        .find(|thread| thread.summary.id == thread_id)
        .and_then(|thread| thread.workspace.clone())
    })
    .or_else(|| context.workspace.clone());
  let Some(workspace) = workspace else {
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
