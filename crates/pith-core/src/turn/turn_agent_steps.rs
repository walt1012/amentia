use std::collections::HashMap;

use pith_protocol::TimelineItem;

use crate::request_state::PreparedTurnAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentStepRecord {
  step_id: String,
  step_index: usize,
  invocation: AgentToolInvocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentToolInvocation {
  call_id: String,
  kind: AgentToolKind,
  name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AgentToolKind {
  Connector,
  File,
  Plugin,
  Shell,
  Workspace,
  Web,
}

impl AgentStepRecord {
  pub(crate) fn from_turn_action(
    turn_id: &str,
    step_index: usize,
    action: &PreparedTurnAction,
  ) -> Self {
    let tool_kind = tool_kind_for_action(action);
    let tool_name = tool_name_for_action(action);
    let step_id = format!("{turn_id}-step-{step_index}");
    Self {
      step_id,
      step_index,
      invocation: AgentToolInvocation {
        call_id: format!("{turn_id}-step-{step_index}-tool-1"),
        kind: tool_kind,
        name: tool_name,
      },
    }
  }

  pub(crate) fn tag_items(&self, items: &mut [TimelineItem], outcome: AgentStepOutcome) {
    for item in items {
      let phase = item_phase(item);
      let attributes = item.attributes.get_or_insert_with(HashMap::new);
      attributes.insert("agentStepId".to_string(), self.step_id.clone());
      attributes.insert("agentStepIndex".to_string(), self.step_index.to_string());
      attributes.insert("agentStepPhase".to_string(), phase.as_str().to_string());
      attributes.insert("agentStepStatus".to_string(), outcome.as_str().to_string());
      attributes.insert(
        "agentToolKind".to_string(),
        self.invocation.kind.as_str().to_string(),
      );
      attributes.insert("agentToolName".to_string(), self.invocation.name.clone());
      if matches!(phase, AgentStepPhase::ToolCall | AgentStepPhase::Observation) {
        attributes.insert("toolCallId".to_string(), self.invocation.call_id.clone());
      }
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentStepOutcome {
  ApprovalPaused,
  Cancelled,
  Completed,
  Failed,
  Streaming,
}

impl AgentStepOutcome {
  pub(crate) fn from_items(
    items: &[TimelineItem],
    has_pending_approval: bool,
    has_pending_active_turn: bool,
  ) -> Self {
    if items.iter().any(is_cancelled_item) {
      return Self::Cancelled;
    }
    if has_pending_approval {
      return Self::ApprovalPaused;
    }
    if has_pending_active_turn {
      return Self::Streaming;
    }
    if items.iter().any(is_failure_item) {
      return Self::Failed;
    }
    Self::Completed
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::ApprovalPaused => "approvalPaused",
      Self::Cancelled => "cancelled",
      Self::Completed => "completed",
      Self::Failed => "failed",
      Self::Streaming => "streaming",
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentStepPhase {
  ApprovalPause,
  Final,
  Observation,
  Plan,
  ToolCall,
}

impl AgentStepPhase {
  fn as_str(self) -> &'static str {
    match self {
      Self::ApprovalPause => "approvalPause",
      Self::Final => "final",
      Self::Observation => "observation",
      Self::Plan => "plan",
      Self::ToolCall => "toolCall",
    }
  }
}

fn tool_kind_for_action(action: &PreparedTurnAction) -> AgentToolKind {
  match action {
    PreparedTurnAction::Write { .. } | PreparedTurnAction::ReadFile { .. } => {
      AgentToolKind::File
    }
    PreparedTurnAction::Shell { .. } => AgentToolKind::Shell,
    PreparedTurnAction::PluginCommand { snapshot } => {
      if snapshot.uses_connector() {
        AgentToolKind::Connector
      } else {
        AgentToolKind::Plugin
      }
    }
    PreparedTurnAction::PluginCommandRouteFailed { .. } => AgentToolKind::Plugin,
    PreparedTurnAction::Search { .. }
    | PreparedTurnAction::ListWorkspace
    | PreparedTurnAction::NoWorkspace => AgentToolKind::Workspace,
    PreparedTurnAction::WebSearch(_) | PreparedTurnAction::WebSearchCandidate(_) => {
      AgentToolKind::Web
    }
  }
}

fn tool_name_for_action(action: &PreparedTurnAction) -> String {
  match action {
    PreparedTurnAction::Write { .. } => "write_file".to_string(),
    PreparedTurnAction::Shell { .. } => "run_shell".to_string(),
    PreparedTurnAction::PluginCommand { snapshot } => snapshot.command_id().to_string(),
    PreparedTurnAction::PluginCommandRouteFailed { command_id, .. } => command_id.clone(),
    PreparedTurnAction::ReadFile { .. } => "read_file".to_string(),
    PreparedTurnAction::Search { .. } => "search_files".to_string(),
    PreparedTurnAction::WebSearch(_) | PreparedTurnAction::WebSearchCandidate(_) => {
      "web_search".to_string()
    }
    PreparedTurnAction::ListWorkspace => "list_workspace".to_string(),
    PreparedTurnAction::NoWorkspace => "answer_without_workspace".to_string(),
  }
}

fn item_phase(item: &TimelineItem) -> AgentStepPhase {
  match item.kind.as_str() {
    "plan" => AgentStepPhase::Plan,
    "toolStart" | "pluginCommand" => AgentStepPhase::ToolCall,
    "toolResult" | "pluginResult" | "diffArtifact" | "warning" => {
      AgentStepPhase::Observation
    }
    "approvalRequested" => AgentStepPhase::ApprovalPause,
    "assistantMessage" => AgentStepPhase::Final,
    _ => AgentStepPhase::Observation,
  }
}

fn is_cancelled_item(item: &TimelineItem) -> bool {
  item
    .attributes
    .as_ref()
    .and_then(|attributes| attributes.get("streamingStatus"))
    .is_some_and(|status| status == "cancelled")
}

fn is_failure_item(item: &TimelineItem) -> bool {
  if item.kind == "warning" {
    return true;
  }
  item.attributes.as_ref().is_some_and(|attributes| {
    attributes
      .get("pluginCommandStatus")
      .is_some_and(|status| status == "failed")
  })
}

impl AgentToolKind {
  fn as_str(&self) -> &'static str {
    match self {
      Self::Connector => "connector",
      Self::File => "file",
      Self::Plugin => "plugin",
      Self::Shell => "shell",
      Self::Workspace => "workspace",
      Self::Web => "web",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tags_plan_tool_observation_and_final_items() {
    let step = AgentStepRecord {
      step_id: "turn-1-step-1".to_string(),
      step_index: 1,
      invocation: AgentToolInvocation {
        call_id: "turn-1-step-1-tool-1".to_string(),
        kind: AgentToolKind::File,
        name: "read_file".to_string(),
      },
    };
    let mut items = vec![
      item("plan", "Plan"),
      item("toolStart", "read_file"),
      item("toolResult", "read_file result"),
      item("assistantMessage", "Assistant"),
    ];

    step.tag_items(&mut items, AgentStepOutcome::Streaming);

    assert_eq!(
      attribute(&items[0], "agentStepPhase"),
      Some("plan".to_string())
    );
    assert_eq!(
      attribute(&items[1], "toolCallId"),
      Some("turn-1-step-1-tool-1".to_string())
    );
    assert_eq!(
      attribute(&items[2], "agentStepPhase"),
      Some("observation".to_string())
    );
    assert_eq!(
      attribute(&items[3], "agentStepStatus"),
      Some("streaming".to_string())
    );
  }

  #[test]
  fn approval_paused_outcome_wins_over_streaming_absence() {
    let outcome =
      AgentStepOutcome::from_items(&[item("approvalRequested", "Approval")], true, false);

    assert_eq!(outcome, AgentStepOutcome::ApprovalPaused);
  }

  fn item(kind: &str, title: &str) -> TimelineItem {
    TimelineItem {
      kind: kind.to_string(),
      title: title.to_string(),
      content: String::new(),
      attributes: None,
    }
  }

  fn attribute(item: &TimelineItem, key: &str) -> Option<String> {
    item
      .attributes
      .as_ref()
      .and_then(|attributes| attributes.get(key))
      .cloned()
  }
}
