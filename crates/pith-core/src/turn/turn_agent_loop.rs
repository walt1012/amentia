use std::collections::{HashMap, VecDeque};

use pith_protocol::TimelineItem;

use super::turn_agent_steps::{AgentStepOutcome, AgentStepRecord};
use crate::intent_inference::{WebSearchIntent, WriteIntent};
use crate::plugin_permissions::permission_is_granted;
use crate::request_state::PreparedTurnAction;

pub(crate) const LOOP_MAX_STEPS: usize = 3;
pub(crate) const LOOP_MAX_EXTENDED_STEPS: usize = 8;
const LOOP_MODE: &str = "dispatcherLoop";
const LOOP_SCHEMA: &str = "pith.agentLoop.v1";

pub(crate) struct AgentLoopCoordinator {
  loop_id: String,
  max_steps: usize,
  turn_id: String,
}

impl AgentLoopCoordinator {
  pub(crate) fn new(turn_id: &str, max_steps: usize) -> Self {
    Self {
      loop_id: format!("{turn_id}-loop-1"),
      max_steps: max_steps.clamp(1, LOOP_MAX_EXTENDED_STEPS),
      turn_id: turn_id.to_string(),
    }
  }

  pub(crate) fn max_steps(&self) -> usize {
    self.max_steps
  }

  pub(crate) fn allow_max_steps(&mut self, max_steps: usize) {
    self.max_steps = self
      .max_steps
      .max(max_steps.clamp(1, LOOP_MAX_EXTENDED_STEPS));
  }

  pub(crate) fn begin_step(
    &self,
    step_index: usize,
    action: &PreparedTurnAction,
  ) -> AgentStepRecord {
    AgentStepRecord::from_turn_action(&self.turn_id, step_index, action)
  }

  pub(crate) fn finish_step(
    &self,
    step: &AgentStepRecord,
    items: &mut [TimelineItem],
    observation: &AgentLoopObservation,
    step_count: usize,
    stop_reason: AgentLoopStopReason,
    has_pending_approval: bool,
    has_pending_active_turn: bool,
  ) {
    let outcome =
      AgentStepOutcome::from_items(items, has_pending_approval, has_pending_active_turn);
    step.tag_items(items, outcome);
    self.tag_loop_items(items, observation, step_count, stop_reason);
  }

  fn tag_loop_items(
    &self,
    items: &mut [TimelineItem],
    observation: &AgentLoopObservation,
    step_count: usize,
    stop_reason: AgentLoopStopReason,
  ) {
    for item in items {
      let attributes = item.attributes.get_or_insert_with(HashMap::new);
      attributes.insert("agentLoopId".to_string(), self.loop_id.clone());
      attributes.insert("agentLoopSchema".to_string(), LOOP_SCHEMA.to_string());
      attributes.insert("agentLoopMode".to_string(), LOOP_MODE.to_string());
      attributes.insert("agentLoopMaxSteps".to_string(), self.max_steps.to_string());
      attributes.insert("agentLoopStepCount".to_string(), step_count.to_string());
      attributes.insert(
        "agentLoopBudgetRemaining".to_string(),
        self.max_steps.saturating_sub(step_count).to_string(),
      );
      attributes.insert(
        "agentLoopStopReason".to_string(),
        stop_reason.as_str().to_string(),
      );
      attributes.insert(
        "agentLoopObservationCount".to_string(),
        observation.count().to_string(),
      );
      attributes.insert(
        "agentLoopSuccessfulObservationCount".to_string(),
        observation.success_count().to_string(),
      );
      attributes.insert(
        "agentLoopFailureCount".to_string(),
        observation.failure_count().to_string(),
      );
      observation.insert_last_observation_attributes(attributes);
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentLoopObservation {
  observation_count: usize,
  successful_observation_count: usize,
  failure_count: usize,
  last_successful_observation: Option<AgentLoopLastObservation>,
  planned_next_action: Option<AgentLoopPlannedAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentLoopLastObservation {
  kind: String,
  title: String,
  tool: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentLoopPlannedAction {
  ListWorkspace,
  ReadFile {
    relative_path: String,
  },
  Search {
    query: String,
  },
  Shell {
    command: String,
  },
  WebSearch {
    query: String,
    routing_reason: &'static str,
  },
  WriteFile {
    relative_path: String,
    content: String,
  },
  PluginCommand {
    command_id: String,
    input: Option<String>,
  },
}

impl AgentLoopObservation {
  pub(crate) fn from_items(items: &[TimelineItem]) -> Self {
    let mut observation_count = 0;
    let mut successful_observation_count = 0;
    let mut failure_count = 0;
    let mut last_successful_observation = None;
    let mut planned_next_action = None;

    for item in items {
      if is_observation_item(item) {
        observation_count += 1;
      }
      if is_failure_item(item) {
        failure_count += 1;
      } else if is_observation_item(item) {
        successful_observation_count += 1;
        last_successful_observation = Some(last_observation_from_item(item));
        planned_next_action = planned_action_from_item(item).or(planned_next_action);
      }
    }

    Self {
      observation_count,
      successful_observation_count,
      failure_count,
      last_successful_observation,
      planned_next_action,
    }
  }

  pub(crate) fn can_inform_next_action(&self) -> bool {
    self.observation_count > 0 && self.failure_count == 0
  }

  fn count(&self) -> usize {
    self.observation_count
  }

  fn success_count(&self) -> usize {
    self.successful_observation_count
  }

  fn failure_count(&self) -> usize {
    self.failure_count
  }

  fn has_failure(&self) -> bool {
    self.failure_count > 0
  }

  fn insert_last_observation_attributes(&self, attributes: &mut HashMap<String, String>) {
    let Some(last_observation) = self.last_successful_observation.as_ref() else {
      return;
    };
    attributes.insert(
      "agentLoopLastObservationKind".to_string(),
      last_observation.kind.clone(),
    );
    attributes.insert(
      "agentLoopLastObservationTitle".to_string(),
      last_observation.title.clone(),
    );
    if let Some(tool) = last_observation.tool.as_ref() {
      attributes.insert("agentLoopLastObservationTool".to_string(), tool.clone());
    }
  }

  #[cfg(test)]
  pub(crate) fn planned_next_action(&self) -> Option<PreparedTurnAction> {
    self.planned_next_action_with_approvals(&HashMap::new(), &mut VecDeque::new())
  }

  pub(crate) fn planned_action(&self) -> Option<&AgentLoopPlannedAction> {
    self.planned_next_action.as_ref()
  }

  pub(crate) fn planned_next_action_with_approvals(
    &self,
    permission_sources: &HashMap<String, Vec<String>>,
    reserved_approval_ids: &mut VecDeque<String>,
  ) -> Option<PreparedTurnAction> {
    let action = self.planned_next_action.as_ref()?;
    match action {
      AgentLoopPlannedAction::ListWorkspace => Some(PreparedTurnAction::ListWorkspace),
      AgentLoopPlannedAction::ReadFile { relative_path } => Some(PreparedTurnAction::ReadFile {
        relative_path: relative_path.clone(),
      }),
      AgentLoopPlannedAction::Search { query } => Some(PreparedTurnAction::Search {
        query: query.clone(),
      }),
      AgentLoopPlannedAction::Shell { command } => Some(PreparedTurnAction::Shell {
        command: command.clone(),
        approval_id: next_approval_id(permission_sources, reserved_approval_ids, "shell.exec"),
      }),
      AgentLoopPlannedAction::WebSearch {
        query,
        routing_reason,
      } => Some(PreparedTurnAction::WebSearch(WebSearchIntent {
        query: query.clone(),
        routing_reason,
      })),
      AgentLoopPlannedAction::WriteFile {
        relative_path,
        content,
      } => Some(PreparedTurnAction::Write {
        intent: WriteIntent {
          relative_path: relative_path.clone(),
          content: content.clone(),
        },
        approval_id: next_approval_id(permission_sources, reserved_approval_ids, "file.write"),
      }),
      AgentLoopPlannedAction::PluginCommand { .. } => None,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentLoopStopReason {
  ApprovalPaused,
  Cancelled,
  Completed,
  Failed,
  Streaming,
  StepBudgetExhausted,
}

impl AgentLoopStopReason {
  pub(crate) fn from_step_state(
    observation: &AgentLoopObservation,
    cancellation_is_cancelled: bool,
    has_pending_approval: bool,
    has_pending_active_turn: bool,
  ) -> Self {
    if cancellation_is_cancelled {
      Self::Cancelled
    } else if has_pending_approval {
      Self::ApprovalPaused
    } else if has_pending_active_turn {
      Self::Streaming
    } else if observation.has_failure() {
      Self::Failed
    } else {
      Self::Completed
    }
  }

  fn as_str(self) -> &'static str {
    match self {
      Self::ApprovalPaused => "approvalPaused",
      Self::Cancelled => "cancelled",
      Self::Completed => "completed",
      Self::Failed => "failed",
      Self::Streaming => "streaming",
      Self::StepBudgetExhausted => "stepBudgetExhausted",
    }
  }

  pub(crate) fn can_continue(self) -> bool {
    matches!(self, Self::Completed)
  }
}

fn is_observation_item(item: &TimelineItem) -> bool {
  matches!(
    item.kind.as_str(),
    "toolResult" | "pluginResult" | "diffArtifact" | "warning"
  )
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

fn last_observation_from_item(item: &TimelineItem) -> AgentLoopLastObservation {
  AgentLoopLastObservation {
    kind: item.kind.clone(),
    title: item.title.clone(),
    tool: item.attributes.as_ref().and_then(observation_tool_name),
  }
}

fn observation_tool_name(attributes: &HashMap<String, String>) -> Option<String> {
  attributes
    .get("tool")
    .or_else(|| attributes.get("commandId"))
    .cloned()
}

fn planned_action_from_item(item: &TimelineItem) -> Option<AgentLoopPlannedAction> {
  let attributes = item.attributes.as_ref()?;
  match attributes.get("nextAction").map(String::as_str) {
    Some("list_directory" | "list_workspace") => Some(AgentLoopPlannedAction::ListWorkspace),
    Some("read_file") => Some(AgentLoopPlannedAction::ReadFile {
      relative_path: attributes.get("nextRelativePath")?.clone(),
    }),
    Some("search" | "search_files") => Some(AgentLoopPlannedAction::Search {
      query: attributes.get("nextQuery")?.clone(),
    }),
    Some("run_shell" | "shell") => Some(AgentLoopPlannedAction::Shell {
      command: attributes.get("nextCommand")?.clone(),
    }),
    Some("web_search") => Some(AgentLoopPlannedAction::WebSearch {
      query: attributes.get("nextQuery")?.clone(),
      routing_reason: normalized_next_routing_reason(attributes.get("nextRoutingReason")),
    }),
    Some("write_file") => Some(AgentLoopPlannedAction::WriteFile {
      relative_path: attributes.get("nextRelativePath")?.clone(),
      content: attributes.get("nextContent")?.clone(),
    }),
    Some("plugin_command") => Some(AgentLoopPlannedAction::PluginCommand {
      command_id: attributes.get("nextCommandId")?.clone(),
      input: attributes.get("nextCommandInput").cloned(),
    }),
    _ => None,
  }
}

fn next_approval_id(
  permission_sources: &HashMap<String, Vec<String>>,
  reserved_approval_ids: &mut VecDeque<String>,
  permission: &str,
) -> Option<String> {
  if permission_is_granted(permission_sources, permission) {
    reserved_approval_ids.pop_front()
  } else {
    None
  }
}

fn normalized_next_routing_reason(value: Option<&String>) -> &'static str {
  match value.map(String::as_str) {
    Some("explicitWebSearchRequest") => "explicitWebSearchRequest",
    Some("freshPublicInformation") => "freshPublicInformation",
    Some("modelToolPlanning") => "modelToolPlanning",
    _ => "observationNextAction",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dispatcher_loop_tags_loop_budget_and_stop_reason() {
    let coordinator = AgentLoopCoordinator::new("turn-1", LOOP_MAX_STEPS);
    let action = PreparedTurnAction::ListWorkspace;
    let step = coordinator.begin_step(1, &action);
    let mut items = vec![TimelineItem {
      kind: "plan".to_string(),
      title: "Plan".to_string(),
      content: "List workspace.".to_string(),
      attributes: None,
    }];

    coordinator.finish_step(
      &step,
      &mut items,
      &AgentLoopObservation::from_items(&[]),
      1,
      AgentLoopStopReason::Completed,
      false,
      false,
    );

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("agentLoopSchema").map(String::as_str),
      Some("pith.agentLoop.v1")
    );
    assert_eq!(
      attributes.get("agentLoopMode").map(String::as_str),
      Some("dispatcherLoop")
    );
    assert_eq!(
      attributes.get("agentLoopMaxSteps").map(String::as_str),
      Some("3")
    );
    assert_eq!(
      attributes.get("agentStepId").map(String::as_str),
      Some("turn-1-step-1")
    );
    assert_eq!(
      attributes.get("agentLoopStepCount").map(String::as_str),
      Some("1")
    );
    assert_eq!(
      attributes
        .get("agentLoopBudgetRemaining")
        .map(String::as_str),
      Some("2")
    );
    assert_eq!(
      attributes.get("agentLoopStopReason").map(String::as_str),
      Some("completed")
    );
    assert_eq!(
      attributes
        .get("agentLoopObservationCount")
        .map(String::as_str),
      Some("0")
    );
  }

  #[test]
  fn stop_reason_marks_warning_observations_as_failed() {
    let items = vec![TimelineItem {
      kind: "warning".to_string(),
      title: "Tool Failed".to_string(),
      content: String::new(),
      attributes: None,
    }];

    let reason = AgentLoopStopReason::from_step_state(
      &AgentLoopObservation::from_items(&items),
      false,
      false,
      false,
    );

    assert_eq!(reason, AgentLoopStopReason::Failed);
  }

  #[test]
  fn observation_summary_tracks_success_failure_and_last_tool() {
    let observation = AgentLoopObservation::from_items(&[
      TimelineItem {
        kind: "toolResult".to_string(),
        title: "read_file result".to_string(),
        content: String::new(),
        attributes: Some(HashMap::from([(
          "tool".to_string(),
          "read_file".to_string(),
        )])),
      },
      TimelineItem {
        kind: "warning".to_string(),
        title: "Tool Failed".to_string(),
        content: String::new(),
        attributes: None,
      },
    ]);
    let mut attributes = HashMap::new();

    observation.insert_last_observation_attributes(&mut attributes);

    assert_eq!(observation.count(), 2);
    assert_eq!(observation.success_count(), 1);
    assert_eq!(observation.failure_count(), 1);
    assert_eq!(
      attributes
        .get("agentLoopLastObservationTool")
        .map(String::as_str),
      Some("read_file")
    );
  }

  #[test]
  fn observation_summary_recovers_planned_next_action() {
    let observation = AgentLoopObservation::from_items(&[TimelineItem {
      kind: "toolResult".to_string(),
      title: "search_files result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("tool".to_string(), "search_files".to_string()),
        ("nextAction".to_string(), "read_file".to_string()),
        ("nextRelativePath".to_string(), "README.md".to_string()),
      ])),
    }]);

    let next_action = observation.planned_next_action().expect("next action");

    match next_action {
      PreparedTurnAction::ReadFile { relative_path } => assert_eq!(relative_path, "README.md"),
      other => panic!("unexpected next action: {other:?}"),
    }
  }

  #[test]
  fn observation_summary_recovers_search_next_action() {
    let observation = AgentLoopObservation::from_items(&[TimelineItem {
      kind: "pluginResult".to_string(),
      title: "Plugin Result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("tool".to_string(), "workspace-notes".to_string()),
        ("nextAction".to_string(), "search_files".to_string()),
        ("nextQuery".to_string(), "model catalog".to_string()),
      ])),
    }]);

    let next_action = observation.planned_next_action().expect("next action");

    match next_action {
      PreparedTurnAction::Search { query } => assert_eq!(query, "model catalog"),
      other => panic!("unexpected next action: {other:?}"),
    }
  }

  #[test]
  fn observation_summary_recovers_web_search_next_action() {
    let observation = AgentLoopObservation::from_items(&[TimelineItem {
      kind: "pluginResult".to_string(),
      title: "Plugin Result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("tool".to_string(), "notion.prepare-page-draft".to_string()),
        ("nextAction".to_string(), "web_search".to_string()),
        (
          "nextQuery".to_string(),
          "latest small GGUF model".to_string(),
        ),
        (
          "nextRoutingReason".to_string(),
          "freshPublicInformation".to_string(),
        ),
      ])),
    }]);

    let next_action = observation.planned_next_action().expect("next action");

    match next_action {
      PreparedTurnAction::WebSearch(intent) => {
        assert_eq!(intent.query, "latest small GGUF model");
        assert_eq!(intent.routing_reason, "freshPublicInformation");
      }
      other => panic!("unexpected next action: {other:?}"),
    }
  }

  #[test]
  fn observation_summary_recovers_shell_next_action_with_approval() {
    let observation = AgentLoopObservation::from_items(&[TimelineItem {
      kind: "pluginResult".to_string(),
      title: "Plugin Result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("tool".to_string(), "review-assistant".to_string()),
        ("nextAction".to_string(), "run_shell".to_string()),
        ("nextCommand".to_string(), "git status --short".to_string()),
      ])),
    }]);
    let mut approval_ids = ["approval-1".to_string()].into_iter().collect();
    let permissions = HashMap::from([("shell.exec".to_string(), vec!["Test".to_string()])]);

    let next_action = observation
      .planned_next_action_with_approvals(&permissions, &mut approval_ids)
      .expect("next action");

    match next_action {
      PreparedTurnAction::Shell {
        command,
        approval_id,
      } => {
        assert_eq!(command, "git status --short");
        assert_eq!(approval_id.as_deref(), Some("approval-1"));
      }
      other => panic!("unexpected next action: {other:?}"),
    }
  }

  #[test]
  fn observation_summary_recovers_write_next_action_without_bypassing_approval() {
    let observation = AgentLoopObservation::from_items(&[TimelineItem {
      kind: "pluginResult".to_string(),
      title: "Plugin Result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("tool".to_string(), "workspace-notes".to_string()),
        ("nextAction".to_string(), "write_file".to_string()),
        ("nextRelativePath".to_string(), "notes/today.md".to_string()),
        (
          "nextContent".to_string(),
          "Ship the cowork loop.".to_string(),
        ),
      ])),
    }]);
    let mut approval_ids = ["approval-2".to_string()].into_iter().collect();
    let permissions = HashMap::from([("file.write".to_string(), vec!["Test".to_string()])]);

    let next_action = observation
      .planned_next_action_with_approvals(&permissions, &mut approval_ids)
      .expect("next action");

    match next_action {
      PreparedTurnAction::Write {
        intent,
        approval_id,
      } => {
        assert_eq!(intent.relative_path, "notes/today.md");
        assert_eq!(intent.content, "Ship the cowork loop.");
        assert_eq!(approval_id.as_deref(), Some("approval-2"));
      }
      other => panic!("unexpected next action: {other:?}"),
    }
  }

  #[test]
  fn observation_summary_recovers_plugin_command_next_action() {
    let observation = AgentLoopObservation::from_items(&[TimelineItem {
      kind: "pluginResult".to_string(),
      title: "Plugin Result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("tool".to_string(), "source-plugin.run".to_string()),
        ("nextAction".to_string(), "plugin_command".to_string()),
        (
          "nextCommandId".to_string(),
          "target-plugin::target-plugin.run".to_string(),
        ),
        ("nextCommandInput".to_string(), "draft handoff".to_string()),
      ])),
    }]);

    let next_action = observation.planned_action().expect("next action");

    match next_action {
      AgentLoopPlannedAction::PluginCommand { command_id, input } => {
        assert_eq!(command_id, "target-plugin::target-plugin.run");
        assert_eq!(input.as_deref(), Some("draft handoff"));
      }
      other => panic!("unexpected next action: {other:?}"),
    }
  }
}
