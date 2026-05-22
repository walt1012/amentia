use std::collections::HashMap;
use std::path::Path;

use pith_memory::{MemoryEvent, MemoryNote};
use pith_model_runtime::{GenerationCancellation, LocalModelRuntime};
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::WorkspaceSummary;
use pith_tools::{run_shell_with_cancellation, ShellCommandResult};

use crate::approval_types::PendingApproval;
use crate::local_responses::{format_shell_result, summarize_shell_result};
use crate::plugin_hooks::build_shell_completed_hook_items;
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::turn::turn_tool_limits::SHELL_OUTPUT_PREVIEW_MAX_BYTES;
use crate::turn_tool_provenance::workspace_tool_attributes;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_timeline::{
  assistant_item, tool_result_item, tool_start_item, warning_item,
};

pub(super) fn append_approved_shell_execution(
  events: &mut ApprovalExecutionEvents,
  approval: &PendingApproval,
  workspace: &WorkspaceSummary,
  model_runtime: &LocalModelRuntime,
  cancellation: &GenerationCancellation,
  memory_notes: &[MemoryNote],
  permission_sources: &HashMap<String, Vec<String>>,
  plugins: &[PluginCatalogEntry],
) {
  if !permission_is_granted(permission_sources, "shell.exec") {
    events.extend_items(build_permission_denied_items(
      permission_sources,
      "shell.exec",
      "complete the approved shell command",
      &workspace.display_name,
      HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        (
          "command".to_string(),
          approval.command.clone().unwrap_or_default(),
        ),
      ]),
    ));
    return;
  }

  let command = approval.command.clone().unwrap_or_default();
  events.push_item(tool_start_item(
    "run_shell",
    command.clone(),
    Some(workspace_tool_attributes(
      "run_shell",
      workspace,
      [
        ("approvalId".to_string(), approval.id.clone()),
        ("command".to_string(), command.clone()),
        (
          "maxOutputBytes".to_string(),
          SHELL_OUTPUT_PREVIEW_MAX_BYTES.to_string(),
        ),
      ],
    )),
  ));

  match run_shell_with_cancellation(
    Path::new(&workspace.root_path),
    &command,
    SHELL_OUTPUT_PREVIEW_MAX_BYTES,
    || cancellation.is_cancelled(),
  ) {
    Ok(result) => {
      events.set_memory_event(MemoryEvent::ShellCommandRan {
        workspace_display_name: workspace.display_name.clone(),
        command: command.clone(),
      });
      let (summary, summary_attributes) = summarize_shell_result(
        model_runtime,
        memory_notes,
        &workspace.display_name,
        &result,
        Some(cancellation),
      );
      let summary_attributes =
        approved_shell_handoff_attributes(summary_attributes, approval, &command, &result);
      events.push_item(tool_result_item(
        "run_shell result",
        format_shell_result(&result),
        Some({
          let mut attributes = workspace_tool_attributes(
            "run_shell",
            workspace,
            [
              ("approvalId".to_string(), approval.id.clone()),
              ("command".to_string(), command.clone()),
              ("exitCode".to_string(), result.exit_code.to_string()),
              ("timedOut".to_string(), result.timed_out.to_string()),
              ("cancelled".to_string(), result.cancelled.to_string()),
              (
                "maxOutputBytes".to_string(),
                SHELL_OUTPUT_PREVIEW_MAX_BYTES.to_string(),
              ),
            ],
          );
          attributes.extend(result.sandbox.attributes());
          attributes.extend(result.output_context.attributes());
          attributes
        }),
      ));
      events.push_item(assistant_item(summary, Some(summary_attributes)));
      let (hook_items, memory_captures) =
        build_shell_completed_hook_items(plugins, workspace, &command, &result);
      events.extend_hook_memory_captures(memory_captures);
      events.extend_items(hook_items);
    }
    Err(error) => events.push_item(warning_item(
      "run_shell failed",
      error.to_string(),
      Some(workspace_tool_attributes(
        "run_shell",
        workspace,
        [
          ("approvalId".to_string(), approval.id.clone()),
          ("command".to_string(), command.clone()),
        ],
      )),
    )),
  }
}

fn approved_shell_handoff_attributes(
  mut attributes: HashMap<String, String>,
  approval: &PendingApproval,
  command: &str,
  result: &ShellCommandResult,
) -> HashMap<String, String> {
  attributes.extend(HashMap::from([
    ("responseRole".to_string(), "actionHandoff".to_string()),
    ("handoffKind".to_string(), "approvedShell".to_string()),
    ("approvalId".to_string(), approval.id.clone()),
    ("action".to_string(), approval.action.clone()),
    ("command".to_string(), command.to_string()),
    ("exitCode".to_string(), result.exit_code.to_string()),
    ("timedOut".to_string(), result.timed_out.to_string()),
    ("cancelled".to_string(), result.cancelled.to_string()),
  ]));
  attributes
}
