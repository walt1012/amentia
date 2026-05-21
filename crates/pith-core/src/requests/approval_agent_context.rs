use std::collections::HashMap;

use pith_protocol::TimelineItem;

use crate::approval_types::PendingApproval;

const AGENT_CONTEXT_KEYS: &[&str] = &[
  "agentLoopId",
  "agentLoopSchema",
  "agentLoopMode",
  "agentLoopMaxSteps",
  "agentLoopStepCount",
  "agentStepId",
  "agentStepIndex",
  "agentToolSchema",
  "agentToolKind",
  "agentToolName",
  "toolCallId",
];

#[derive(Debug, Clone, Default)]
pub(crate) struct ApprovalAgentContext {
  attributes: HashMap<String, String>,
}

impl ApprovalAgentContext {
  pub(crate) fn from_thread_items(approval: &PendingApproval, items: &[TimelineItem]) -> Self {
    let Some(attributes) = items
      .iter()
      .rev()
      .find(|item| is_matching_approval_request(item, approval))
      .and_then(|item| item.attributes.as_ref())
    else {
      return Self::default();
    };

    let mut captured = HashMap::new();
    for key in AGENT_CONTEXT_KEYS {
      if let Some(value) = attributes.get(*key) {
        captured.insert((*key).to_string(), value.clone());
      }
    }
    capture_step_tool_call_id(&mut captured, items);
    Self {
      attributes: captured,
    }
  }

  pub(crate) fn tag_items(&self, items: &mut [TimelineItem]) {
    if self.attributes.is_empty() {
      return;
    }

    let step_status = resumed_step_status(items);
    for item in items {
      let kind = item.kind.clone();
      let attributes = item.attributes.get_or_insert_with(HashMap::new);
      for (key, value) in &self.attributes {
        attributes.insert(key.clone(), value.clone());
      }
      attributes.insert("agentStepResume".to_string(), "true".to_string());
      attributes.insert(
        "agentStepPhase".to_string(),
        resumed_phase(&kind).to_string(),
      );
      attributes.insert("agentStepStatus".to_string(), step_status.to_string());
      if let Some(status) = resumed_tool_call_status(&kind) {
        attributes.insert("toolCallStatus".to_string(), status.to_string());
      }
    }
  }
}

fn capture_step_tool_call_id(captured: &mut HashMap<String, String>, items: &[TimelineItem]) {
  if captured.contains_key("toolCallId") {
    return;
  }
  let Some(step_id) = captured.get("agentStepId") else {
    return;
  };
  let step_id = step_id.clone();
  let tool_call_id = items
    .iter()
    .rev()
    .find_map(|item| {
      let attributes = item.attributes.as_ref()?;
      if attributes.get("agentStepId")? != &step_id {
        return None;
      }
      attributes.get("toolCallId").cloned()
    })
    .unwrap_or_else(|| format!("{step_id}-tool-1"));
  captured.insert("toolCallId".to_string(), tool_call_id);
}

fn is_matching_approval_request(item: &TimelineItem, approval: &PendingApproval) -> bool {
  if item.kind != "approvalRequested" {
    return false;
  }
  item.attributes.as_ref().is_some_and(|attributes| {
    attributes
      .get("approvalId")
      .is_some_and(|approval_id| approval_id == &approval.id)
  })
}

fn resumed_phase(kind: &str) -> &'static str {
  match kind {
    "approvalResolved" => "approvalResume",
    "toolStart" | "pluginCommand" => "toolCall",
    "assistantMessage" => "final",
    _ => "observation",
  }
}

fn resumed_step_status(items: &[TimelineItem]) -> &'static str {
  if items.iter().any(|item| item.kind == "warning") {
    return "failed";
  }
  if items.iter().any(|item| {
    item.kind == "approvalResolved"
      && item.attributes.as_ref().is_some_and(|attributes| {
        attributes
          .get("decision")
          .is_some_and(|decision| decision.as_str() == "denied")
      })
  }) {
    return "denied";
  }
  "completed"
}

fn resumed_tool_call_status(kind: &str) -> Option<&'static str> {
  match kind {
    "toolStart" | "pluginCommand" => Some("started"),
    "toolResult" | "pluginResult" => Some("completed"),
    "warning" => Some("failed"),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn captures_matching_approval_agent_context() {
    let approval = approval("approval-2");
    let context = ApprovalAgentContext::from_thread_items(
      &approval,
      &[
        approval_item("approval-1", "old-step", "old-loop"),
        tool_item("step-1", "call-1"),
        approval_item("approval-2", "step-1", "loop-1"),
      ],
    );
    let mut items = vec![TimelineItem {
      kind: "approvalResolved".to_string(),
      title: "Approval Granted".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([(
        "decision".to_string(),
        "approved".to_string(),
      )])),
    }];

    context.tag_items(&mut items);

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("agentStepId").map(String::as_str),
      Some("step-1")
    );
    assert_eq!(
      attributes.get("agentLoopId").map(String::as_str),
      Some("loop-1")
    );
    assert_eq!(
      attributes.get("agentStepPhase").map(String::as_str),
      Some("approvalResume")
    );
    assert_eq!(
      attributes.get("agentStepResume").map(String::as_str),
      Some("true")
    );
    assert_eq!(
      attributes.get("toolCallId").map(String::as_str),
      Some("call-1")
    );
  }

  #[test]
  fn denied_approval_marks_resumed_step_denied() {
    let context = ApprovalAgentContext {
      attributes: HashMap::from([("agentStepId".to_string(), "step-1".to_string())]),
    };
    let mut items = vec![TimelineItem {
      kind: "approvalResolved".to_string(),
      title: "Approval Denied".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([(
        "decision".to_string(),
        "denied".to_string(),
      )])),
    }];

    context.tag_items(&mut items);

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("agentStepStatus").map(String::as_str),
      Some("denied")
    );
  }

  #[test]
  fn synthesizes_tool_call_id_when_pause_has_no_prior_tool_item() {
    let approval = approval("approval-1");
    let context = ApprovalAgentContext::from_thread_items(
      &approval,
      &[approval_item("approval-1", "step-1", "loop-1")],
    );
    let mut items = vec![TimelineItem {
      kind: "toolStart".to_string(),
      title: "run_shell".to_string(),
      content: String::new(),
      attributes: None,
    }];

    context.tag_items(&mut items);

    let attributes = items[0].attributes.as_ref().expect("attributes");
    assert_eq!(
      attributes.get("toolCallId").map(String::as_str),
      Some("step-1-tool-1")
    );
  }

  fn approval(id: &str) -> PendingApproval {
    PendingApproval {
      id: id.to_string(),
      thread_id: "thread-1".to_string(),
      action: "write_file".to_string(),
      title: "Write README.md".to_string(),
      relative_path: "README.md".to_string(),
      content: Some("content".to_string()),
      command: None,
    }
  }

  fn approval_item(id: &str, step_id: &str, loop_id: &str) -> TimelineItem {
    TimelineItem {
      kind: "approvalRequested".to_string(),
      title: "Approval Requested".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("approvalId".to_string(), id.to_string()),
        ("agentStepId".to_string(), step_id.to_string()),
        ("agentLoopId".to_string(), loop_id.to_string()),
        (
          "agentToolSchema".to_string(),
          "pith.localTool.v1".to_string(),
        ),
      ])),
    }
  }

  fn tool_item(step_id: &str, call_id: &str) -> TimelineItem {
    TimelineItem {
      kind: "toolResult".to_string(),
      title: "Tool Result".to_string(),
      content: String::new(),
      attributes: Some(HashMap::from([
        ("agentStepId".to_string(), step_id.to_string()),
        ("toolCallId".to_string(), call_id.to_string()),
      ])),
    }
  }
}
