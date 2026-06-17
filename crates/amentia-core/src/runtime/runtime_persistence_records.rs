use amentia_storage::{StoredApprovalRecord, StoredThreadRecord};

use crate::approval_types::PendingApproval;
use crate::thread_state::StoredThread;

pub(super) fn stored_approval_record(approval: PendingApproval) -> StoredApprovalRecord {
  StoredApprovalRecord {
    id: approval.id,
    thread_id: approval.thread_id,
    action: approval.action,
    title: approval.title,
    relative_path: approval.relative_path,
    content: approval.content,
    command: approval.command,
  }
}

pub(super) fn stored_thread_record(thread: &StoredThread) -> StoredThreadRecord {
  StoredThreadRecord {
    summary: thread.summary().clone(),
    turn_count: thread.turn_count(),
    items: thread.items().to_vec(),
    workspace: thread.workspace_cloned(),
  }
}

pub(super) fn stored_thread(thread: StoredThreadRecord) -> StoredThread {
  StoredThread::new(
    thread.summary,
    thread.turn_count,
    thread.items,
    thread.workspace,
  )
}

pub(super) fn pending_approval(approval: StoredApprovalRecord) -> PendingApproval {
  PendingApproval {
    id: approval.id,
    thread_id: approval.thread_id,
    action: approval.action,
    title: approval.title,
    relative_path: approval.relative_path,
    content: approval.content,
    command: approval.command,
  }
}
