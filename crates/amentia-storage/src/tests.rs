use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use amentia_memory::MemoryNote;
use amentia_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use rusqlite::Connection;

use super::*;

fn create_temp_directory(label: &str) -> PathBuf {
  let unique = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_nanos();
  let path = env::temp_dir().join(format!("amentia-storage-{label}-{unique}"));
  fs::create_dir_all(&path).expect("create temp directory");
  path
}

fn workspace_change(id: &str, thread_id: &str) -> StoredWorkspaceChangeRecord {
  StoredWorkspaceChangeRecord {
    id: id.to_string(),
    thread_id: thread_id.to_string(),
    approval_id: Some(id.to_string()),
    workspace_root_path: "/tmp/amentia-workspace".to_string(),
    relative_path: "notes.txt".to_string(),
    action: "write_file".to_string(),
    previous_content: Some(b"before".to_vec()),
    next_content: b"after".to_vec(),
    reverted_at: None,
  }
}

#[test]
fn sqlite_store_round_trips_threads_and_workspace() {
  let root = create_temp_directory("sqlite-roundtrip");
  let store = RuntimeStore::new(root.join("amentia.db"));

  store
    .save_workspace(&WorkspaceSummary {
      root_path: "/tmp/amentia".to_string(),
      display_name: "amentia".to_string(),
    })
    .expect("save workspace");
  store
    .save_threads(&[StoredThreadRecord {
      summary: ThreadSummary {
        id: "thread-1".to_string(),
        title: "Thread".to_string(),
        status: "ready".to_string(),
        workspace: Some(WorkspaceSummary {
          root_path: "/tmp/amentia".to_string(),
          display_name: "amentia".to_string(),
        }),
      },
      turn_count: 3,
      items: vec![TimelineItem {
        kind: "system".to_string(),
        title: "Thread Ready".to_string(),
        content: "Ready".to_string(),
        attributes: None,
      }],
      workspace: Some(WorkspaceSummary {
        root_path: "/tmp/amentia".to_string(),
        display_name: "amentia".to_string(),
      }),
    }])
    .expect("save threads");

  let workspace = store.load_workspace().expect("load workspace");
  let threads = store.load_threads().expect("load threads");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(workspace.expect("workspace").display_name, "amentia");
  assert_eq!(threads.len(), 1);
  assert_eq!(threads[0].turn_count, 3);
  assert_eq!(threads[0].summary.id, "thread-1");
  assert_eq!(
    threads[0]
      .workspace
      .as_ref()
      .expect("thread workspace")
      .display_name,
    "amentia"
  );
}

#[test]
fn sqlite_store_round_trips_pending_approvals_and_resolution_audit() {
  let root = create_temp_directory("approval-roundtrip");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let approval = StoredApprovalRecord {
    id: "approval-4".to_string(),
    thread_id: "thread-2".to_string(),
    action: "write_file".to_string(),
    title: "Write docs/output.txt".to_string(),
    relative_path: "docs/output.txt".to_string(),
    content: Some("hello".to_string()),
    command: None,
  };

  store
    .save_pending_approvals(std::slice::from_ref(&approval))
    .expect("save pending approvals");
  let pending = store
    .load_pending_approvals()
    .expect("load pending approvals");
  assert_eq!(pending.len(), 1);
  assert_eq!(pending[0].id, "approval-4");

  store
    .resolve_approval(&approval, "approved")
    .expect("resolve approval");
  let reloaded_pending = store
    .load_pending_approvals()
    .expect("reload pending approvals");
  let next_sequence = store
    .next_approval_sequence()
    .expect("next approval sequence");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert!(reloaded_pending.is_empty());
  assert_eq!(next_sequence, 5);
}

#[test]
fn sqlite_store_deletes_all_approvals_for_thread() {
  let root = create_temp_directory("approval-delete-thread");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let approval = StoredApprovalRecord {
    id: "approval-4".to_string(),
    thread_id: "thread-2".to_string(),
    action: "write_file".to_string(),
    title: "Write docs/output.txt".to_string(),
    relative_path: "docs/output.txt".to_string(),
    content: Some("hello".to_string()),
    command: None,
  };

  store
    .save_pending_approvals(std::slice::from_ref(&approval))
    .expect("save pending approval");
  store
    .resolve_approval(&approval, "approved")
    .expect("resolve approval");

  let deleted = store
    .delete_approvals_for_thread("thread-2")
    .expect("delete approvals");
  let pending = store
    .load_pending_approvals()
    .expect("load pending approvals");
  let next_sequence = store
    .next_approval_sequence()
    .expect("next approval sequence");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(deleted, 1);
  assert!(pending.is_empty());
  assert_eq!(next_sequence, 1);
}

#[test]
fn sqlite_store_round_trips_workspace_change_ledger() {
  let root = create_temp_directory("workspace-change-ledger");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let change = StoredWorkspaceChangeRecord {
    id: "approval-9".to_string(),
    thread_id: "thread-7".to_string(),
    approval_id: Some("approval-9".to_string()),
    workspace_root_path: "/tmp/amentia-workspace".to_string(),
    relative_path: "notes.txt".to_string(),
    action: "write_file".to_string(),
    previous_content: Some(b"before".to_vec()),
    next_content: b"after".to_vec(),
    reverted_at: None,
  };

  store
    .save_workspace_change(&change)
    .expect("save workspace change");
  store
    .mark_workspace_change_reverted("approval-9")
    .expect("mark workspace change reverted");
  let changes = store
    .load_workspace_changes_for_thread("thread-7")
    .expect("load workspace changes");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(changes.len(), 1);
  assert_eq!(changes[0].id, change.id);
  assert!(changes[0].reverted_at.is_some());
}

#[test]
fn sqlite_store_deletes_workspace_changes_for_thread() {
  let root = create_temp_directory("workspace-change-delete-thread");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let mut first_change = workspace_change("change-1", "thread-1");
  let second_change = workspace_change("change-2", "thread-2");
  first_change.relative_path = "first.txt".to_string();

  store
    .save_workspace_change(&first_change)
    .expect("save first workspace change");
  store
    .save_workspace_change(&second_change)
    .expect("save second workspace change");
  let deleted = store
    .delete_workspace_changes_for_thread("thread-1")
    .expect("delete workspace changes");
  let first_thread_changes = store
    .load_workspace_changes_for_thread("thread-1")
    .expect("load first thread changes");
  let second_thread_changes = store
    .load_workspace_changes_for_thread("thread-2")
    .expect("load second thread changes");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(deleted, 1);
  assert!(first_thread_changes.is_empty());
  assert_eq!(second_thread_changes.len(), 1);
  assert_eq!(second_thread_changes[0].id, "change-2");
}

#[test]
fn sqlite_store_migrates_existing_version_one_database() {
  let root = create_temp_directory("migrate-v1");
  let database_path = root.join("amentia.db");
  let connection = Connection::open(&database_path).expect("open seed database");
  connection
    .execute_batch(
      "
        CREATE TABLE workspace_state (
          id INTEGER PRIMARY KEY CHECK (id = 1),
          root_path TEXT NOT NULL,
          display_name TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        CREATE TABLE threads (
          id TEXT PRIMARY KEY,
          title TEXT NOT NULL,
          status TEXT NOT NULL,
          turn_count INTEGER NOT NULL,
          items_json TEXT NOT NULL,
          updated_at INTEGER NOT NULL
        );
        INSERT INTO threads (id, title, status, turn_count, items_json, updated_at)
        VALUES ('thread-old', 'Old Thread', 'ready', 1, '[]', 1);
        PRAGMA user_version = 1;
        ",
    )
    .expect("seed version one schema");
  drop(connection);

  let store = RuntimeStore::new(database_path.clone());
  let threads = store.load_threads().expect("load migrated threads");
  let pending_approvals = store
    .load_pending_approvals()
    .expect("load migrated approvals");
  let connection = Connection::open(&database_path).expect("reopen migrated database");
  let migration_versions: Vec<i64> = connection
    .prepare("SELECT version FROM schema_migrations ORDER BY version ASC")
    .expect("prepare schema migrations query")
    .query_map([], |row| row.get(0))
    .expect("query schema migrations")
    .collect::<std::result::Result<Vec<_>, _>>()
    .expect("collect schema migrations");
  let approval_indexes: Vec<String> = connection
    .prepare(
      "SELECT name
         FROM sqlite_master
         WHERE type = 'index' AND tbl_name = 'approvals'
         ORDER BY name ASC",
    )
    .expect("prepare approvals index query")
    .query_map([], |row| row.get(0))
    .expect("query approvals indexes")
    .collect::<std::result::Result<Vec<_>, _>>()
    .expect("collect approvals indexes");
  let credential_columns: Vec<String> = connection
    .prepare("PRAGMA table_info(plugin_connector_credentials)")
    .expect("prepare connector credential column query")
    .query_map([], |row| row.get(1))
    .expect("query connector credential columns")
    .collect::<std::result::Result<Vec<_>, _>>()
    .expect("collect connector credential columns");
  let workspace_change_table_count: i64 = connection
    .query_row(
      "SELECT COUNT(*)
       FROM sqlite_master
       WHERE type = 'table' AND name = 'workspace_changes'",
      [],
      |row| row.get(0),
    )
    .expect("query workspace change table");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(threads.len(), 1);
  assert_eq!(threads[0].summary.id, "thread-old");
  assert!(pending_approvals.is_empty());
  assert_eq!(migration_versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  assert!(approval_indexes.contains(&"idx_approvals_requested_at".to_string()));
  assert!(!credential_columns.contains(&"credential_secret".to_string()));
  assert_eq!(workspace_change_table_count, 1);
}

#[test]
fn sqlite_store_round_trips_memory_notes() {
  let root = create_temp_directory("memory-roundtrip");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let note = MemoryNote {
    id: "memory-7".to_string(),
    title: "Opened workspace amentia".to_string(),
    body: "Amentia opened the workspace at /tmp/amentia.".to_string(),
    scope: "amentia".to_string(),
    source: "workspace".to_string(),
    created_at: 7,
    tags: vec!["workspace".to_string(), "session".to_string()],
  };

  store.save_memory_note(&note).expect("save memory note");
  let notes = store.load_memory_notes(10).expect("load memory notes");
  let next_sequence = store.next_memory_sequence().expect("next memory sequence");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(notes.len(), 1);
  assert_eq!(notes[0].id, "memory-7");
  assert_eq!(notes[0].tags, vec!["workspace", "session"]);
  assert_eq!(next_sequence, 8);
}

#[test]
fn sqlite_store_round_trips_plugin_states() {
  let root = create_temp_directory("plugin-state");
  let store = RuntimeStore::new(root.join("amentia.db"));

  store
    .save_plugin_enabled("workspace-notes", true)
    .expect("save plugin enabled");
  store
    .save_plugin_enabled("shell-recorder", false)
    .expect("save plugin disabled");
  let plugin_states = store.load_plugin_states().expect("load plugin states");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(plugin_states.get("workspace-notes"), Some(&true));
  assert_eq!(plugin_states.get("shell-recorder"), Some(&false));
}

#[test]
fn sqlite_store_deletes_plugin_state() {
  let root = create_temp_directory("plugin-state-delete");
  let store = RuntimeStore::new(root.join("amentia.db"));

  store
    .save_plugin_enabled("workspace-notes", true)
    .expect("save plugin enabled");
  store
    .delete_plugin_state("workspace-notes")
    .expect("delete plugin state");
  let plugin_states = store.load_plugin_states().expect("load plugin states");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert!(!plugin_states.contains_key("workspace-notes"));
}

#[test]
fn sqlite_store_round_trips_plugin_connector_credentials() {
  let root = create_temp_directory("plugin-connector-credentials");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let credential = StoredPluginConnectorCredential {
    connector_id: "notion-connector::notion".to_string(),
    plugin_id: "notion-connector".to_string(),
    credential_store: "local".to_string(),
    credential_label: "Notion authorization marker".to_string(),
    authorized_at: 10,
    updated_at: 10,
  };

  store
    .save_plugin_connector_credential(&credential)
    .expect("save connector credential");
  let credentials = store
    .load_plugin_connector_credentials()
    .expect("load connector credentials");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(credentials, vec![credential]);
}

#[test]
fn sqlite_store_deletes_plugin_connector_credentials() {
  let root = create_temp_directory("plugin-connector-credential-delete");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let credential = StoredPluginConnectorCredential {
    connector_id: "notion-connector::notion".to_string(),
    plugin_id: "notion-connector".to_string(),
    credential_store: "local".to_string(),
    credential_label: "Notion authorization marker".to_string(),
    authorized_at: 10,
    updated_at: 10,
  };

  store
    .save_plugin_connector_credential(&credential)
    .expect("save connector credential");
  store
    .delete_plugin_connector_credential("notion-connector::notion")
    .expect("delete connector credential");
  let credentials = store
    .load_plugin_connector_credentials()
    .expect("load connector credentials");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert!(credentials.is_empty());
}

#[test]
fn sqlite_store_deletes_plugin_connector_credentials_by_plugin() {
  let root = create_temp_directory("plugin-connector-credential-plugin-delete");
  let store = RuntimeStore::new(root.join("amentia.db"));
  let notion = StoredPluginConnectorCredential {
    connector_id: "notion-connector::notion".to_string(),
    plugin_id: "notion-connector".to_string(),
    credential_store: "local".to_string(),
    credential_label: "Notion authorization marker".to_string(),
    authorized_at: 10,
    updated_at: 10,
  };
  let other = StoredPluginConnectorCredential {
    connector_id: "other-connector::service".to_string(),
    plugin_id: "other-connector".to_string(),
    credential_store: "local".to_string(),
    credential_label: "Other authorization marker".to_string(),
    authorized_at: 11,
    updated_at: 11,
  };

  store
    .save_plugin_connector_credential(&notion)
    .expect("save notion credential");
  store
    .save_plugin_connector_credential(&other)
    .expect("save other credential");
  store
    .delete_plugin_connector_credentials_for_plugin("notion-connector")
    .expect("delete connector credentials for plugin");
  let credentials = store
    .load_plugin_connector_credentials()
    .expect("load connector credentials");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(credentials, vec![other]);
}
