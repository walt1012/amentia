use std::collections::HashMap;
use std::path::Path;

use pith_memory::{MemoryEvent, MemoryNote};
use pith_model_runtime::LocalModelRuntime;
use pith_plugin_host::PluginCatalogEntry;
use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::run_shell;

use crate::approval_types::PendingApproval;
use crate::local_responses::{format_shell_result, summarize_shell_result};
use crate::plugin_hooks::build_shell_completed_hook_items;
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::turn_tool_provenance::workspace_tool_attributes;

use super::approval_execution_events::ApprovalExecutionEvents;
use super::approval_execution_timeline::{assistant_item, tool_start_item, warning_item};

pub(super) fn append_approved_shell_execution(
  events: &mut ApprovalExecutionEvents,
  approval: &PendingApproval,
  workspace: &WorkspaceSummary,
  model_runtime: &LocalModelRuntime,
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
      ],
    )),
  ));

  match run_shell(Path::new(&workspace.root_path), &command, 4096) {
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
        None,
      );
      events.push_item(TimelineItem {
        kind: "toolResult".to_string(),
        title: "run_shell result".to_string(),
        content: format_shell_result(&result),
        attributes: Some({
          let mut attributes = workspace_tool_attributes(
            "run_shell",
            workspace,
            [
              ("approvalId".to_string(), approval.id.clone()),
              ("command".to_string(), command.clone()),
              ("exitCode".to_string(), result.exit_code.to_string()),
            ],
          );
          attributes.extend(result.sandbox.attributes());
          attributes.extend(result.output_context.attributes());
          attributes
        }),
      });
      events.push_item(assistant_item(summary, Some(summary_attributes)));
      let (hook_items, memory_captures) =
        build_shell_completed_hook_items(plugins, workspace, &command, &result);
      events.extend_hook_memory_captures(memory_captures);
      events.extend_items(hook_items);
    }
    Err(error) => events.push_item(warning_item("run_shell failed", error.to_string())),
  }
}
