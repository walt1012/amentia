use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use pith_memory::MemoryNote;
use pith_protocol::{ThreadSummary, TimelineItem, WorkspaceSummary};
use rusqlite::Connection;

use super::*;

fn create_temp_directory(label: &str) -> PathBuf {
  let unique = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .expect("system time")
    .as_nanos();
  let path = env::temp_dir().join(format!("pith-storage-{label}-{unique}"));
  fs::create_dir_all(&path).expect("create temp directory");
  path
}

#[test]
fn sqlite_store_round_trips_threads_and_workspace() {
  let root = create_temp_directory("sqlite-roundtrip");
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));

  store
    .save_workspace(&WorkspaceSummary {
      root_path: "/tmp/pith".to_string(),
      display_name: "pith".to_string(),
    })
    .expect("save workspace");
  store
    .save_threads(&[StoredThreadRecord {
      summary: ThreadSummary {
        id: "thread-1".to_string(),
        title: "Thread".to_string(),
        status: "ready".to_string(),
        workspace: Some(WorkspaceSummary {
          root_path: "/tmp/pith".to_string(),
          display_name: "pith".to_string(),
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
        root_path: "/tmp/pith".to_string(),
        display_name: "pith".to_string(),
      }),
    }])
    .expect("save threads");

  let workspace = store.load_workspace().expect("load workspace");
  let threads = store.load_threads().expect("load threads");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(workspace.expect("workspace").display_name, "pith");
  assert_eq!(threads.len(), 1);
  assert_eq!(threads[0].turn_count, 3);
  assert_eq!(threads[0].summary.id, "thread-1");
  assert_eq!(
    threads[0]
      .workspace
      .as_ref()
      .expect("thread workspace")
      .display_name,
    "pith"
  );
}

#[test]
fn sqlite_store_round_trips_pending_approvals_and_resolution_audit() {
  let root = create_temp_directory("approval-roundtrip");
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));
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
fn sqlite_store_imports_legacy_json_threads() {
  let root = create_temp_directory("legacy-import");
  let database_path = root.join("pith.db");
  let legacy_path = root.join("threads.json");
  let legacy_workspace = WorkspaceSummary {
    root_path: "/tmp/pith-legacy".to_string(),
    display_name: "pith-legacy".to_string(),
  };
  fs::write(
    &legacy_path,
    serde_json::to_string(&vec![StoredThreadRecord {
      summary: ThreadSummary {
        id: "thread-legacy".to_string(),
        title: "Legacy".to_string(),
        status: "ready".to_string(),
        workspace: Some(legacy_workspace.clone()),
      },
      turn_count: 1,
      items: vec![],
      workspace: Some(legacy_workspace.clone()),
    }])
    .expect("serialize legacy threads"),
  )
  .expect("write legacy threads");

  let store = RuntimeStore::new(database_path, legacy_path);
  let threads = store.load_threads().expect("load migrated threads");

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(threads.len(), 1);
  assert_eq!(threads[0].summary.id, "thread-legacy");
  assert_eq!(
    threads[0]
      .workspace
      .as_ref()
      .map(|workspace| workspace.root_path.as_str()),
    Some("/tmp/pith-legacy")
  );
  assert_eq!(
    threads[0]
      .summary
      .workspace
      .as_ref()
      .map(|workspace| workspace.display_name.as_str()),
    Some("pith-legacy")
  );
}

#[test]
fn sqlite_store_migrates_existing_version_one_database() {
  let root = create_temp_directory("migrate-v1");
  let database_path = root.join("pith.db");
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

  let store = RuntimeStore::new(database_path.clone(), root.join("threads.json"));
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

  fs::remove_dir_all(&root).expect("cleanup temp directory");

  assert_eq!(threads.len(), 1);
  assert_eq!(threads[0].summary.id, "thread-old");
  assert!(pending_approvals.is_empty());
  assert_eq!(migration_versions, vec![1, 2, 3, 4, 5, 6, 7]);
  assert!(approval_indexes.contains(&"idx_approvals_requested_at".to_string()));
}

#[test]
fn sqlite_store_round_trips_memory_notes() {
  let root = create_temp_directory("memory-roundtrip");
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));
  let note = MemoryNote {
    id: "memory-7".to_string(),
    title: "Opened workspace pith".to_string(),
    body: "Pith opened the workspace at /tmp/pith.".to_string(),
    scope: "pith".to_string(),
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
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));

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
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));

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
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));
  let credential = StoredPluginConnectorCredential {
    connector_id: "notion-connector::notion".to_string(),
    plugin_id: "notion-connector".to_string(),
    credential_store: "keychain".to_string(),
    credential_label: "Notion authorization marker".to_string(),
    credential_secret: Some("notion-token".to_string()),
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
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));
  let credential = StoredPluginConnectorCredential {
    connector_id: "notion-connector::notion".to_string(),
    plugin_id: "notion-connector".to_string(),
    credential_store: "keychain".to_string(),
    credential_label: "Notion authorization marker".to_string(),
    credential_secret: None,
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
  let store = RuntimeStore::new(root.join("pith.db"), root.join("threads.json"));
  let notion = StoredPluginConnectorCredential {
    connector_id: "notion-connector::notion".to_string(),
    plugin_id: "notion-connector".to_string(),
    credential_store: "keychain".to_string(),
    credential_label: "Notion authorization marker".to_string(),
    credential_secret: None,
    authorized_at: 10,
    updated_at: 10,
  };
  let other = StoredPluginConnectorCredential {
    connector_id: "other-connector::service".to_string(),
    plugin_id: "other-connector".to_string(),
    credential_store: "keychain".to_string(),
    credential_label: "Other authorization marker".to_string(),
    credential_secret: None,
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
