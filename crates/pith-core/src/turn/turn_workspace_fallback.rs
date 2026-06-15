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
    "Open a project before running filesystem tools.".to_string(),
    Some(&snapshot.cancellation),
  ));
  if snapshot.cancellation.is_cancelled() {
    items.extend(crate::turn_streaming::build_turn_cancelled_items(
      &snapshot.turn_id,
    ));
    return;
  }
  items.push(TimelineItem {
    kind: "warning".to_string(),
    title: "Project Required".to_string(),
    content: "Open a project before asking Pith to inspect files.".to_string(),
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
