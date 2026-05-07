use pith_protocol::TimelineItem;

use crate::local_responses::build_plan_item;
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_no_workspace_turn(
  snapshot: &PreparedTurnSnapshot,
  items: &mut Vec<TimelineItem>,
) {
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
