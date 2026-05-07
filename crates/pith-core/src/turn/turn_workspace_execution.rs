use std::collections::HashMap;
use std::path::Path;

use pith_protocol::{TimelineItem, WorkspaceSummary};
use pith_tools::{list_directory, read_file, search_files};

use crate::active_turns::{start_streaming_assistant_turn, ActiveTurn};
use crate::local_responses::{
  build_plan_item, format_directory_result, format_file_result, format_search_result,
  summarize_directory_result, summarize_file_result, summarize_search_result,
};
use crate::plugin_permissions::{build_permission_denied_items, permission_is_granted};
use crate::request_state::PreparedTurnSnapshot;

pub(super) fn execute_read_turn(
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

pub(super) fn execute_search_turn(
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

pub(super) fn execute_list_turn(
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
