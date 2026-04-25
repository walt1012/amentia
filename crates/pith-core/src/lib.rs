use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::Result;
use context_compaction::{pack_relevant_memory_notes, ContextPack};
use pith_memory::{MemoryEvent, MemoryManager, MemoryNote};
use pith_model_runtime::{
  GenerateRequest, LocalModelRuntime, ModelBootstrap, ModelHealth, ModelRole,
};
use pith_plugin_host::{
  build_capability_registry, build_command_registry, build_connector_registry, build_hook_registry,
  configured_plugin_install_root, configured_plugin_roots, discover_plugins_in_roots,
  inspect_plugin_bundle, install_plugin_bundle, remove_local_plugin_bundle,
  PluginCapabilityRegistration as HostPluginCapabilityRegistration, PluginCatalogEntry,
  PluginCommandEntry as HostPluginCommandEntry, PluginConnectorEntry as HostPluginConnectorEntry,
  PluginHookEntry as HostPluginHookEntry,
};
use pith_protocol::{
  methods, ApprovalRequest, ApprovalRespondParams, ApprovalRespondResult, HealthPingResult,
  InitializeParams, InitializeResult, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
  MemoryCreateParams, MemoryCreateResult, MemoryListResult, MemoryNoteSummary, MemoryStatusResult,
  ModelBootstrapResult, ModelHealthResult, PluginCapabilityRegistration,
  PluginCapabilityRegistryResult, PluginCapabilityRegistrySummary, PluginCommandRegistryResult,
  PluginCommandRunParams, PluginCommandSummary, PluginConnectorRegistryResult,
  PluginConnectorSummary, PluginHookRegistryResult, PluginHookSummary, PluginInstallParams,
  PluginInstallResult, PluginListResult, PluginRemoveParams, PluginRemoveResult,
  PluginSetEnabledParams, PluginSetEnabledResult, PluginSummary as ProtocolPluginSummary,
  ServerCapabilities, ServerInfo, ThreadListResult, ThreadReadParams, ThreadReadResult,
  ThreadStartParams, ThreadStartResult, ThreadSummary, ThreadUpdatedNotificationParams,
  TimelineItem, TurnCancelParams, TurnCancelResult, TurnStartParams, TurnStartResult,
  WorkspaceCurrentResult, WorkspaceOpenParams, WorkspaceOpenResult, WorkspaceSearchMatch,
  WorkspaceSearchParams, WorkspaceSearchResult, WorkspaceSummary,
};
use pith_storage::{FileThreadStore, StoredApprovalRecord, StoredThreadRecord};
use pith_tools::{
  generate_diff, list_directory, read_file, run_shell, search_files, write_file, DirectoryEntry,
  ReadFileResult, SearchMatch, ShellCommandResult,
};

mod context_compaction;

#[derive(Debug, Clone)]
struct StoredThread {
  summary: ThreadSummary,
  turn_count: usize,
  items: Vec<TimelineItem>,
  workspace: Option<WorkspaceSummary>,
}

#[derive(Debug, Clone)]
struct PendingApproval {
  id: String,
  thread_id: String,
  action: String,
  title: String,
  relative_path: String,
  content: Option<String>,
  command: Option<String>,
}

#[derive(Debug, Clone)]
struct PluginHookMemoryCapture {
  hook: HostPluginHookEntry,
  content: String,
  command: String,
  exit_code: i32,
  stdout_preview: String,
  stderr_preview: String,
}

#[derive(Debug, Clone)]
struct WriteIntent {
  relative_path: String,
  content: String,
}

#[derive(Debug, Clone)]
struct ActiveTurn {
  id: String,
  thread_id: String,
  full_content: String,
  emitted_chars: usize,
  total_chars: usize,
  started_at: Instant,
}

#[derive(Debug, Clone)]
pub struct RuntimeContext {
  server_name: String,
  server_version: String,
  model_runtime: LocalModelRuntime,
  memory_manager: MemoryManager,
  store: Option<FileThreadStore>,
  memory_notes: Vec<MemoryNote>,
  threads: Vec<StoredThread>,
  workspace: Option<WorkspaceSummary>,
  plugin_roots: Vec<PathBuf>,
  plugin_install_root: PathBuf,
  plugins: Vec<PluginCatalogEntry>,
  pending_approvals: HashMap<String, PendingApproval>,
  active_turns: HashMap<String, ActiveTurn>,
  next_thread_number: usize,
  next_approval_number: usize,
}

impl RuntimeContext {
  pub fn new() -> Result<Self> {
    let store = FileThreadStore::new_default()?;
    let persisted_threads = store.load_threads()?;
    let persisted_workspace = store.load_workspace()?;
    let persisted_pending_approvals = store.load_pending_approvals()?;
    let persisted_memory_notes = store.load_memory_notes(128)?;
    let persisted_plugin_states = store.load_plugin_states()?;
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    let plugins = apply_plugin_states(
      load_plugin_catalog(&plugin_roots)?,
      &persisted_plugin_states,
    );
    let next_thread_number = persisted_threads.len() + 1;
    let next_approval_number = store.next_approval_sequence()?;
    let next_memory_number = store.next_memory_sequence()?;

    Ok(Self {
      server_name: "pith-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      model_runtime: LocalModelRuntime::new_default(),
      memory_manager: MemoryManager::new(next_memory_number),
      store: Some(store),
      memory_notes: persisted_memory_notes,
      threads: persisted_threads
        .into_iter()
        .map(|thread| StoredThread {
          summary: thread.summary,
          turn_count: thread.turn_count,
          items: thread.items,
          workspace: thread.workspace,
        })
        .collect(),
      workspace: persisted_workspace,
      plugin_roots,
      plugin_install_root,
      plugins,
      pending_approvals: persisted_pending_approvals
        .into_iter()
        .map(|approval| {
          (
            approval.id.clone(),
            PendingApproval {
              id: approval.id,
              thread_id: approval.thread_id,
              action: approval.action,
              title: approval.title,
              relative_path: approval.relative_path,
              content: approval.content,
              command: approval.command,
            },
          )
        })
        .collect(),
      active_turns: HashMap::new(),
      next_thread_number,
      next_approval_number,
    })
  }

  pub fn new_in_memory() -> Self {
    let plugin_roots = configured_plugin_roots();
    let plugin_install_root = configured_plugin_install_root();
    Self {
      server_name: "pith-runtime".to_string(),
      server_version: env!("CARGO_PKG_VERSION").to_string(),
      model_runtime: LocalModelRuntime::new_default(),
      memory_manager: MemoryManager::new(1),
      store: None,
      memory_notes: vec![],
      threads: vec![],
      workspace: None,
      plugin_roots: plugin_roots.clone(),
      plugin_install_root,
      plugins: load_plugin_catalog(&plugin_roots).unwrap_or_default(),
      pending_approvals: HashMap::new(),
      active_turns: HashMap::new(),
      next_thread_number: 1,
      next_approval_number: 1,
    }
  }

  fn persist_threads(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    let threads = self
      .threads
      .iter()
      .map(|thread| StoredThreadRecord {
        summary: thread.summary.clone(),
        turn_count: thread.turn_count,
        items: thread.items.clone(),
        workspace: thread.workspace.clone(),
      })
      .collect::<Vec<_>>();

    store.save_threads(&threads)
  }

  fn persist_pending_approvals(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    let approvals = self
      .pending_approvals
      .values()
      .cloned()
      .map(stored_approval_record)
      .collect::<Vec<_>>();

    store.save_pending_approvals(&approvals)
  }

  fn persist_runtime_state(&self) -> Result<()> {
    self.persist_threads()?;
    self.persist_pending_approvals()
  }

  fn persist_memory_note(&self, note: &MemoryNote) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    store.save_memory_note(note)
  }

  fn persist_workspace(&self) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };
    let Some(workspace) = &self.workspace else {
      return Ok(());
    };

    store.save_workspace(workspace)
  }

  fn persist_resolved_approval(&self, approval: &PendingApproval, decision: &str) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    store.resolve_approval(&stored_approval_record(approval.clone()), decision)
  }

  fn remember(&mut self, event: MemoryEvent) -> Result<MemoryNote> {
    let note = self
      .memory_manager
      .record_event(&mut self.memory_notes, event);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  fn create_memory_note(
    &mut self,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> Result<MemoryNote> {
    let note =
      self
        .memory_manager
        .create_note(&mut self.memory_notes, title, body, scope, source, tags);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  fn upsert_memory_note(
    &mut self,
    id: String,
    title: String,
    body: String,
    scope: String,
    source: String,
    tags: Vec<String>,
  ) -> Result<MemoryNote> {
    let note =
      self
        .memory_manager
        .upsert_note(&mut self.memory_notes, id, title, body, scope, source, tags);
    self.persist_memory_note(&note)?;
    Ok(note)
  }

  fn persist_plugin_enabled(&self, plugin_id: &str, enabled: bool) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    store.save_plugin_enabled(plugin_id, enabled)
  }

  fn delete_plugin_state(&self, plugin_id: &str) -> Result<()> {
    let Some(store) = &self.store else {
      return Ok(());
    };

    store.delete_plugin_state(plugin_id)
  }

  fn persisted_plugin_states(&self) -> Result<HashMap<String, bool>> {
    let Some(store) = &self.store else {
      return Ok(HashMap::new());
    };

    store.load_plugin_states()
  }

  fn refresh_plugins(&mut self) -> Result<()> {
    let plugin_states = self.persisted_plugin_states()?;
    self.plugins = apply_plugin_states(load_plugin_catalog(&self.plugin_roots)?, &plugin_states);
    Ok(())
  }
}

impl Default for RuntimeContext {
  fn default() -> Self {
    Self::new_in_memory()
  }
}

pub fn handle_request(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  match request.method.as_str() {
    methods::APPROVAL_RESPOND => handle_approval_respond(context, request),
    methods::INITIALIZE => handle_initialize(context, request),
    methods::HEALTH_PING => JsonRpcResponse::success(
      request.id,
      &HealthPingResult {
        status: "ok".to_string(),
      },
    ),
    methods::MEMORY_CREATE => handle_memory_create(context, request),
    methods::MEMORY_LIST => JsonRpcResponse::success(
      request.id,
      &MemoryListResult {
        notes: context
          .memory_notes
          .iter()
          .take(16)
          .cloned()
          .map(to_protocol_memory_note)
          .collect(),
      },
    ),
    methods::MEMORY_STATUS => JsonRpcResponse::success(
      request.id,
      &to_protocol_memory_status(context.memory_manager.status(&context.memory_notes)),
    ),
    methods::MODEL_BOOTSTRAP => handle_model_bootstrap(context, request),
    methods::MODEL_HEALTH => JsonRpcResponse::success(
      request.id,
      &to_protocol_model_health(context.model_runtime.health()),
    ),
    methods::PLUGIN_CAPABILITY_REGISTRY => JsonRpcResponse::success(
      request.id,
      &build_protocol_capability_registry(&context.plugins),
    ),
    methods::PLUGIN_COMMAND_REGISTRY => JsonRpcResponse::success(
      request.id,
      &build_protocol_command_registry(&context.plugins),
    ),
    methods::PLUGIN_COMMAND_RUN => handle_plugin_command_run(context, request),
    methods::PLUGIN_CONNECTOR_REGISTRY => JsonRpcResponse::success(
      request.id,
      &build_protocol_connector_registry(&context.plugins),
    ),
    methods::PLUGIN_HOOK_REGISTRY => {
      JsonRpcResponse::success(request.id, &build_protocol_hook_registry(&context.plugins))
    }
    methods::PLUGIN_INSTALL => handle_plugin_install(context, request),
    methods::PLUGIN_LIST => JsonRpcResponse::success(
      request.id,
      &PluginListResult {
        plugins: context
          .plugins
          .iter()
          .cloned()
          .map(to_protocol_plugin)
          .collect(),
      },
    ),
    methods::PLUGIN_REMOVE => handle_plugin_remove(context, request),
    methods::PLUGIN_SET_ENABLED => handle_plugin_set_enabled(context, request),
    methods::WORKSPACE_CURRENT => JsonRpcResponse::success(
      request.id,
      &WorkspaceCurrentResult {
        workspace: context.workspace.clone(),
      },
    ),
    methods::WORKSPACE_OPEN => handle_workspace_open(context, request),
    methods::WORKSPACE_SEARCH => handle_workspace_search(context, request),
    methods::TURN_CANCEL => handle_turn_cancel(context, request),
    methods::THREAD_READ => handle_thread_read(context, request),
    methods::THREAD_START => handle_thread_start(context, request),
    methods::THREAD_LIST => JsonRpcResponse::success(
      request.id,
      &ThreadListResult {
        threads: context
          .threads
          .iter()
          .map(|thread| thread.summary.clone())
          .collect(),
      },
    ),
    methods::TURN_START => handle_turn_start(context, request),
    _ => JsonRpcResponse::error(request.id, -32601, "Method not found"),
  }
}

pub fn collect_notifications(context: &mut RuntimeContext) -> Result<Vec<JsonRpcNotification>> {
  let active_turn_ids = context.active_turns.keys().cloned().collect::<Vec<_>>();
  let mut notifications = vec![];
  let mut did_update = false;

  for turn_id in active_turn_ids {
    if let Some(params) = advance_active_turn(context, &turn_id)? {
      did_update = true;
      notifications.push(JsonRpcNotification {
        method: methods::THREAD_UPDATED_NOTIFICATION.to_string(),
        params: Some(serde_json::to_value(params)?),
      });
    }
  }

  if did_update {
    context.persist_runtime_state()?;
  }

  Ok(notifications)
}

fn to_protocol_model_health(health: ModelHealth) -> ModelHealthResult {
  ModelHealthResult {
    pack_id: health.pack_id,
    display_name: health.display_name,
    backend: health.backend,
    status: health.status,
    detail: health.detail,
    source: health.source,
    binary_path: health.binary_path,
    model_path: health.model_path,
    manifest_path: health.manifest_path,
    metrics: health.metrics,
  }
}

fn to_protocol_model_bootstrap(result: ModelBootstrap) -> ModelBootstrapResult {
  ModelBootstrapResult {
    manifest_path: result.manifest_path.display().to_string(),
    readme_path: result.readme_path.map(|path| path.display().to_string()),
    copied_files: result
      .copied_files
      .into_iter()
      .map(|path| path.display().to_string())
      .collect(),
  }
}

fn to_protocol_memory_note(note: MemoryNote) -> MemoryNoteSummary {
  MemoryNoteSummary {
    id: note.id,
    title: note.title,
    body: note.body,
    scope: note.scope,
    source: note.source,
    created_at: note.created_at,
    tags: note.tags,
  }
}

fn to_protocol_memory_status(status: pith_memory::MemoryStatus) -> MemoryStatusResult {
  MemoryStatusResult {
    note_count: status.note_count,
    latest_title: status.latest_title,
    summary: status.summary,
  }
}

fn to_protocol_plugin(plugin: PluginCatalogEntry) -> ProtocolPluginSummary {
  ProtocolPluginSummary {
    id: plugin.id,
    name: plugin.name,
    version: plugin.version,
    display_name: plugin.display_name,
    status: plugin.status,
    description: plugin.description,
    author_name: plugin.author_name,
    enabled: plugin.enabled,
    default_enabled: plugin.default_enabled,
    capabilities: plugin.capabilities,
    permissions: plugin.permissions,
    manifest_path: plugin.manifest_path,
    provenance: plugin.provenance,
    validation_error: plugin.validation_error,
    validation_hint: plugin.validation_hint,
  }
}

fn to_protocol_capability(
  capability: HostPluginCapabilityRegistration,
) -> PluginCapabilityRegistration {
  PluginCapabilityRegistration {
    capability_id: capability.capability_id,
    kind: capability.kind,
    identifier: capability.identifier,
    plugin_id: capability.plugin_id,
    plugin_display_name: capability.plugin_display_name,
    permissions: capability.permissions,
    manifest_path: capability.manifest_path,
    metadata: capability.metadata,
  }
}

fn build_protocol_capability_registry(
  plugins: &[PluginCatalogEntry],
) -> PluginCapabilityRegistryResult {
  let capabilities = build_capability_registry(plugins)
    .into_iter()
    .map(to_protocol_capability)
    .collect::<Vec<_>>();
  let enabled_plugin_count = plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
    .count();
  let mut capability_counts_by_kind = HashMap::new();
  for capability in &capabilities {
    *capability_counts_by_kind
      .entry(capability.kind.clone())
      .or_insert(0) += 1;
  }

  PluginCapabilityRegistryResult {
    summary: PluginCapabilityRegistrySummary {
      enabled_plugin_count,
      total_capability_count: capabilities.len(),
      capability_counts_by_kind,
    },
    capabilities,
  }
}

fn to_protocol_plugin_command(command: HostPluginCommandEntry) -> PluginCommandSummary {
  let memory_summary = command
    .memory_note_title
    .as_ref()
    .map(|title| format!("Stores a workspace memory note as `{title}` after execution."));
  PluginCommandSummary {
    command_id: command.command_id,
    title: command.title,
    description: command.description,
    plugin_id: command.plugin_id,
    plugin_display_name: command.plugin_display_name,
    permissions: command.permissions,
    source_path: command.source_path,
    execution_kind: command.execution_kind,
    memory_summary,
  }
}

fn build_protocol_command_registry(plugins: &[PluginCatalogEntry]) -> PluginCommandRegistryResult {
  PluginCommandRegistryResult {
    commands: build_command_registry(plugins)
      .into_iter()
      .map(to_protocol_plugin_command)
      .collect(),
  }
}

fn to_protocol_plugin_connector(connector: HostPluginConnectorEntry) -> PluginConnectorSummary {
  PluginConnectorSummary {
    connector_id: connector.connector_id,
    display_name: connector.display_name,
    service: connector.service,
    plugin_id: connector.plugin_id,
    plugin_display_name: connector.plugin_display_name,
    enabled: connector.enabled,
    status: connector.status,
    permissions: connector.permissions,
    manifest_path: connector.manifest_path,
    homepage: connector.homepage,
    auth_type: connector.auth_type,
    auth_required: connector.auth_required,
    auth_scopes: connector.auth_scopes,
    credential_store: connector.credential_store,
  }
}

fn build_protocol_connector_registry(
  plugins: &[PluginCatalogEntry],
) -> PluginConnectorRegistryResult {
  PluginConnectorRegistryResult {
    connectors: build_connector_registry(plugins)
      .into_iter()
      .map(to_protocol_plugin_connector)
      .collect(),
  }
}

fn to_protocol_plugin_hook(hook: HostPluginHookEntry) -> PluginHookSummary {
  let memory_summary = hook
    .memory_note_title
    .as_ref()
    .map(|title| format!("Stores a workspace memory note as `{title}` when the hook runs."));
  PluginHookSummary {
    hook_id: hook.hook_id,
    title: hook.title,
    description: hook.description,
    event: hook.event,
    plugin_id: hook.plugin_id,
    plugin_display_name: hook.plugin_display_name,
    permissions: hook.permissions,
    source_path: hook.source_path,
    memory_summary,
  }
}

fn build_protocol_hook_registry(plugins: &[PluginCatalogEntry]) -> PluginHookRegistryResult {
  PluginHookRegistryResult {
    hooks: build_hook_registry(plugins)
      .into_iter()
      .map(to_protocol_plugin_hook)
      .collect(),
  }
}

fn shell_output_preview(output: &str) -> String {
  let preview = output
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or(output)
    .trim();

  if preview.is_empty() {
    "none".to_string()
  } else {
    preview.chars().take(120).collect()
  }
}

fn render_hook_message(template: &str, replacements: &[(&str, String)]) -> String {
  let mut rendered = template.to_string();
  for (key, value) in replacements {
    rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
  }
  rendered
}

fn build_shell_completed_hook_items(
  plugins: &[PluginCatalogEntry],
  workspace: &WorkspaceSummary,
  command: &str,
  result: &ShellCommandResult,
) -> (Vec<TimelineItem>, Vec<PluginHookMemoryCapture>) {
  let stdout_preview = shell_output_preview(&result.stdout);
  let stderr_preview = shell_output_preview(&result.stderr);
  let mut items = vec![];
  let mut memory_captures = vec![];

  for hook in build_hook_registry(plugins)
    .into_iter()
    .filter(|hook| hook.event == "shell.completed")
  {
    let content = render_hook_message(
      &hook.message_template,
      &[
        ("workspaceName", workspace.display_name.clone()),
        ("command", command.to_string()),
        ("exitCode", result.exit_code.to_string()),
        ("stdoutPreview", stdout_preview.clone()),
        ("stderrPreview", stderr_preview.clone()),
      ],
    );
    if hook.memory_note_title.is_some() {
      memory_captures.push(PluginHookMemoryCapture {
        hook: hook.clone(),
        content: content.clone(),
        command: command.to_string(),
        exit_code: result.exit_code,
        stdout_preview: stdout_preview.clone(),
        stderr_preview: stderr_preview.clone(),
      });
    }
    items.push(TimelineItem {
      kind: "pluginHook".to_string(),
      title: hook.title,
      content,
      attributes: Some(HashMap::from([
        ("hookId".to_string(), hook.hook_id),
        ("hookEvent".to_string(), hook.event),
        ("pluginId".to_string(), hook.plugin_id),
        ("command".to_string(), command.to_string()),
        ("exitCode".to_string(), result.exit_code.to_string()),
        ("sourcePath".to_string(), hook.source_path),
      ])),
    });
  }

  (items, memory_captures)
}

fn handle_initialize(context: &RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<InitializeParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid initialize params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing initialize params");
    }
  };

  let _client = params.client_info;

  JsonRpcResponse::success(
    request.id,
    &InitializeResult {
      server_info: ServerInfo {
        name: context.server_name.clone(),
        version: context.server_version.clone(),
      },
      protocol_version: "0.1.0".to_string(),
      capabilities: ServerCapabilities {
        supports_memory: true,
        supports_threads: true,
        supports_tools: true,
        supports_plugins: !context.plugins.is_empty(),
      },
    },
  )
}

fn load_plugin_catalog(plugin_roots: &[PathBuf]) -> Result<Vec<PluginCatalogEntry>> {
  if plugin_roots.is_empty() {
    return Ok(vec![]);
  }

  discover_plugins_in_roots(plugin_roots)
}

fn apply_plugin_states(
  mut plugins: Vec<PluginCatalogEntry>,
  persisted_states: &HashMap<String, bool>,
) -> Vec<PluginCatalogEntry> {
  for plugin in &mut plugins {
    if plugin.status != "ready" {
      plugin.enabled = false;
      continue;
    }
    if let Some(enabled) = persisted_states.get(&plugin.id) {
      plugin.enabled = *enabled;
    }
  }

  plugins
}

fn granted_permission_sources(plugins: &[PluginCatalogEntry]) -> HashMap<String, Vec<String>> {
  let mut permissions = HashMap::new();

  for plugin in plugins
    .iter()
    .filter(|plugin| plugin.status == "ready" && plugin.enabled)
  {
    for permission in &plugin.permissions {
      permissions
        .entry(permission.clone())
        .or_insert_with(Vec::new)
        .push(plugin.display_name.clone());
    }
  }

  for plugin_names in permissions.values_mut() {
    plugin_names.sort();
    plugin_names.dedup();
  }

  permissions
}

fn permission_is_granted(
  permission_sources: &HashMap<String, Vec<String>>,
  permission: &str,
) -> bool {
  permission_sources.contains_key(permission)
}

fn build_permission_denied_items(
  permission_sources: &HashMap<String, Vec<String>>,
  permission: &str,
  blocked_action: &str,
  workspace_name: &str,
  mut attributes: HashMap<String, String>,
) -> Vec<TimelineItem> {
  let granted_by = permission_sources
    .get(permission)
    .map(|plugins| plugins.join(", "))
    .unwrap_or_else(|| "none".to_string());
  attributes.insert("requiredPermission".to_string(), permission.to_string());
  attributes.insert("blockedAction".to_string(), blocked_action.to_string());
  attributes.insert("grantedBy".to_string(), granted_by.clone());

  vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Plugin Permission Required".to_string(),
      content: format!(
        "Pith could not {blocked_action} in {workspace_name} because no enabled plugin grants `{permission}`."
      ),
      attributes: Some(attributes.clone()),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: format!(
        "Enable a plugin that grants `{permission}` before asking Pith to {blocked_action}. Currently granted by: {granted_by}."
      ),
      attributes: Some(attributes),
    },
  ]
}

fn handle_memory_create(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<MemoryCreateParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid memory/create params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing memory/create params");
    }
  };

  let Some(workspace) = context.workspace.clone() else {
    return JsonRpcResponse::error(
      request.id,
      -32040,
      "Open a workspace before creating memory notes",
    );
  };

  let title = params.title.trim();
  let body = params.body.trim();
  if title.is_empty() || body.is_empty() {
    return JsonRpcResponse::error(
      request.id,
      -32602,
      "memory/create title and body must be non-empty",
    );
  }

  match context.create_memory_note(
    title.to_string(),
    body.to_string(),
    workspace.display_name,
    "user".to_string(),
    vec![
      "workspace".to_string(),
      "user".to_string(),
      "manual".to_string(),
    ],
  ) {
    Ok(note) => JsonRpcResponse::success(
      request.id,
      &MemoryCreateResult {
        note: to_protocol_memory_note(note),
      },
    ),
    Err(error) => JsonRpcResponse::error(request.id, -32041, error.to_string()),
  }
}

fn handle_model_bootstrap(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  match context.model_runtime.bootstrap_pack_metadata() {
    Ok(result) => {
      context.model_runtime = LocalModelRuntime::new_default();
      JsonRpcResponse::success(request.id, &to_protocol_model_bootstrap(result))
    }
    Err(error) => JsonRpcResponse::error(request.id, -32042, error.to_string()),
  }
}

fn handle_plugin_set_enabled(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<PluginSetEnabledParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid plugin/setEnabled params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing plugin/setEnabled params");
    }
  };

  let Some(plugin_index) = context
    .plugins
    .iter()
    .position(|plugin| plugin.id == params.plugin_id)
  else {
    return JsonRpcResponse::error(request.id, -32050, "Plugin not found");
  };
  if context.plugins[plugin_index].status != "ready" {
    return JsonRpcResponse::error(
      request.id,
      -32051,
      context.plugins[plugin_index]
        .validation_error
        .clone()
        .unwrap_or_else(|| "Plugin manifest is invalid".to_string()),
    );
  }

  context.plugins[plugin_index].enabled = params.enabled;
  let plugin_id = context.plugins[plugin_index].id.clone();
  let plugin_enabled = context.plugins[plugin_index].enabled;
  let updated_plugin = context.plugins[plugin_index].clone();

  if let Err(error) = context.persist_plugin_enabled(&plugin_id, plugin_enabled) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &PluginSetEnabledResult {
      plugin: to_protocol_plugin(updated_plugin),
    },
  )
}

fn handle_plugin_install(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<PluginInstallParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid plugin/install params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing plugin/install params");
    }
  };

  let source_path = PathBuf::from(&params.source_path);
  let candidate_plugin = match inspect_plugin_bundle(&source_path) {
    Ok(plugin) => plugin,
    Err(error) => return JsonRpcResponse::error(request.id, -32053, error.to_string()),
  };
  if context
    .plugins
    .iter()
    .any(|plugin| plugin.id == candidate_plugin.id)
  {
    return JsonRpcResponse::error(
      request.id,
      -32053,
      format!(
        "Plugin `{}` is already installed",
        candidate_plugin.display_name
      ),
    );
  }
  let installed_plugin = match install_plugin_bundle(&source_path, &context.plugin_install_root) {
    Ok(plugin) => plugin,
    Err(error) => return JsonRpcResponse::error(request.id, -32053, error.to_string()),
  };

  if let Err(error) = context.refresh_plugins() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  let refreshed_plugin = context
    .plugins
    .iter()
    .find(|plugin| plugin.id == installed_plugin.id)
    .cloned()
    .unwrap_or(installed_plugin);

  JsonRpcResponse::success(
    request.id,
    &PluginInstallResult {
      plugin: to_protocol_plugin(refreshed_plugin),
    },
  )
}

fn handle_plugin_remove(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<PluginRemoveParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid plugin/remove params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing plugin/remove params");
    }
  };

  let manifest_path = PathBuf::from(&params.manifest_path);
  let removed_plugin =
    match remove_local_plugin_bundle(&manifest_path, &context.plugin_install_root) {
      Ok(plugin) => plugin,
      Err(error) => return JsonRpcResponse::error(request.id, -32054, error.to_string()),
    };

  if let Err(error) = context.delete_plugin_state(&removed_plugin.plugin_id) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }
  if let Err(error) = context.refresh_plugins() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &PluginRemoveResult {
      plugin_id: removed_plugin.plugin_id,
      display_name: removed_plugin.display_name,
      removed_path: removed_plugin.removed_path,
    },
  )
}

fn handle_plugin_command_run(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<PluginCommandRunParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid plugin/commandRun params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing plugin/commandRun params");
    }
  };

  let Some(command) = build_command_registry(&context.plugins)
    .into_iter()
    .find(|command| command.command_id == params.command_id)
  else {
    return JsonRpcResponse::error(request.id, -32052, "Plugin command not found");
  };

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  let workspace = thread
    .workspace
    .clone()
    .or_else(|| context.workspace.clone());
  let command_input = params
    .input
    .as_deref()
    .map(str::trim)
    .filter(|input| !input.is_empty());
  let memory_query = command_input
    .map(|input| {
      format!(
        "{} {} {} {}",
        command.title, command.description, command.prompt, input
      )
    })
    .unwrap_or_else(|| {
      format!(
        "{} {} {}",
        command.title, command.description, command.prompt
      )
    });
  let context_pack = pack_memory_context(
    &context.memory_notes,
    workspace.as_ref().map(|entry| entry.display_name.as_str()),
    &memory_query,
  );
  let command_item =
    build_plugin_command_timeline_item(&command, workspace.as_ref(), command_input, &context_pack);
  if let Some(result) = execute_builtin_plugin_command(
    context,
    &params.thread_id,
    &command,
    workspace.clone(),
    command_input,
    command_item.clone(),
  ) {
    return match result {
      Ok(result) => JsonRpcResponse::success(request.id, &result),
      Err((code, message)) => JsonRpcResponse::error(request.id, code, message),
    };
  }

  JsonRpcResponse::error(
    request.id,
    -32053,
    format!(
      "Plugin command `{}` requires an explicit execution contract.",
      command.command_id
    ),
  )
}

fn handle_workspace_open(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<WorkspaceOpenParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid workspace/open params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing workspace/open params");
    }
  };

  let workspace_path = PathBuf::from(params.path);
  if !workspace_path.is_dir() {
    return JsonRpcResponse::error(request.id, -32020, "Workspace path is not a directory");
  }

  let resolved_path = match fs::canonicalize(&workspace_path) {
    Ok(path) => path,
    Err(error) => {
      return JsonRpcResponse::error(
        request.id,
        -32021,
        format!("Failed to resolve workspace path: {error}"),
      )
    }
  };

  let workspace = WorkspaceSummary {
    root_path: resolved_path.display().to_string(),
    display_name: resolved_path
      .file_name()
      .map(|name| name.to_string_lossy().into_owned())
      .filter(|name| !name.is_empty())
      .unwrap_or_else(|| resolved_path.display().to_string()),
  };
  context.workspace = Some(workspace.clone());

  if let Err(error) = context.persist_workspace() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = context.remember(MemoryEvent::WorkspaceOpened {
    display_name: workspace.display_name.clone(),
    root_path: workspace.root_path.clone(),
  }) {
    return JsonRpcResponse::error(request.id, -32011, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &WorkspaceOpenResult {
      workspace,
      thread_count: context.threads.len(),
    },
  )
}

fn handle_thread_read(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ThreadReadParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid thread/read params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing thread/read params");
    }
  };

  let did_refresh = match refresh_active_turn_for_thread(context, &params.thread_id) {
    Ok(did_refresh) => did_refresh,
    Err(error) => {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  };

  if did_refresh {
    if let Err(error) = context.persist_runtime_state() {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  }

  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == params.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };

  JsonRpcResponse::success(
    request.id,
    &ThreadReadResult {
      thread: thread.summary.clone(),
      items: thread.items.clone(),
      pending_approvals: approvals_for_thread(context, &thread.summary.id),
      active_turn_id: active_turn_id_for_thread(context, &thread.summary.id),
    },
  )
}

fn handle_workspace_search(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<WorkspaceSearchParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid workspace/search params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing workspace/search params");
    }
  };

  let Some(workspace) = context.workspace.clone() else {
    return JsonRpcResponse::error(request.id, -32040, "Open a workspace before searching");
  };

  let max_results = params.max_results.unwrap_or(24).clamp(1, 100);
  let matches = match search_files(Path::new(&workspace.root_path), &params.query, max_results) {
    Ok(matches) => matches
      .into_iter()
      .map(|entry| WorkspaceSearchMatch {
        relative_path: entry.relative_path,
        line_number: entry.line_number,
        line: entry.line,
      })
      .collect::<Vec<_>>(),
    Err(error) => {
      return JsonRpcResponse::error(request.id, -32041, error.to_string());
    }
  };

  JsonRpcResponse::success(
    request.id,
    &WorkspaceSearchResult {
      query: params.query,
      workspace,
      matches,
    },
  )
}

fn handle_thread_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ThreadStartParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid thread/start params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing thread/start params");
    }
  };

  let workspace = context.workspace.clone();
  let thread = ThreadSummary {
    id: format!("thread-{}", context.next_thread_number),
    title: params.title,
    status: "ready".to_string(),
    workspace: workspace.clone(),
  };
  context.next_thread_number += 1;
  let items = vec![TimelineItem {
    kind: "system".to_string(),
    title: "Thread Ready".to_string(),
    content: format!("{} is ready for local runtime messages.", thread.title),
    attributes: None,
  }];
  context.threads.push(StoredThread {
    summary: thread.clone(),
    turn_count: 0,
    items: items.clone(),
    workspace,
  });

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  JsonRpcResponse::success(request.id, &ThreadStartResult { thread })
}

fn handle_turn_start(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<TurnStartParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid turn/start params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing turn/start params");
    }
  };

  match execute_turn_request(
    context,
    &params.thread_id,
    params.message.clone(),
    params.message,
    vec![],
  ) {
    Ok(result) => JsonRpcResponse::success(request.id, &result),
    Err((code, message)) => JsonRpcResponse::error(request.id, code, message),
  }
}

fn execute_turn_request(
  context: &mut RuntimeContext,
  requested_thread_id: &str,
  display_message: String,
  message: String,
  initial_items: Vec<TimelineItem>,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);
  let (thread_id, turn_id, items, active_turn_id, pending_active_turn) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == requested_thread_id)
    else {
      return Err((-32004, "Thread not found".to_string()));
    };

    thread.turn_count += 1;
    let turn_count = thread.turn_count;
    let thread_id = thread.summary.id.clone();
    let thread_title = thread.summary.title.clone();
    if thread.workspace.is_none() {
      thread.workspace = current_workspace.clone();
      thread.summary.workspace = thread.workspace.clone();
    }
    let workspace = thread.workspace.clone();
    let turn_id = format!("{thread_id}-turn-{turn_count}");
    let mut pending_active_turn = None;

    thread.summary.status = match &workspace {
      Some(workspace) => format!("{turn_count} turn(s) in {}", workspace.display_name),
      None => format!("{turn_count} turn(s)"),
    };

    let mut items = initial_items;
    items.push(TimelineItem {
      kind: "userMessage".to_string(),
      title: "User".to_string(),
      content: display_message.clone(),
      attributes: None,
    });

    if let Some(workspace) = workspace {
      let workspace_root = Path::new(&workspace.root_path);

      if let Some(write_intent) = infer_write_intent(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.write") {
            format!(
              "Request approval before writing {} in {}.",
              write_intent.relative_path, workspace.display_name
            )
          } else {
            format!(
              "Check plugin permissions before writing {} in {}.",
              write_intent.relative_path, workspace.display_name
            )
          },
        ));
        if !permission_is_granted(&permission_sources, "file.write") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.write",
            "prepare a file write",
            &workspace.display_name,
            HashMap::from([(
              "relativePath".to_string(),
              write_intent.relative_path.clone(),
            )]),
          ));
        } else {
          let approval_id = format!("approval-{}", context.next_approval_number);
          context.next_approval_number += 1;

          let approval = PendingApproval {
            id: approval_id.clone(),
            thread_id: thread_id.clone(),
            action: "write_file".to_string(),
            title: format!("Write {}", write_intent.relative_path),
            relative_path: write_intent.relative_path.clone(),
            content: Some(write_intent.content.clone()),
            command: None,
          };
          context
            .pending_approvals
            .insert(approval_id.clone(), approval.clone());

          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "generate_diff".to_string(),
            content: write_intent.relative_path.clone(),
            attributes: None,
          });
          match generate_diff(
            workspace_root,
            &write_intent.relative_path,
            &write_intent.content,
          ) {
            Ok(diff) => {
              items.push(TimelineItem {
                kind: "diffArtifact".to_string(),
                title: "Diff Preview".to_string(),
                content: diff,
                attributes: Some(HashMap::from([
                  ("action".to_string(), "write_file".to_string()),
                  (
                    "relativePath".to_string(),
                    write_intent.relative_path.clone(),
                  ),
                ])),
              });
            }
            Err(error) => {
              items.push(TimelineItem {
                kind: "warning".to_string(),
                title: "generate_diff failed".to_string(),
                content: error.to_string(),
                attributes: None,
              });
            }
          }
          items.push(TimelineItem {
            kind: "approvalRequested".to_string(),
            title: "Approval Requested".to_string(),
            content: format!(
              "Pith wants to write {} in {}.",
              write_intent.relative_path, workspace.display_name
            ),
            attributes: Some(HashMap::from([
              ("approvalId".to_string(), approval.id.clone()),
              ("action".to_string(), approval.action.clone()),
              ("relativePath".to_string(), approval.relative_path.clone()),
            ])),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: format!(
              "Pith prepared a write for {} and is waiting for your approval.",
              write_intent.relative_path
            ),
            attributes: None,
          });
        }
      } else if let Some(shell_command) = infer_shell_command(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "shell.exec") {
            format!(
              "Request approval before running a shell command in {}.",
              workspace.display_name
            )
          } else {
            format!(
              "Check plugin permissions before running a shell command in {}.",
              workspace.display_name
            )
          },
        ));
        if !permission_is_granted(&permission_sources, "shell.exec") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "shell.exec",
            "run a shell command",
            &workspace.display_name,
            HashMap::from([("command".to_string(), shell_command.clone())]),
          ));
        } else {
          let approval_id = format!("approval-{}", context.next_approval_number);
          context.next_approval_number += 1;

          let approval = PendingApproval {
            id: approval_id.clone(),
            thread_id: thread_id.clone(),
            action: "run_shell".to_string(),
            title: "Run Shell Command".to_string(),
            relative_path: ".".to_string(),
            content: None,
            command: Some(shell_command.clone()),
          };
          context
            .pending_approvals
            .insert(approval_id.clone(), approval.clone());

          items.push(TimelineItem {
            kind: "approvalRequested".to_string(),
            title: "Approval Requested".to_string(),
            content: format!(
              "Pith wants to run this shell command in {}:\n{}",
              workspace.display_name, shell_command
            ),
            attributes: Some(HashMap::from([
              ("approvalId".to_string(), approval.id.clone()),
              ("action".to_string(), approval.action.clone()),
              ("command".to_string(), shell_command),
            ])),
          });
          items.push(TimelineItem {
            kind: "assistantMessage".to_string(),
            title: "Assistant".to_string(),
            content: "Pith is waiting for your approval before running the shell command."
              .to_string(),
            attributes: None,
          });
        }
      } else if let Some(relative_path) = infer_requested_file_path(&message, workspace_root) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.read") {
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
        if !permission_is_granted(&permission_sources, "file.read") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.read",
            "inspect a file",
            &workspace.display_name,
            HashMap::from([("relativePath".to_string(), relative_path.clone())]),
          ));
        } else {
          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "read_file".to_string(),
            content: relative_path.clone(),
            attributes: None,
          });

          match read_file(workspace_root, &relative_path, 4096) {
            Ok(result) => {
              items.push(TimelineItem {
                kind: "toolResult".to_string(),
                title: "read_file result".to_string(),
                content: format_file_result(&result),
                attributes: None,
              });
              let (summary, summary_attributes) = summarize_file_result(
                &model_runtime,
                &memory_notes,
                &thread_title,
                &workspace.display_name,
                &result,
              );
              pending_active_turn = maybe_start_streaming_assistant_turn(
                &thread_id,
                &turn_id,
                &mut items,
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
      } else if let Some(search_query) = infer_search_query(&message) {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.read") {
            format!(
              "Search {} for matches to \"{}\" with the built-in search_files tool.",
              workspace.display_name, search_query
            )
          } else {
            format!(
              "Check plugin permissions before searching {} for \"{}\".",
              workspace.display_name, search_query
            )
          },
        ));
        if !permission_is_granted(&permission_sources, "file.read") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.read",
            "search files",
            &workspace.display_name,
            HashMap::from([("query".to_string(), search_query.clone())]),
          ));
        } else {
          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "search_files".to_string(),
            content: search_query.clone(),
            attributes: None,
          });

          match search_files(workspace_root, &search_query, 12) {
            Ok(matches) => {
              items.push(TimelineItem {
                kind: "toolResult".to_string(),
                title: "search_files result".to_string(),
                content: format_search_result(&search_query, &matches),
                attributes: None,
              });
              let (summary, summary_attributes) = summarize_search_result(
                &model_runtime,
                &memory_notes,
                &thread_title,
                &workspace.display_name,
                &search_query,
                &matches,
              );
              pending_active_turn = maybe_start_streaming_assistant_turn(
                &thread_id,
                &turn_id,
                &mut items,
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
      } else {
        items.push(build_plan_item(
          &context.model_runtime,
          &memory_notes,
          &message,
          Some(&workspace),
          if permission_is_granted(&permission_sources, "file.read") {
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
        if !permission_is_granted(&permission_sources, "file.read") {
          items.extend(build_permission_denied_items(
            &permission_sources,
            "file.read",
            "inspect the workspace",
            &workspace.display_name,
            HashMap::new(),
          ));
        } else {
          items.push(TimelineItem {
            kind: "toolStart".to_string(),
            title: "list_directory".to_string(),
            content: ".".to_string(),
            attributes: None,
          });

          match list_directory(workspace_root, None, 24) {
            Ok(entries) => {
              items.push(TimelineItem {
                kind: "toolResult".to_string(),
                title: "list_directory result".to_string(),
                content: format_directory_result(&entries),
                attributes: None,
              });
              let (summary, summary_attributes) = summarize_directory_result(
                &model_runtime,
                &memory_notes,
                &thread_title,
                &workspace.display_name,
                &entries,
              );
              pending_active_turn = maybe_start_streaming_assistant_turn(
                &thread_id,
                &turn_id,
                &mut items,
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
      }
    } else {
      items.push(build_plan_item(
        &context.model_runtime,
        &memory_notes,
        &message,
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
          "Pith received your message in {}, but Milestone 1 tools need an opened workspace first.",
          thread_title
        ),
        attributes: None,
      });
    }

    let active_turn_id = pending_active_turn.as_ref().map(|turn| turn.id.clone());
    if active_turn_id.is_some() {
      thread.summary.status = "Streaming assistant response".to_string();
    } else if !thread.summary.status.contains("approval") {
      thread.summary.status = "Ready".to_string();
    }

    thread.items.extend(items.clone());

    (
      thread_id,
      turn_id,
      items,
      active_turn_id,
      pending_active_turn,
    )
  };

  if let Some(active_turn) = pending_active_turn {
    context
      .active_turns
      .insert(active_turn.id.clone(), active_turn);
  }

  if let Err(error) = context.persist_runtime_state() {
    return Err((-32010, error.to_string()));
  }

  if active_turn_id.is_none() {
    if let Err(error) = refresh_thread_summary_note(context, &thread_id) {
      return Err((-32012, error.to_string()));
    }
  }

  let pending_approvals = approvals_for_thread(context, &thread_id);

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals,
    active_turn_id,
  })
}

fn build_plugin_command_timeline_item(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
  context_pack: &ContextPack,
) -> TimelineItem {
  let mut attributes = HashMap::from([
    ("commandId".to_string(), command.command_id.clone()),
    ("pluginId".to_string(), command.plugin_id.clone()),
    (
      "pluginDisplayName".to_string(),
      command.plugin_display_name.clone(),
    ),
    ("sourcePath".to_string(), command.source_path.clone()),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  if let Some(input) = input {
    attributes.insert("commandInput".to_string(), input.to_string());
  }
  if let Some(execution_kind) = command.execution_kind.as_ref() {
    attributes.insert("executionKind".to_string(), execution_kind.clone());
  }
  merge_context_pack_attributes(&mut attributes, context_pack);

  let workspace_label = workspace
    .map(|entry| entry.display_name.clone())
    .unwrap_or_else(|| "No Workspace".to_string());
  let mut content = format!(
    "Run {} from {} in {}.\n{}",
    command.title, command.plugin_display_name, workspace_label, command.description
  );
  if let Some(input) = input {
    content.push_str(&format!("\nCommand input: {input}"));
  }

  TimelineItem {
    kind: "pluginCommand".to_string(),
    title: command.title.clone(),
    content,
    attributes: Some(attributes),
  }
}

fn execute_builtin_plugin_command(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  workspace: Option<WorkspaceSummary>,
  input: Option<&str>,
  command_item: TimelineItem,
) -> Option<std::result::Result<TurnStartResult, (i32, String)>> {
  let execution_kind = command.execution_kind.as_deref()?;
  let result_content = match execution_kind {
    "builtin.workspaceReadmeNote" => {
      build_workspace_readme_note_result(command, workspace.as_ref(), input)
    }
    "builtin.shellSessionSummary" => {
      build_shell_session_summary_result(context, workspace.as_ref())
    }
    "builtin.reviewDiffSummary" => build_review_diff_summary_result(command, workspace.as_ref()),
    _ => return None,
  };

  let result_item =
    build_plugin_result_timeline_item(command, execution_kind, result_content.clone());
  let assistant_item = TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: format!(
      "{} completed through {}.\n\n{}",
      command.title, command.plugin_display_name, result_content
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
    ])),
  };
  let items = vec![command_item, result_item, assistant_item];
  let result = complete_plugin_command_items(context, thread_id, workspace, items);

  Some(result.and_then(|mut result| {
    match maybe_capture_plugin_command_memory(context, thread_id, command, input, &result.items) {
      Ok(Some(memory_item)) => {
        if let Some(thread) = context
          .threads
          .iter_mut()
          .find(|thread| thread.summary.id == thread_id)
        {
          thread.items.push(memory_item.clone());
        }
        result.items.push(memory_item);
        context
          .persist_runtime_state()
          .map_err(|error| (-32010, error.to_string()))?;
        refresh_thread_summary_note(context, thread_id)
          .map_err(|error| (-32012, error.to_string()))?;
      }
      Ok(None) => {}
      Err(error) => {
        let warning_item = build_plugin_command_memory_warning_item(command, error.to_string());
        if let Some(thread) = context
          .threads
          .iter_mut()
          .find(|thread| thread.summary.id == thread_id)
        {
          thread.items.push(warning_item.clone());
        }
        result.items.push(warning_item);
        context
          .persist_runtime_state()
          .map_err(|error| (-32010, error.to_string()))?;
      }
    }
    Ok(result)
  }))
}

fn complete_plugin_command_items(
  context: &mut RuntimeContext,
  requested_thread_id: &str,
  workspace: Option<WorkspaceSummary>,
  items: Vec<TimelineItem>,
) -> std::result::Result<TurnStartResult, (i32, String)> {
  let (thread_id, turn_id) = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == requested_thread_id)
    else {
      return Err((-32004, "Thread not found".to_string()));
    };

    if thread.workspace.is_none() {
      thread.workspace = workspace.clone();
    }
    thread.turn_count += 1;
    let turn_id = format!("{}-turn-{}", thread.summary.id, thread.turn_count);
    thread.summary.status = match &thread.workspace {
      Some(workspace) => format!(
        "{} plugin command(s) in {}",
        thread.turn_count, workspace.display_name
      ),
      None => format!("{} plugin command(s)", thread.turn_count),
    };
    thread.items.extend(items.clone());
    thread.summary.status = "Ready".to_string();
    (thread.summary.id.clone(), turn_id)
  };

  context
    .persist_runtime_state()
    .map_err(|error| (-32010, error.to_string()))?;
  refresh_thread_summary_note(context, &thread_id).map_err(|error| (-32012, error.to_string()))?;

  Ok(TurnStartResult {
    turn_id,
    thread_id,
    items,
    pending_approvals: approvals_for_thread(context, requested_thread_id),
    active_turn_id: None,
  })
}

fn build_plugin_result_timeline_item(
  command: &HostPluginCommandEntry,
  execution_kind: &str,
  content: String,
) -> TimelineItem {
  TimelineItem {
    kind: "pluginResult".to_string(),
    title: format!("{} Result", command.title),
    content,
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
      ("executionKind".to_string(), execution_kind.to_string()),
      ("sourcePath".to_string(), command.source_path.clone()),
    ])),
  }
}

fn build_workspace_readme_note_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
  input: Option<&str>,
) -> String {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return "This command cannot read workspace files because its plugin does not declare `file.read`."
      .to_string();
  }
  let Some(workspace) = workspace else {
    return "Open a workspace before capturing a workspace note.".to_string();
  };

  match read_file(Path::new(&workspace.root_path), "README.md", 4096) {
    Ok(result) => {
      let summary = compact_text_preview(&result.content, 10, 900);
      let input_summary = input
        .map(|value| format!("\nOperator input: {}", value.trim()))
        .unwrap_or_default();
      format!(
        "Workspace note candidate from README.md in {}.{}\n\n{}",
        workspace.display_name, input_summary, summary
      )
    }
    Err(error) => format!(
      "Could not capture a README-based note in {}: {}",
      workspace.display_name, error
    ),
  }
}

fn build_shell_session_summary_result(
  context: &RuntimeContext,
  workspace: Option<&WorkspaceSummary>,
) -> String {
  let workspace_label = workspace
    .map(|workspace| workspace.display_name.as_str())
    .unwrap_or("the current workspace");
  let shell_notes = context
    .memory_notes
    .iter()
    .filter(|note| {
      note.source == "plugin.shell-recorder"
        || note.tags.iter().any(|tag| tag == "shell" || tag == "hook")
    })
    .rev()
    .take(5)
    .map(|note| {
      format!(
        "- {}: {}",
        note.title,
        compact_text_preview(&note.body, 2, 220)
      )
    })
    .collect::<Vec<_>>();

  if shell_notes.is_empty() {
    return format!(
      "No shell completion notes are recorded for {} yet. Enable Shell Recorder and approve shell commands to build this timeline.",
      workspace_label
    );
  }

  format!(
    "Recent shell activity for {}:\n{}",
    workspace_label,
    shell_notes.join("\n")
  )
}

fn build_review_diff_summary_result(
  command: &HostPluginCommandEntry,
  workspace: Option<&WorkspaceSummary>,
) -> String {
  if !command
    .permissions
    .iter()
    .any(|permission| permission == "file.read")
  {
    return "This command cannot inspect the workspace because its plugin does not declare `file.read`."
      .to_string();
  }
  let Some(workspace) = workspace else {
    return "Open a workspace before inspecting the current diff.".to_string();
  };
  let workspace_root = Path::new(&workspace.root_path);
  let stat = git_workspace_output(workspace_root, &["diff", "--stat"]);
  let names = git_workspace_output(workspace_root, &["diff", "--name-only"]);

  match (stat, names) {
    (Some(stat), Some(names)) if !stat.trim().is_empty() || !names.trim().is_empty() => {
      format!(
        "Current diff snapshot for {}.\n\nChanged files:\n{}\n\nDiff stat:\n{}\n\nReview focus:\n- Check behavioral regressions first.\n- Verify missing tests around changed paths.\n- Inspect risky file writes before approving follow-up changes.",
        workspace.display_name,
        compact_text_preview(&names, 20, 900),
        compact_text_preview(&stat, 20, 1200)
      )
    }
    (Some(_), Some(_)) => format!(
      "No active git diff was detected in {}. The review command is ready once files change.",
      workspace.display_name
    ),
    _ => format!(
      "Could not read a git diff in {}. Ensure the workspace is a git repository and git is available.",
      workspace.display_name
    ),
  }
}

fn git_workspace_output(workspace_root: &Path, args: &[&str]) -> Option<String> {
  let output = Command::new("git")
    .arg("-C")
    .arg(workspace_root)
    .args(args)
    .output()
    .ok()?;
  if !output.status.success() {
    return None;
  }
  Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn compact_text_preview(content: &str, max_lines: usize, max_chars: usize) -> String {
  let mut preview = content
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .take(max_lines)
    .collect::<Vec<_>>()
    .join("\n");
  if preview.is_empty() {
    preview = "No content available.".to_string();
  }
  if preview.chars().count() > max_chars {
    preview = preview.chars().take(max_chars).collect::<String>();
    preview.push_str("...");
  }
  preview
}

fn maybe_capture_plugin_command_memory(
  context: &mut RuntimeContext,
  thread_id: &str,
  command: &HostPluginCommandEntry,
  input: Option<&str>,
  items: &[TimelineItem],
) -> Result<Option<TimelineItem>> {
  let Some(note_title) = command.memory_note_title.as_ref() else {
    return Ok(None);
  };
  let Some(assistant_message) = items
    .iter()
    .rev()
    .find(|item| item.kind == "assistantMessage")
  else {
    return Ok(None);
  };
  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == thread_id)
  else {
    return Ok(None);
  };
  let Some(workspace) = thread
    .workspace
    .as_ref()
    .or(context.workspace.as_ref())
    .cloned()
  else {
    return Ok(None);
  };

  let note_body =
    build_plugin_command_memory_body(command, &workspace, input, &assistant_message.content);
  let note_source = command
    .memory_note_source
    .clone()
    .unwrap_or_else(|| format!("plugin.{}", command.plugin_id));
  let note_tags = plugin_command_memory_tags(command);
  let note = context.create_memory_note(
    note_title.clone(),
    note_body,
    workspace.display_name.clone(),
    note_source,
    note_tags,
  )?;

  Ok(Some(TimelineItem {
    kind: "system".to_string(),
    title: "Memory Note Saved".to_string(),
    content: format!(
      "Saved workspace memory note \"{}\" from {}.",
      note.title, command.title
    ),
    attributes: Some(HashMap::from([
      ("memoryNoteId".to_string(), note.id),
      ("memoryNoteTitle".to_string(), note.title),
      ("memoryScope".to_string(), note.scope),
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
    ])),
  }))
}

fn build_plugin_command_memory_body(
  command: &HostPluginCommandEntry,
  workspace: &WorkspaceSummary,
  input: Option<&str>,
  assistant_content: &str,
) -> String {
  let mut body = format!(
    "Plugin: {} ({})\nCommand: {} ({})\nWorkspace: {} at {}.",
    command.plugin_display_name,
    command.plugin_id,
    command.title,
    command.command_id,
    workspace.display_name,
    workspace.root_path
  );
  if let Some(input) = input {
    body.push_str(&format!("\nCommand input: {input}"));
  }
  body.push_str("\n\nCommand result:\n");
  body.push_str(assistant_content.trim());
  body
}

fn plugin_command_memory_tags(command: &HostPluginCommandEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "command".to_string(),
    command.plugin_id.clone(),
    command.command_id.clone(),
  ];
  for tag in &command.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

fn build_plugin_command_memory_warning_item(
  command: &HostPluginCommandEntry,
  error_message: String,
) -> TimelineItem {
  TimelineItem {
    kind: "warning".to_string(),
    title: "Plugin Memory Capture Failed".to_string(),
    content: format!(
      "{} could not save its workspace memory note. {}",
      command.title, error_message
    ),
    attributes: Some(HashMap::from([
      ("pluginId".to_string(), command.plugin_id.clone()),
      ("commandId".to_string(), command.command_id.clone()),
    ])),
  }
}

fn capture_plugin_hook_memory(
  context: &mut RuntimeContext,
  workspace: &WorkspaceSummary,
  capture: &PluginHookMemoryCapture,
) -> Result<TimelineItem> {
  let Some(note_title) = capture.hook.memory_note_title.as_ref() else {
    return Ok(TimelineItem {
      kind: "system".to_string(),
      title: "Plugin Hook Memory Skipped".to_string(),
      content: format!(
        "{} did not declare a memory note title.",
        capture.hook.title
      ),
      attributes: Some(HashMap::from([(
        "hookId".to_string(),
        capture.hook.hook_id.clone(),
      )])),
    });
  };
  let note_source = capture
    .hook
    .memory_note_source
    .clone()
    .unwrap_or_else(|| format!("plugin.{}", capture.hook.plugin_id));
  let note = context.create_memory_note(
    note_title.clone(),
    build_plugin_hook_memory_body(workspace, capture),
    workspace.display_name.clone(),
    note_source,
    plugin_hook_memory_tags(&capture.hook),
  )?;

  Ok(TimelineItem {
    kind: "system".to_string(),
    title: "Hook Memory Note Saved".to_string(),
    content: format!(
      "Saved workspace memory note \"{}\" from {}.",
      note.title, capture.hook.title
    ),
    attributes: Some(HashMap::from([
      ("memoryNoteId".to_string(), note.id),
      ("memoryNoteTitle".to_string(), note.title),
      ("memoryScope".to_string(), note.scope),
      ("pluginId".to_string(), capture.hook.plugin_id.clone()),
      ("hookId".to_string(), capture.hook.hook_id.clone()),
    ])),
  })
}

fn build_plugin_hook_memory_body(
  workspace: &WorkspaceSummary,
  capture: &PluginHookMemoryCapture,
) -> String {
  format!(
    "Plugin: {} ({})\nHook: {} ({})\nEvent: {}\nWorkspace: {} at {}.\nCommand: {}\nExit code: {}\nstdout: {}\nstderr: {}\n\nHook result:\n{}",
    capture.hook.plugin_display_name,
    capture.hook.plugin_id,
    capture.hook.title,
    capture.hook.hook_id,
    capture.hook.event,
    workspace.display_name,
    workspace.root_path,
    capture.command,
    capture.exit_code,
    capture.stdout_preview,
    capture.stderr_preview,
    capture.content
  )
}

fn plugin_hook_memory_tags(hook: &HostPluginHookEntry) -> Vec<String> {
  let mut tags = vec![
    "plugin".to_string(),
    "hook".to_string(),
    hook.plugin_id.clone(),
    hook.hook_id.clone(),
    hook.event.clone(),
  ];
  for tag in &hook.memory_note_tags {
    if !tags.iter().any(|existing| existing == tag) {
      tags.push(tag.clone());
    }
  }
  tags
}

fn handle_approval_respond(
  context: &mut RuntimeContext,
  request: JsonRpcRequest,
) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<ApprovalRespondParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid approval/respond params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing approval/respond params");
    }
  };

  let decision = params.decision.to_lowercase();
  if decision != "approved" && decision != "denied" {
    return JsonRpcResponse::error(
      request.id,
      -32602,
      "approval/respond decision must be approved or denied",
    );
  }

  let Some(approval) = context.pending_approvals.get(&params.approval_id).cloned() else {
    return JsonRpcResponse::error(request.id, -32030, "Approval request not found");
  };

  let current_workspace = context.workspace.clone();
  let model_runtime = context.model_runtime.clone();
  let memory_notes = context.memory_notes.clone();
  let permission_sources = granted_permission_sources(&context.plugins);

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == approval.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };
  if thread.workspace.is_none() {
    thread.workspace = current_workspace;
    thread.summary.workspace = thread.workspace.clone();
  }
  let workspace = match thread.workspace.clone() {
    Some(workspace) => workspace,
    None => {
      return JsonRpcResponse::error(
        request.id,
        -32031,
        "Open a workspace for this thread before resolving approvals",
      )
    }
  };

  context.pending_approvals.remove(&params.approval_id);

  let mut items = vec![];
  let mut memory_event = None;
  let mut hook_memory_captures = vec![];
  if decision == "approved" {
    items.push(TimelineItem {
      kind: "approvalResolved".to_string(),
      title: "Approval Granted".to_string(),
      content: format!(
        "Approved {} for {}.",
        approval.action, approval.relative_path
      ),
      attributes: Some(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("decision".to_string(), "approved".to_string()),
      ])),
    });
    if approval.action == "write_file" {
      if !permission_is_granted(&permission_sources, "file.write") {
        items.extend(build_permission_denied_items(
          &permission_sources,
          "file.write",
          "complete the approved file write",
          &workspace.display_name,
          HashMap::from([
            ("approvalId".to_string(), approval.id.clone()),
            ("relativePath".to_string(), approval.relative_path.clone()),
          ]),
        ));
      } else {
        let content = approval.content.clone().unwrap_or_default();
        items.push(TimelineItem {
          kind: "toolStart".to_string(),
          title: "write_file".to_string(),
          content: approval.relative_path.clone(),
          attributes: None,
        });

        match write_file(
          Path::new(&workspace.root_path),
          &approval.relative_path,
          &content,
        ) {
          Ok(relative_path) => {
            memory_event = Some(MemoryEvent::FileWritten {
              workspace_display_name: workspace.display_name.clone(),
              relative_path: relative_path.clone(),
            });
            items.push(TimelineItem {
              kind: "toolResult".to_string(),
              title: "write_file result".to_string(),
              content: format!("Wrote {} bytes to {}.", content.len(), relative_path),
              attributes: None,
            });
            items.push(TimelineItem {
              kind: "assistantMessage".to_string(),
              title: "Assistant".to_string(),
              content: format!(
                "Pith wrote {} in {} after your approval.",
                relative_path, workspace.display_name
              ),
              attributes: None,
            });
          }
          Err(error) => {
            items.push(TimelineItem {
              kind: "warning".to_string(),
              title: "write_file failed".to_string(),
              content: error.to_string(),
              attributes: None,
            });
          }
        }
      }
    } else if approval.action == "run_shell" {
      if !permission_is_granted(&permission_sources, "shell.exec") {
        items.extend(build_permission_denied_items(
          &permission_sources,
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
      } else {
        let command = approval.command.clone().unwrap_or_default();
        items.push(TimelineItem {
          kind: "toolStart".to_string(),
          title: "run_shell".to_string(),
          content: command.clone(),
          attributes: None,
        });

        match run_shell(Path::new(&workspace.root_path), &command, 4096) {
          Ok(result) => {
            memory_event = Some(MemoryEvent::ShellCommandRan {
              workspace_display_name: workspace.display_name.clone(),
              command: command.clone(),
            });
            let (summary, summary_attributes) = summarize_shell_result(
              &model_runtime,
              &memory_notes,
              &workspace.display_name,
              &result,
            );
            items.push(TimelineItem {
              kind: "toolResult".to_string(),
              title: "run_shell result".to_string(),
              content: format_shell_result(&result),
              attributes: None,
            });
            items.push(TimelineItem {
              kind: "assistantMessage".to_string(),
              title: "Assistant".to_string(),
              content: summary,
              attributes: Some(summary_attributes),
            });
            let (hook_items, memory_captures) =
              build_shell_completed_hook_items(&context.plugins, &workspace, &command, &result);
            hook_memory_captures.extend(memory_captures);
            items.extend(hook_items);
          }
          Err(error) => {
            items.push(TimelineItem {
              kind: "warning".to_string(),
              title: "run_shell failed".to_string(),
              content: error.to_string(),
              attributes: None,
            });
          }
        }
      }
    }
  } else {
    memory_event = Some(MemoryEvent::ApprovalDenied {
      title: approval.title.clone(),
      action: approval.action.clone(),
    });
    let (summary, summary_attributes) = summarize_denied_approval(
      &model_runtime,
      &memory_notes,
      &workspace.display_name,
      &approval,
    );
    items.push(TimelineItem {
      kind: "approvalResolved".to_string(),
      title: "Approval Denied".to_string(),
      content: format!("Denied {} for {}.", approval.action, approval.relative_path),
      attributes: Some(HashMap::from([
        ("approvalId".to_string(), approval.id.clone()),
        ("decision".to_string(), "denied".to_string()),
      ])),
    });
    items.push(TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: summary,
      attributes: Some(summary_attributes),
    });
  }

  thread.items.extend(items.clone());

  if let Err(error) = context.persist_resolved_approval(&approval, &decision) {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = context.persist_runtime_state() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Some(memory_event) = memory_event {
    if let Err(error) = context.remember(memory_event) {
      return JsonRpcResponse::error(request.id, -32012, error.to_string());
    }
  }

  if !hook_memory_captures.is_empty() {
    let mut hook_memory_items = vec![];
    for capture in &hook_memory_captures {
      match capture_plugin_hook_memory(context, &workspace, capture) {
        Ok(item) => hook_memory_items.push(item),
        Err(error) => hook_memory_items.push(TimelineItem {
          kind: "warning".to_string(),
          title: "Hook Memory Capture Failed".to_string(),
          content: format!(
            "{} could not save its workspace memory note. {}",
            capture.hook.title, error
          ),
          attributes: Some(HashMap::from([
            ("pluginId".to_string(), capture.hook.plugin_id.clone()),
            ("hookId".to_string(), capture.hook.hook_id.clone()),
          ])),
        }),
      }
    }
    if let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == approval.thread_id)
    {
      thread.items.extend(hook_memory_items.clone());
    }
    items.extend(hook_memory_items);

    if let Err(error) = context.persist_runtime_state() {
      return JsonRpcResponse::error(request.id, -32010, error.to_string());
    }
  }

  if let Err(error) = refresh_thread_summary_note(context, &approval.thread_id) {
    return JsonRpcResponse::error(request.id, -32012, error.to_string());
  }

  let pending_approvals = approvals_for_thread(context, &approval.thread_id);

  JsonRpcResponse::success(
    request.id,
    &ApprovalRespondResult {
      approval_id: approval.id,
      thread_id: approval.thread_id.clone(),
      items,
      pending_approvals,
    },
  )
}

fn handle_turn_cancel(context: &mut RuntimeContext, request: JsonRpcRequest) -> JsonRpcResponse {
  let params = match request.params {
    Some(value) => match serde_json::from_value::<TurnCancelParams>(value) {
      Ok(params) => params,
      Err(error) => {
        return JsonRpcResponse::error(
          request.id,
          -32602,
          format!("Invalid turn/cancel params: {error}"),
        )
      }
    },
    None => {
      return JsonRpcResponse::error(request.id, -32602, "Missing turn/cancel params");
    }
  };

  let Some(active_turn_snapshot) = context.active_turns.get(&params.turn_id).cloned() else {
    return JsonRpcResponse::error(request.id, -32040, "Turn is not active");
  };

  let Some(thread) = context
    .threads
    .iter_mut()
    .find(|thread| thread.summary.id == active_turn_snapshot.thread_id)
  else {
    return JsonRpcResponse::error(request.id, -32004, "Thread not found");
  };
  let cancelled_thread_id = thread.summary.id.clone();

  context.active_turns.remove(&params.turn_id);
  let partial_content = take_characters(
    &active_turn_snapshot.full_content,
    compute_streamed_char_count(&active_turn_snapshot)
      .min(active_turn_snapshot.full_content.chars().count()),
  );
  update_streaming_item(
    thread,
    &params.turn_id,
    &partial_content,
    "cancelled",
    partial_content.chars().count(),
    active_turn_snapshot.total_chars,
  );
  thread.summary.status = "Turn cancelled".to_string();

  let items = vec![
    TimelineItem {
      kind: "warning".to_string(),
      title: "Turn Cancelled".to_string(),
      content: format!(
        "Cancelled {} before the assistant response completed.",
        params.turn_id
      ),
      attributes: Some(HashMap::from([(
        "turnId".to_string(),
        params.turn_id.clone(),
      )])),
    },
    TimelineItem {
      kind: "assistantMessage".to_string(),
      title: "Assistant".to_string(),
      content: "Pith stopped the active response at your request.".to_string(),
      attributes: Some(HashMap::from([
        ("turnId".to_string(), params.turn_id.clone()),
        ("streamingStatus".to_string(), "cancelled".to_string()),
      ])),
    },
  ];
  thread.items.extend(items.clone());

  if let Err(error) = context.persist_threads() {
    return JsonRpcResponse::error(request.id, -32010, error.to_string());
  }

  if let Err(error) = refresh_thread_summary_note(context, &cancelled_thread_id) {
    return JsonRpcResponse::error(request.id, -32012, error.to_string());
  }

  JsonRpcResponse::success(
    request.id,
    &TurnCancelResult {
      turn_id: params.turn_id,
      thread_id: active_turn_snapshot.thread_id,
      items,
      active_turn_id: active_turn_id_for_thread(context, &cancelled_thread_id),
    },
  )
}

fn infer_requested_file_path(message: &str, workspace_root: &Path) -> Option<String> {
  let common_files = ["README.md", "Cargo.toml", "Package.swift"];
  let lowercased_message = message.to_lowercase();

  for candidate in common_files {
    if lowercased_message.contains(&candidate.to_lowercase())
      && workspace_root.join(candidate).is_file()
    {
      return Some(candidate.to_string());
    }
  }

  let punctuation: &[char] = &['`', '"', '\'', ',', ';', ':', '(', ')', '[', ']', '{', '}'];
  for token in message.split_whitespace() {
    let candidate = token.trim_matches(punctuation);
    if candidate.is_empty() || (!candidate.contains('/') && !candidate.contains('.')) {
      continue;
    }

    if workspace_root.join(candidate).is_file() {
      return Some(candidate.replace('\\', "/"));
    }
  }

  None
}

fn infer_write_intent(message: &str) -> Option<WriteIntent> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for prefix in ["write ", "create ", "update "] {
    if lowercased_message.starts_with(prefix) {
      let remainder = &trimmed[prefix.len()..];
      let (path, content) = remainder.split_once(':')?;
      let relative_path = path
        .trim()
        .trim_matches(&['"', '\'', '`'][..])
        .replace('\\', "/");
      let content = content.trim().to_string();

      if relative_path.is_empty() || content.is_empty() {
        return None;
      }

      return Some(WriteIntent {
        relative_path,
        content,
      });
    }
  }

  None
}

fn infer_shell_command(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for prefix in ["run shell:", "shell:", "run command:"] {
    if lowercased_message.starts_with(prefix) {
      let command = trimmed[prefix.len()..].trim();
      if !command.is_empty() {
        return Some(command.to_string());
      }
    }
  }

  None
}

fn infer_search_query(message: &str) -> Option<String> {
  let trimmed = message.trim();
  let lowercased_message = trimmed.to_lowercase();

  for keyword in ["search for ", "find ", "search "] {
    if let Some(index) = lowercased_message.find(keyword) {
      let query = trimmed[index + keyword.len()..]
        .trim()
        .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
      if !query.is_empty() {
        return Some(query.to_string());
      }
    }
  }

  if lowercased_message.contains("grep ") {
    let query = trimmed
      .split_once("grep ")
      .map(|(_, remainder)| remainder.trim())
      .unwrap_or_default()
      .trim_matches(&['"', '\'', '.', '?', '!', '`'][..]);
    if !query.is_empty() {
      return Some(query.to_string());
    }
  }

  None
}

fn refresh_thread_summary_note(context: &mut RuntimeContext, thread_id: &str) -> Result<()> {
  let Some(thread) = context
    .threads
    .iter()
    .find(|thread| thread.summary.id == thread_id)
    .cloned()
  else {
    return Ok(());
  };

  let pending_approvals = approvals_for_thread(context, thread_id);
  let workspace_snapshot = thread.workspace.clone();
  let scope = thread
    .workspace
    .as_ref()
    .map(|workspace| workspace.display_name.clone())
    .unwrap_or_else(|| "global".to_string());

  context.upsert_memory_note(
    format!("memory-thread-summary-{thread_id}"),
    format!("Thread summary: {}", thread.summary.title),
    build_thread_summary_body(&thread, workspace_snapshot.as_ref(), &pending_approvals),
    scope,
    "thread".to_string(),
    vec![
      "thread".to_string(),
      "summary".to_string(),
      thread_id.to_string(),
    ],
  )?;

  Ok(())
}

fn build_thread_summary_body(
  thread: &StoredThread,
  workspace: Option<&WorkspaceSummary>,
  pending_approvals: &[ApprovalRequest],
) -> String {
  let workspace_line = workspace
    .map(|workspace| format!("Workspace: {}.", workspace.display_name))
    .unwrap_or_else(|| "Workspace: unavailable.".to_string());
  let latest_user = thread
    .items
    .iter()
    .rev()
    .find(|item| item.kind == "userMessage")
    .map(|item| truncate_memory_line(&item.content, 180))
    .unwrap_or_else(|| "No user request captured yet.".to_string());
  let latest_assistant = thread
    .items
    .iter()
    .rev()
    .find(|item| item.kind == "assistantMessage")
    .map(|item| truncate_memory_line(&item.content, 180))
    .unwrap_or_else(|| "No assistant update captured yet.".to_string());
  let recent_activity = thread
    .items
    .iter()
    .rev()
    .filter(|item| item.kind != "system")
    .take(4)
    .map(|item| item.title.clone())
    .collect::<Vec<_>>();
  let activity_line = if recent_activity.is_empty() {
    "Recent activity: none yet.".to_string()
  } else {
    format!("Recent activity: {}.", recent_activity.join(", "))
  };

  format!(
    "{workspace_line}\nStatus: {}.\nLast user request: {}.\nLatest assistant update: {}.\nPending approvals: {}.\n{}",
    thread.summary.status,
    latest_user,
    latest_assistant,
    pending_approvals.len(),
    activity_line
  )
}

fn truncate_memory_line(content: &str, limit: usize) -> String {
  let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
  let character_count = normalized.chars().count();
  if character_count <= limit {
    return normalized;
  }

  let truncated = normalized
    .chars()
    .take(limit.saturating_sub(3))
    .collect::<String>();
  format!("{truncated}...")
}

fn maybe_start_streaming_assistant_turn(
  thread_id: &str,
  turn_id: &str,
  items: &mut Vec<TimelineItem>,
  full_content: String,
  mut attributes: HashMap<String, String>,
) -> Option<ActiveTurn> {
  let initial_chars = 48.min(full_content.chars().count());
  let total_chars = full_content.chars().count();
  let initial_content = take_characters(&full_content, initial_chars);
  let is_complete = initial_chars >= total_chars;
  let streaming_status = if is_complete {
    "completed"
  } else {
    "in_progress"
  };
  attributes.insert("turnId".to_string(), turn_id.to_string());
  attributes.insert("streamingStatus".to_string(), streaming_status.to_string());
  attributes.insert("streamedCharacters".to_string(), initial_chars.to_string());
  attributes.insert("totalCharacters".to_string(), total_chars.to_string());
  attributes.insert("responseRole".to_string(), "summarizer".to_string());

  items.push(TimelineItem {
    kind: "assistantMessage".to_string(),
    title: "Assistant".to_string(),
    content: initial_content,
    attributes: Some(attributes),
  });

  if is_complete {
    return None;
  }

  Some(ActiveTurn {
    id: turn_id.to_string(),
    thread_id: thread_id.to_string(),
    full_content,
    emitted_chars: initial_chars,
    total_chars,
    started_at: Instant::now(),
  })
}

fn refresh_active_turn_for_thread(context: &mut RuntimeContext, thread_id: &str) -> Result<bool> {
  let active_turn_ids = context
    .active_turns
    .values()
    .filter(|turn| turn.thread_id == thread_id)
    .map(|turn| turn.id.clone())
    .collect::<Vec<_>>();
  let mut did_update = false;

  for turn_id in active_turn_ids {
    if advance_active_turn(context, &turn_id)?.is_some() {
      did_update = true;
    }
  }

  Ok(did_update)
}

fn advance_active_turn(
  context: &mut RuntimeContext,
  turn_id: &str,
) -> Result<Option<ThreadUpdatedNotificationParams>> {
  let Some(snapshot) = context.active_turns.get(turn_id).cloned() else {
    return Ok(None);
  };
  let target_chars = compute_streamed_char_count(&snapshot).min(snapshot.total_chars);

  if target_chars <= snapshot.emitted_chars {
    return Ok(None);
  }

  let thread_id = snapshot.thread_id.clone();
  let streamed_content = take_characters(&snapshot.full_content, target_chars);
  let is_complete = target_chars >= snapshot.total_chars;
  let streaming_status = if is_complete {
    "completed"
  } else {
    "in_progress"
  };

  let thread_snapshot = {
    let Some(thread) = context
      .threads
      .iter_mut()
      .find(|thread| thread.summary.id == snapshot.thread_id)
    else {
      return Ok(None);
    };

    update_streaming_item(
      thread,
      turn_id,
      &streamed_content,
      streaming_status,
      target_chars,
      snapshot.total_chars,
    );

    if is_complete {
      thread.summary.status = "Ready".to_string();
    } else {
      thread.summary.status = format!(
        "Streaming assistant response ({})",
        streaming_progress_label(target_chars, snapshot.total_chars)
      );
    }

    (thread.summary.clone(), thread.items.clone())
  };

  if is_complete {
    context.active_turns.remove(turn_id);
    refresh_thread_summary_note(context, &thread_id)?;
  } else if let Some(active_turn) = context.active_turns.get_mut(turn_id) {
    active_turn.emitted_chars = target_chars;
  }

  Ok(Some(ThreadUpdatedNotificationParams {
    thread: thread_snapshot.0,
    items: thread_snapshot.1,
    pending_approvals: approvals_for_thread(context, &thread_id),
    active_turn_id: active_turn_id_for_thread(context, &thread_id),
  }))
}

fn compute_streamed_char_count(turn: &ActiveTurn) -> usize {
  let elapsed_steps = (turn.started_at.elapsed().as_millis() / 180) as usize;
  let base_chars = 48;
  let step_chars = 72;

  base_chars + elapsed_steps * step_chars
}

fn update_streaming_item(
  thread: &mut StoredThread,
  turn_id: &str,
  content: &str,
  streaming_status: &str,
  streamed_chars: usize,
  total_chars: usize,
) {
  let Some(item) = thread.items.iter_mut().rev().find(|item| {
    item.kind == "assistantMessage"
      && item
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get("turnId"))
        .map(|value| value == turn_id)
        .unwrap_or(false)
  }) else {
    return;
  };

  item.content = content.to_string();
  let mut attributes = item.attributes.clone().unwrap_or_default();
  attributes.insert("turnId".to_string(), turn_id.to_string());
  attributes.insert("streamingStatus".to_string(), streaming_status.to_string());
  attributes.insert("streamedCharacters".to_string(), streamed_chars.to_string());
  attributes.insert("totalCharacters".to_string(), total_chars.to_string());
  item.attributes = Some(attributes);
}

fn active_turn_id_for_thread(context: &RuntimeContext, thread_id: &str) -> Option<String> {
  context
    .active_turns
    .values()
    .find(|turn| turn.thread_id == thread_id)
    .map(|turn| turn.id.clone())
}

fn streaming_progress_label(streamed_chars: usize, total_chars: usize) -> String {
  if total_chars == 0 {
    return "0%".to_string();
  }

  let percentage = ((streamed_chars as f64 / total_chars as f64) * 100.0).round() as usize;
  format!("{}%", percentage.min(100))
}

fn build_plan_item(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  message: &str,
  workspace: Option<&WorkspaceSummary>,
  plan_hint: String,
) -> TimelineItem {
  let context_pack = pack_memory_context(
    memory_notes,
    workspace.map(|entry| entry.display_name.as_str()),
    message,
  );
  let workspace_context = workspace
    .map(|workspace| {
      format!(
        "Workspace: {} at {}.",
        workspace.display_name, workspace.root_path
      )
    })
    .unwrap_or_else(|| "Workspace: unavailable.".to_string());
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Planner,
    prompt: format!(
      "You are the local planner for Pith.\n{}\n{}\nUser request: {}\nCandidate local action: {}\nWrite one concise English sentence describing the next action Pith should take.",
      workspace_context,
      format_memory_prompt(&context_pack.notes),
      message,
      plan_hint
    ),
    max_tokens: 80,
  });
  let mut attributes = HashMap::from([
    ("responseRole".to_string(), "planner".to_string()),
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  if let Some(workspace) = workspace {
    attributes.insert(
      "workspaceDisplayName".to_string(),
      workspace.display_name.clone(),
    );
  }
  merge_context_pack_attributes(&mut attributes, &context_pack);

  TimelineItem {
    kind: "plan".to_string(),
    title: "Plan".to_string(),
    content: result.text,
    attributes: Some(attributes),
  }
}

fn take_characters(content: &str, count: usize) -> String {
  content.chars().take(count).collect()
}

fn format_file_result(result: &ReadFileResult) -> String {
  if result.is_truncated {
    format!(
      "File: {}\n\n{}\n\n[output truncated at 4096 bytes]",
      result.relative_path, result.content
    )
  } else {
    format!("File: {}\n\n{}", result.relative_path, result.content)
  }
}

fn summarize_file_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  result: &ReadFileResult,
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(
    memory_notes,
    Some(workspace_name),
    &format!("{thread_title} {}", result.relative_path),
  );
  let preview = result
    .content
    .lines()
    .find(|line| !line.trim().is_empty())
    .unwrap_or("The file is empty.");

  let observation_summary = format!(
    "Pith inspected {} for {} in {}. First useful line: {}",
    result.relative_path, thread_title, workspace_name, preview
  );
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a file inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nFile: {}\nPreview:\n{}",
    format_memory_prompt(&context_pack.notes),
    result.relative_path,
    result.content
  );

  generate_local_summary(model_runtime, prompt, observation_summary, &context_pack)
}

fn format_directory_result(entries: &[DirectoryEntry]) -> String {
  if entries.is_empty() {
    return "The directory is empty.".to_string();
  }

  entries
    .iter()
    .map(|entry| format!("[{}] {}", entry.entry_type, entry.relative_path))
    .collect::<Vec<_>>()
    .join("\n")
}

fn format_search_result(query: &str, matches: &[SearchMatch]) -> String {
  if matches.is_empty() {
    return format!("No matches found for \"{}\".", query);
  }

  matches
    .iter()
    .map(|entry| {
      format!(
        "{}:{}: {}",
        entry.relative_path, entry.line_number, entry.line
      )
    })
    .collect::<Vec<_>>()
    .join("\n")
}

fn summarize_directory_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  entries: &[DirectoryEntry],
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(
    memory_notes,
    Some(workspace_name),
    &format!("{thread_title} workspace root"),
  );
  if entries.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Pith, a concise local coding agent. Summarize an empty workspace root inspection.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}",
        format_memory_prompt(&context_pack.notes)
      ),
      format!(
        "Pith inspected {} for {} and found an empty root directory.",
        workspace_name, thread_title
      ),
      &context_pack,
    );
  }

  let preview = entries
    .iter()
    .take(5)
    .map(|entry| entry.name.clone())
    .collect::<Vec<_>>()
    .join(", ");

  let observation_summary = format!(
    "Pith inspected {} for {} and found {} root entries, including {}.",
    workspace_name,
    thread_title,
    entries.len(),
    preview
  );
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a root directory inspection in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nEntries:\n{}",
    format_memory_prompt(&context_pack.notes),
    format_directory_result(entries)
  );

  generate_local_summary(model_runtime, prompt, observation_summary, &context_pack)
}

fn summarize_search_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  thread_title: &str,
  workspace_name: &str,
  query: &str,
  matches: &[SearchMatch],
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(memory_notes, Some(workspace_name), query);
  if matches.is_empty() {
    return generate_local_summary(
      model_runtime,
      format!(
        "You are Pith, a concise local coding agent. Summarize a search with no matches.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}",
        format_memory_prompt(&context_pack.notes)
      ),
      format!(
        "Pith searched {} for {} and found no matches for \"{}\".",
        workspace_name, thread_title, query
      ),
      &context_pack,
    );
  }

  let preview = matches
    .iter()
    .take(3)
    .map(|entry| format!("{}:{}", entry.relative_path, entry.line_number))
    .collect::<Vec<_>>()
    .join(", ");

  let observation_summary = format!(
    "Pith searched {} for {} and found {} matches for \"{}\", including {}.",
    workspace_name,
    thread_title,
    matches.len(),
    query,
    preview
  );
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a workspace search in one or two sentences.\nThread: {thread_title}\nWorkspace: {workspace_name}\n{}\nQuery: {query}\nMatches:\n{}",
    format_memory_prompt(&context_pack.notes),
    format_search_result(query, matches)
  );

  generate_local_summary(model_runtime, prompt, observation_summary, &context_pack)
}

fn format_shell_result(result: &ShellCommandResult) -> String {
  let stdout = if result.stdout.trim().is_empty() {
    "[no stdout]".to_string()
  } else {
    result.stdout.clone()
  };
  let stderr = if result.stderr.trim().is_empty() {
    "[no stderr]".to_string()
  } else {
    result.stderr.clone()
  };
  let truncation_note = if result.was_truncated {
    "\n\n[output truncated]"
  } else {
    ""
  };

  format!(
    "Command: {}\nExit Code: {}\n\nstdout:\n{}\n\nstderr:\n{}{}",
    result.command, result.exit_code, stdout, stderr, truncation_note
  )
}

fn summarize_shell_result(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  result: &ShellCommandResult,
) -> (String, HashMap<String, String>) {
  let context_pack = pack_memory_context(memory_notes, Some(workspace_name), &result.command);
  let observation_summary = if result.exit_code == 0 {
    format!(
      "Pith ran `{}` in {} and it finished successfully.",
      result.command, workspace_name
    )
  } else {
    format!(
      "Pith ran `{}` in {} and it exited with code {}.",
      result.command, workspace_name, result.exit_code
    )
  };
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a shell command result in one or two sentences.\nWorkspace: {workspace_name}\n{}\nCommand: {}\nExit Code: {}\nstdout:\n{}\n\nstderr:\n{}",
    format_memory_prompt(&context_pack.notes),
    result.command,
    result.exit_code,
    result.stdout,
    result.stderr
  );

  generate_local_summary(model_runtime, prompt, observation_summary, &context_pack)
}

fn summarize_denied_approval(
  model_runtime: &LocalModelRuntime,
  memory_notes: &[MemoryNote],
  workspace_name: &str,
  approval: &PendingApproval,
) -> (String, HashMap<String, String>) {
  let query = approval
    .command
    .clone()
    .unwrap_or_else(|| format!("{} {}", approval.action, approval.relative_path));
  let context_pack = pack_memory_context(memory_notes, Some(workspace_name), &query);
  let observation_summary = if approval.action == "run_shell" {
    let command = approval.command.clone().unwrap_or_default();
    format!(
      "Pith skipped the shell command `{}` because the approval was denied.",
      command
    )
  } else {
    format!(
      "Pith skipped writing {} because the approval was denied.",
      approval.relative_path
    )
  };
  let prompt = format!(
    "You are Pith, a concise local coding agent. Summarize a denied approval in one sentence.\nWorkspace: {workspace_name}\n{}\nAction: {}\nTarget: {}\nCommand: {}",
    format_memory_prompt(&context_pack.notes),
    approval.action,
    approval.relative_path,
    approval.command.clone().unwrap_or_default()
  );

  generate_local_summary(model_runtime, prompt, observation_summary, &context_pack)
}

fn generate_local_summary(
  model_runtime: &LocalModelRuntime,
  prompt: String,
  observation_summary: String,
  context_pack: &ContextPack,
) -> (String, HashMap<String, String>) {
  let result = model_runtime.generate(GenerateRequest {
    role: ModelRole::Summarizer,
    prompt: format!("{prompt}\nDeterministic observation:\n{observation_summary}"),
    max_tokens: 160,
  });

  let mut attributes = HashMap::from([
    ("modelId".to_string(), result.model_id),
    ("modelBackend".to_string(), result.backend),
    ("modelStatus".to_string(), result.status),
  ]);
  merge_context_pack_attributes(&mut attributes, context_pack);

  (result.text, attributes)
}

fn pack_memory_context(
  memory_notes: &[MemoryNote],
  workspace_scope: Option<&str>,
  query: &str,
) -> ContextPack {
  pack_relevant_memory_notes(memory_notes, workspace_scope, query)
}

fn format_memory_prompt(memory_notes: &[MemoryNote]) -> String {
  if memory_notes.is_empty() {
    return "Memory: none.".to_string();
  }

  let note_lines = memory_notes
    .iter()
    .map(|note| {
      format!(
        "- {} | scope={} | source={} | {}",
        note.title, note.scope, note.source, note.body
      )
    })
    .collect::<Vec<_>>()
    .join("\n");

  format!("Relevant memory notes:\n{note_lines}")
}

fn merge_memory_attributes(attributes: &mut HashMap<String, String>, memory_notes: &[MemoryNote]) {
  attributes.insert(
    "memoryNoteCount".to_string(),
    memory_notes.len().to_string(),
  );
  if memory_notes.is_empty() {
    return;
  }

  attributes.insert(
    "memoryNoteIds".to_string(),
    memory_notes
      .iter()
      .map(|note| note.id.clone())
      .collect::<Vec<_>>()
      .join(", "),
  );
  attributes.insert(
    "memoryNoteTitles".to_string(),
    memory_notes
      .iter()
      .map(|note| note.title.clone())
      .collect::<Vec<_>>()
      .join(" | "),
  );
}

fn merge_context_pack_attributes(
  attributes: &mut HashMap<String, String>,
  context_pack: &ContextPack,
) {
  merge_memory_attributes(attributes, &context_pack.notes);
  attributes.insert("contextMode".to_string(), context_pack.mode().to_string());
  attributes.insert(
    "contextSourceNoteCount".to_string(),
    context_pack.source_note_count.to_string(),
  );
  attributes.insert(
    "contextOmittedNoteCount".to_string(),
    context_pack.omitted_note_count.to_string(),
  );
  attributes.insert(
    "contextTruncatedNoteCount".to_string(),
    context_pack.truncated_note_count.to_string(),
  );
  attributes.insert(
    "contextEstimatedChars".to_string(),
    context_pack.estimated_char_count.to_string(),
  );
  attributes.insert(
    "contextBudgetChars".to_string(),
    context_pack.budget_char_count.to_string(),
  );
}

fn stored_approval_record(approval: PendingApproval) -> StoredApprovalRecord {
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

fn approvals_for_thread(context: &RuntimeContext, thread_id: &str) -> Vec<ApprovalRequest> {
  let mut approvals = context
    .pending_approvals
    .values()
    .filter(|approval| approval.thread_id == thread_id)
    .map(|approval| ApprovalRequest {
      id: approval.id.clone(),
      thread_id: approval.thread_id.clone(),
      action: approval.action.clone(),
      title: approval.title.clone(),
      relative_path: approval.relative_path.clone(),
    })
    .collect::<Vec<_>>();
  approvals.sort_by(|left, right| left.id.cmp(&right.id));
  approvals
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::{json, Value};
  use std::env;
  use std::thread;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn request(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
      id: json!(1),
      method: method.to_string(),
      params,
    }
  }

  fn create_temp_workspace(label: &str) -> PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("system time")
      .as_nanos();
    let path = env::temp_dir().join(format!("pith-{label}-{unique}"));
    fs::create_dir_all(&path).expect("create temp workspace");
    path
  }

  fn create_temp_plugin_bundle(label: &str, plugin_name: &str, display_name: &str) -> PathBuf {
    let root = create_temp_workspace(label);
    let plugin_dir = root.join(plugin_name);
    fs::create_dir_all(plugin_dir.join("commands")).expect("create plugin commands directory");
    fs::write(
      plugin_dir.join("pith-plugin.json"),
      format!(
        r#"{{
  "name": "{plugin_name}",
  "version": "0.1.0",
  "displayName": "{display_name}",
  "description": "Temporary test plugin",
  "author": {{ "name": "Pith" }},
  "capabilities": ["command:{plugin_name}.run"],
  "permissions": ["file.read"],
  "defaultEnabled": true
}}"#
      ),
    )
    .expect("write plugin manifest");
    fs::write(
      plugin_dir
        .join("commands")
        .join(format!("{plugin_name}.run.json")),
      r#"{
  "title": "Run Temporary Plugin",
  "description": "Execute a temporary plugin command.",
  "prompt": "Summarize the local workspace in one paragraph."
}"#,
    )
    .expect("write command manifest");
    plugin_dir
  }

  fn enable_full_access_plugin(context: &mut RuntimeContext) {
    context.plugins = vec![PluginCatalogEntry {
      id: "test-full-access".to_string(),
      name: "test-full-access".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Test Full Access".to_string(),
      status: "ready".to_string(),
      description: "Grants built-in workspace and shell permissions for tests".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["prompt_pack:test.full_access".to_string()],
      permissions: vec![
        "file.read".to_string(),
        "file.write".to_string(),
        "shell.exec".to_string(),
      ],
      manifest_path: "tests/test-full-access/pith-plugin.json".to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }];
  }

  #[test]
  fn initialize_request_returns_capabilities() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(
      &mut context,
      request(
        methods::INITIALIZE,
        Some(json!({
          "clientInfo": {
            "name": "pith-tests",
            "version": "0.1.0"
          }
        })),
      ),
    );

    assert!(response.error.is_none());
    let result = response.result.expect("initialize result");
    assert_eq!(result["protocolVersion"], "0.1.0");
    assert_eq!(result["capabilities"]["supportsThreads"], true);
    assert_eq!(result["capabilities"]["supportsTools"], true);
  }

  #[test]
  fn health_ping_returns_ok() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request(methods::HEALTH_PING, None));

    assert!(response.error.is_none());
    let result = response.result.expect("health result");
    assert_eq!(result["status"], "ok");
  }

  #[test]
  fn model_health_returns_local_model_status() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request(methods::MODEL_HEALTH, None));

    assert!(response.error.is_none());
    let result = response.result.expect("model health result");
    assert_eq!(result["displayName"], "LFM2.5-350M");
    assert!(result["backend"].is_string());
    assert!(result["status"].is_string());
  }

  #[test]
  fn unknown_method_returns_json_rpc_error() {
    let mut context = RuntimeContext::new_in_memory();
    let response = handle_request(&mut context, request("unknown/method", None));

    assert!(response.result.is_none());
    let error = response.error.expect("error payload");
    assert_eq!(error.code, -32601);
  }

  #[test]
  fn workspace_open_sets_runtime_workspace() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("open");

    let response = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(response.error.is_none());
    let result = response.result.expect("workspace open result");
    assert_eq!(
      result["workspace"]["displayName"].as_str().unwrap(),
      workspace.file_name().unwrap().to_string_lossy()
    );
  }

  #[test]
  fn workspace_search_returns_matching_lines() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("workspace-search");
    fs::write(
      workspace.join("README.md"),
      "Pith local search\nNothing else\n",
    )
    .expect("write searchable file");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );

    let response = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_SEARCH,
        Some(json!({
          "query": "local",
          "maxResults": 8
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(response.error.is_none());
    let result = response.result.expect("workspace search result");
    let matches = result["matches"].as_array().expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0]["relativePath"], "README.md");
    assert_eq!(matches[0]["lineNumber"], 1);
  }

  #[test]
  fn memory_create_adds_manual_workspace_note() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("memory-create");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );

    let create_response = handle_request(
      &mut context,
      request(
        methods::MEMORY_CREATE,
        Some(json!({
          "title": "Repository preference",
          "body": "Prefer small, reviewable patches.",
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(create_response.error.is_none());
    let result = create_response.result.expect("memory create result");
    assert_eq!(result["note"]["title"], "Repository preference");
    assert_eq!(result["note"]["source"], "user");
    assert_eq!(context.memory_notes.len(), 2);
    assert_eq!(context.memory_notes[0].title, "Repository preference");
  }

  #[test]
  fn thread_start_persists_thread_for_future_lists() {
    let mut context = RuntimeContext::new_in_memory();

    let start_response = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "First Thread"
        })),
      ),
    );
    assert!(start_response.error.is_none());

    let list_response = handle_request(&mut context, request(methods::THREAD_LIST, None));
    let result = list_response.result.expect("thread list result");
    let threads = result["threads"].as_array().expect("thread array");

    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0]["title"], "First Thread");
  }

  #[test]
  fn thread_read_returns_persisted_thread_items() {
    let mut context = RuntimeContext::new_in_memory();

    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Inspectable Thread"
        })),
      ),
    );

    let read_response = handle_request(
      &mut context,
      request(
        methods::THREAD_READ,
        Some(json!({
          "threadId": "thread-1"
        })),
      ),
    );

    assert!(read_response.error.is_none());
    let result = read_response.result.expect("thread read result");
    let items = result["items"].as_array().expect("thread items");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["kind"], "system");
  }

  #[test]
  fn turn_start_warns_when_workspace_is_missing() {
    let mut context = RuntimeContext::new_in_memory();

    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Chat Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Inspect the project"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items[0]["kind"], "userMessage");
    assert_eq!(items[1]["kind"], "plan");
    assert_eq!(items[2]["kind"], "warning");
  }

  #[test]
  fn turn_start_reads_a_requested_workspace_file() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("read-file");
    fs::write(
      workspace.join("README.md"),
      "# Milestone 1\nWorkspace tool test\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Workspace Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items[1]["kind"], "plan");
    assert_eq!(items[1]["attributes"]["responseRole"], "planner");
    assert_eq!(items[1]["attributes"]["memoryNoteCount"], "1");
    assert_eq!(items[1]["attributes"]["memoryNoteIds"], "memory-1");
    assert_eq!(items[2]["kind"], "toolStart");
    assert_eq!(items[3]["kind"], "toolResult");
    assert_eq!(items[4]["kind"], "assistantMessage");
    assert_eq!(items[4]["attributes"]["responseRole"], "summarizer");
    assert_eq!(items[4]["attributes"]["memoryNoteCount"], "1");
    assert!(items[4]["attributes"]["memoryNoteTitles"]
      .as_str()
      .unwrap()
      .contains("Opened workspace"));
    assert!(matches!(
      items[4]["attributes"]["streamingStatus"].as_str(),
      Some("in_progress") | Some("completed")
    ));
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("Milestone 1"));
    assert_eq!(result["activeTurnId"].as_str().unwrap(), "thread-1-turn-1");
  }

  #[test]
  fn collect_notifications_emits_thread_update_for_active_turn() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("thread-updated");
    fs::write(
      workspace.join("README.md"),
      "# Pith\nNotification coverage\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Notification Thread"
        })),
      ),
    );
    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    thread::sleep(std::time::Duration::from_millis(260));

    let notifications = collect_notifications(&mut context).expect("collect notifications");

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert_eq!(notifications.len(), 1);
    assert_eq!(
      notifications[0].method,
      methods::THREAD_UPDATED_NOTIFICATION
    );
    let params = notifications[0]
      .params
      .as_ref()
      .expect("notification params");
    let items = params["items"].as_array().expect("notification items");
    let assistant_item = items
      .iter()
      .rev()
      .find(|item| item["kind"] == "assistantMessage")
      .expect("assistant message");
    assert!(
      assistant_item["attributes"]["streamedCharacters"]
        .as_str()
        .expect("streamed chars")
        .parse::<usize>()
        .expect("streamed chars usize")
        > 0
    );
  }

  #[test]
  fn turn_cancel_stops_an_active_assistant_response() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("turn-cancel");
    fs::write(
      workspace.join("README.md"),
      "# Milestone 1\nStreaming turn content\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Streaming Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );
    let turn_result = turn_response.result.expect("turn result");
    assert_eq!(turn_result["activeTurnId"], "thread-1-turn-1");

    let cancel_response = handle_request(
      &mut context,
      request(
        methods::TURN_CANCEL,
        Some(json!({
          "turnId": "thread-1-turn-1"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(cancel_response.error.is_none());
    let cancel_result = cancel_response.result.expect("cancel result");
    let items = cancel_result["items"].as_array().expect("cancel items");

    assert_eq!(items[0]["title"], "Turn Cancelled");
    assert_eq!(cancel_result["activeTurnId"], serde_json::Value::Null);
  }

  #[test]
  fn turn_start_searches_workspace_content() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("search-files");
    fs::write(
      workspace.join("README.md"),
      "# Pith\nSearch target lives here\n",
    )
    .expect("write readme");
    fs::create_dir_all(workspace.join("docs")).expect("create docs directory");
    fs::write(
      workspace.join("docs").join("notes.txt"),
      "Another Search target appears in docs\n",
    )
    .expect("write notes");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Search Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Find Search target"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");

    assert_eq!(items[2]["kind"], "toolStart");
    assert_eq!(items[2]["title"], "search_files");
    assert_eq!(items[3]["kind"], "toolResult");
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("README.md:2"));
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("docs/notes.txt:1"));
  }

  #[test]
  fn approval_respond_writes_file_after_approval() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("approval-write");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Approval Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Approval protected content"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let turn_result = turn_response.result.expect("turn result");
    let turn_items = turn_result["items"].as_array().expect("turn items");
    assert_eq!(turn_items[2]["title"], "generate_diff");
    assert_eq!(turn_items[3]["kind"], "diffArtifact");
    assert!(turn_items[3]["content"]
      .as_str()
      .unwrap()
      .contains("+++ b/docs/output.txt"));
    assert_eq!(turn_items[4]["kind"], "approvalRequested");
    let approval_id = turn_result["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );

    let written_content =
      fs::read_to_string(workspace.join("docs").join("output.txt")).expect("read written output");
    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(approval_response.error.is_none());
    let approval_result = approval_response.result.expect("approval result");
    let items = approval_result["items"].as_array().expect("approval items");

    assert_eq!(items[0]["kind"], "approvalResolved");
    assert_eq!(items[1]["title"], "write_file");
    assert_eq!(written_content, "Approval protected content");
  }

  #[test]
  fn approval_respond_runs_shell_after_approval() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("approval-shell");
    context.plugins = vec![PluginCatalogEntry {
      id: "shell-recorder".to_string(),
      name: "shell-recorder".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Shell Recorder".to_string(),
      status: "ready".to_string(),
      description: "Shell access plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: false,
      capabilities: vec![
        "hook:shell.recorder".to_string(),
        "tool:shell.timeline".to_string(),
      ],
      permissions: vec!["shell.exec".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/shell-recorder/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];
    fs::write(workspace.join("marker.txt"), "shell target\n").expect("write shell marker");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Shell Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Run shell: ls"
        })),
      ),
    );

    assert!(turn_response.error.is_none());
    let turn_result = turn_response.result.expect("turn result");
    let approval_id = turn_result["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(approval_response.error.is_none());
    let approval_result = approval_response.result.expect("approval result");
    let items = approval_result["items"].as_array().expect("approval items");

    assert_eq!(items[1]["title"], "run_shell");
    assert!(items[2]["content"].as_str().unwrap().contains("marker.txt"));
    assert!(items.iter().any(|item| item["kind"] == "pluginHook"));
    assert!(items.iter().any(|item| {
      item["title"] == "Record Shell Completion"
        && item["attributes"]["hookEvent"] == "shell.completed"
    }));
    assert!(items.iter().any(|item| {
      item["title"] == "Hook Memory Note Saved"
        && item["attributes"]["memoryNoteTitle"] == "Shell Completion"
    }));
    assert!(context
      .memory_notes
      .iter()
      .any(|note| note.title == "Shell Completion" && note.source == "plugin.shell-recorder"));
  }

  #[test]
  fn thread_summary_memory_note_is_updated_after_approval_resolution() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("thread-summary");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Summary Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Summary content"
        })),
      ),
    );
    let approval_id = turn_response.result.expect("turn result")["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(approval_response.error.is_none());
    let summary_note = context
      .memory_notes
      .iter()
      .find(|note| note.id == "memory-thread-summary-thread-1")
      .expect("thread summary note");
    assert_eq!(summary_note.source, "thread");
    assert_eq!(summary_note.title, "Thread summary: Summary Thread");
    assert!(summary_note.body.contains("docs/output.txt"));
  }

  #[test]
  fn follow_up_turn_retrieves_recent_memory_notes() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace = create_temp_workspace("memory-follow-up");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Memory Thread"
        })),
      ),
    );

    let write_turn = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Write docs/output.txt: Memory connected content"
        })),
      ),
    );
    let approval_id = write_turn.result.expect("write turn result")["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );
    assert!(approval_response.error.is_none());

    let follow_up_turn = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read docs/output.txt"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(follow_up_turn.error.is_none());
    let items = follow_up_turn.result.expect("follow-up turn result")["items"]
      .as_array()
      .expect("follow-up items")
      .clone();

    assert_eq!(items[1]["attributes"]["memoryNoteCount"], "3");
    assert!(items[1]["attributes"]["memoryNoteTitles"]
      .as_str()
      .unwrap()
      .contains("Wrote docs/output.txt"));
    assert!(items[4]["attributes"]["memoryNoteTitles"]
      .as_str()
      .unwrap()
      .contains("Wrote docs/output.txt"));
    assert_eq!(items[4]["attributes"]["memoryNoteCount"], "3");
  }

  #[test]
  fn thread_turns_stay_bound_to_the_thread_workspace() {
    let mut context = RuntimeContext::new_in_memory();
    enable_full_access_plugin(&mut context);
    let workspace_a = create_temp_workspace("workspace-a");
    let workspace_b = create_temp_workspace("workspace-b");
    fs::write(
      workspace_a.join("README.md"),
      "# Workspace A\nThread-bound content\n",
    )
    .expect("write workspace a readme");
    fs::write(
      workspace_b.join("README.md"),
      "# Workspace B\nDifferent content\n",
    )
    .expect("write workspace b readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace_a.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Workspace Bound Thread"
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace_b.display().to_string()
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );

    fs::remove_dir_all(&workspace_a).expect("cleanup workspace a");
    fs::remove_dir_all(&workspace_b).expect("cleanup workspace b");

    assert!(turn_response.error.is_none());
    let result = turn_response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("Workspace A"));
    assert!(items[3]["content"]
      .as_str()
      .unwrap()
      .contains("Thread-bound content"));
  }

  #[test]
  fn plugin_set_enabled_updates_runtime_catalog() {
    let mut context = RuntimeContext::new_in_memory();
    context.plugins = vec![PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "Test plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: false,
      default_enabled: false,
      capabilities: vec!["prompt_pack:workspace.notes".to_string()],
      permissions: vec!["file.read".to_string()],
      manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_SET_ENABLED,
        Some(json!({
          "pluginId": "workspace-notes",
          "enabled": true
        })),
      ),
    );

    assert!(response.error.is_none());
    assert!(context.plugins[0].enabled);
    assert_eq!(
      response.result.expect("plugin set result")["plugin"]["enabled"],
      true
    );
  }

  #[test]
  fn plugin_install_adds_local_plugin_to_the_runtime_catalog() {
    let mut context = RuntimeContext::new_in_memory();
    let source_root =
      create_temp_plugin_bundle("plugin-install-source", "focus-review", "Focus Review");
    let install_root = create_temp_workspace("plugin-install-root");
    context.plugin_roots = vec![install_root.clone()];
    context.plugin_install_root = install_root.clone();
    context.plugins = vec![];

    let response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_INSTALL,
        Some(json!({
          "sourcePath": source_root.display().to_string()
        })),
      ),
    );

    fs::remove_dir_all(source_root.parent().expect("plugin source root"))
      .expect("cleanup plugin source root");
    fs::remove_dir_all(&install_root).expect("cleanup install root");

    assert!(response.error.is_none());
    let result = response.result.expect("plugin install result");
    assert_eq!(result["plugin"]["id"], "focus-review");
    assert_eq!(result["plugin"]["provenance"], "local");
    assert!(context
      .plugins
      .iter()
      .any(|plugin| plugin.id == "focus-review"));
  }

  #[test]
  fn plugin_install_rejects_duplicate_plugin_ids() {
    let mut context = RuntimeContext::new_in_memory();
    let source_root = create_temp_plugin_bundle(
      "plugin-install-duplicate",
      "workspace-notes",
      "Workspace Notes",
    );
    context.plugins = vec![PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "bundled plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["prompt_pack:workspace.notes".to_string()],
      permissions: vec!["file.read".to_string()],
      manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_INSTALL,
        Some(json!({
          "sourcePath": source_root.display().to_string()
        })),
      ),
    );

    fs::remove_dir_all(source_root.parent().expect("plugin source root"))
      .expect("cleanup plugin source root");

    assert!(response.result.is_none());
    let error = response.error.expect("plugin install error");
    assert!(error.message.contains("already installed"));
  }

  #[test]
  fn plugin_remove_deletes_local_plugin_and_clears_persisted_state() {
    let mut context = RuntimeContext::new_in_memory();
    let storage_root = create_temp_workspace("plugin-remove-storage");
    let source_root =
      create_temp_plugin_bundle("plugin-remove-source", "focus-review", "Focus Review");
    let install_root = create_temp_workspace("plugin-remove-root");
    let store = FileThreadStore::new(
      storage_root.join("pith.db"),
      storage_root.join("threads.json"),
    );
    store
      .save_plugin_enabled("focus-review", true)
      .expect("save persisted plugin state");
    context.store = Some(store);
    context.plugin_roots = vec![install_root.clone()];
    context.plugin_install_root = install_root.clone();
    context.plugins = vec![];

    let install_response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_INSTALL,
        Some(json!({
          "sourcePath": source_root.display().to_string()
        })),
      ),
    );
    assert!(install_response.error.is_none());

    let manifest_path = context.plugins[0].manifest_path.clone();
    let remove_response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_REMOVE,
        Some(json!({
          "manifestPath": manifest_path
        })),
      ),
    );

    let persisted_states = context
      .store
      .as_ref()
      .expect("store")
      .load_plugin_states()
      .expect("load plugin states");

    fs::remove_dir_all(source_root.parent().expect("plugin source root"))
      .expect("cleanup plugin source root");
    fs::remove_dir_all(&install_root).expect("cleanup install root");
    fs::remove_dir_all(&storage_root).expect("cleanup storage root");

    assert!(remove_response.error.is_none());
    let result = remove_response.result.expect("plugin remove result");
    assert_eq!(result["pluginId"], "focus-review");
    assert!(context.plugins.is_empty());
    assert!(!persisted_states.contains_key("focus-review"));
  }

  #[test]
  fn plugin_command_registry_lists_enabled_command_plugins() {
    let mut context = RuntimeContext::new_in_memory();
    context.plugins = vec![PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "Command-enabled plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:workspace.capture-note".to_string(),
        "prompt_pack:workspace.notes".to_string(),
      ],
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/workspace-notes/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let response = handle_request(
      &mut context,
      request(methods::PLUGIN_COMMAND_REGISTRY, None),
    );

    assert!(response.error.is_none());
    let result = response.result.expect("command registry result");
    let commands = result["commands"].as_array().expect("commands");
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0]["pluginId"], "workspace-notes");
    assert_eq!(commands[0]["title"], "Capture Workspace Note");
    assert_eq!(commands[0]["executionKind"], "builtin.workspaceReadmeNote");
  }

  #[test]
  fn plugin_hook_registry_lists_enabled_hook_plugins() {
    let mut context = RuntimeContext::new_in_memory();
    context.plugins = vec![PluginCatalogEntry {
      id: "shell-recorder".to_string(),
      name: "shell-recorder".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Shell Recorder".to_string(),
      status: "ready".to_string(),
      description: "Hook-enabled plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: false,
      capabilities: vec![
        "hook:shell.recorder".to_string(),
        "tool:shell.timeline".to_string(),
      ],
      permissions: vec!["shell.exec".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/shell-recorder/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let response = handle_request(&mut context, request(methods::PLUGIN_HOOK_REGISTRY, None));

    assert!(response.error.is_none());
    let result = response.result.expect("hook registry result");
    let hooks = result["hooks"].as_array().expect("hooks");
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["pluginId"], "shell-recorder");
    assert_eq!(hooks[0]["event"], "shell.completed");
    assert_eq!(hooks[0]["title"], "Record Shell Completion");
  }

  #[test]
  fn plugin_connector_registry_lists_disabled_connector_plugins() {
    let mut context = RuntimeContext::new_in_memory();
    context.plugins = vec![PluginCatalogEntry {
      id: "notion-connector".to_string(),
      name: "notion-connector".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Notion Connector".to_string(),
      status: "ready".to_string(),
      description: "Connector plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: false,
      default_enabled: false,
      capabilities: vec![
        "mcp_server:notion".to_string(),
        "connector:notion".to_string(),
      ],
      permissions: vec!["network.outbound".to_string(), "mcp.connect".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/notion-connector/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let response = handle_request(
      &mut context,
      request(methods::PLUGIN_CONNECTOR_REGISTRY, None),
    );

    assert!(response.error.is_none());
    let result = response.result.expect("connector registry result");
    let connectors = result["connectors"].as_array().expect("connectors");
    assert_eq!(connectors.len(), 1);
    assert_eq!(connectors[0]["connectorId"], "notion-connector::notion");
    assert_eq!(connectors[0]["status"], "disabled");
    assert_eq!(connectors[0]["authType"], "oauth2");
    assert_eq!(connectors[0]["credentialStore"], "keychain");
  }

  #[test]
  fn plugin_command_run_executes_builtin_command_for_the_selected_thread() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("plugin-command-run");
    context.plugins = vec![PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "Command-enabled plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec![
        "command:workspace.capture-note".to_string(),
        "prompt_pack:workspace.notes".to_string(),
      ],
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/bundled/workspace-notes/pith-plugin.json")
        .display()
        .to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];
    fs::write(
      workspace.join("README.md"),
      "Workspace A\nCommand registry path\n",
    )
    .expect("write readme");

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Plugin Command Thread"
        })),
      ),
    );

    let response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_COMMAND_RUN,
        Some(json!({
          "threadId": "thread-1",
          "commandId": "workspace-notes::workspace.capture-note"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(response.error.is_none());
    let result = response.result.expect("command run result");
    let items = result["items"].as_array().expect("items");
    assert_eq!(items[0]["kind"], "pluginCommand");
    assert_eq!(items[0]["attributes"]["pluginId"], "workspace-notes");
    assert_eq!(items[1]["kind"], "pluginResult");
    assert_eq!(
      items[1]["attributes"]["executionKind"],
      "builtin.workspaceReadmeNote"
    );
    assert!(items[1]["content"]
      .as_str()
      .unwrap()
      .contains("Command registry path"));
    assert_eq!(items[2]["kind"], "assistantMessage");
    let memory_item = items
      .iter()
      .find(|item| item["title"] == "Memory Note Saved")
      .expect("memory note saved item");
    assert_eq!(memory_item["kind"], "system");
    assert_eq!(memory_item["attributes"]["pluginId"], "workspace-notes");
    assert_eq!(result["threadId"], "thread-1");
    assert_eq!(context.memory_notes.len(), 3);
    assert!(context
      .memory_notes
      .iter()
      .any(|note| note.title == "Workspace Capture" && note.source == "plugin.workspace-notes"));
  }

  #[test]
  fn bundled_builtin_plugin_commands_return_owned_results() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("bundled-plugin-results");
    fs::write(workspace.join("README.md"), "# Bundled Plugin Results\n").expect("write readme");
    context.plugins = vec![
      PluginCatalogEntry {
        id: "review-assistant".to_string(),
        name: "review-assistant".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Review Assistant".to_string(),
        status: "ready".to_string(),
        description: "Review plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: true,
        default_enabled: true,
        capabilities: vec!["command:review.inspect-diff".to_string()],
        permissions: vec!["file.read".to_string(), "model.invoke".to_string()],
        manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
          .join("../../plugins/bundled/review-assistant/pith-plugin.json")
          .display()
          .to_string(),
        provenance: "bundled".to_string(),
        validation_error: None,
        validation_hint: None,
      },
      PluginCatalogEntry {
        id: "shell-recorder".to_string(),
        name: "shell-recorder".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Shell Recorder".to_string(),
        status: "ready".to_string(),
        description: "Shell plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: true,
        default_enabled: false,
        capabilities: vec![
          "command:shell.summarize-session".to_string(),
          "hook:shell.recorder".to_string(),
        ],
        permissions: vec!["shell.exec".to_string()],
        manifest_path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
          .join("../../plugins/bundled/shell-recorder/pith-plugin.json")
          .display()
          .to_string(),
        provenance: "bundled".to_string(),
        validation_error: None,
        validation_hint: None,
      },
    ];

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Bundled Plugin Thread"
        })),
      ),
    );

    let review_response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_COMMAND_RUN,
        Some(json!({
          "threadId": "thread-1",
          "commandId": "review-assistant::review.inspect-diff"
        })),
      ),
    );
    let shell_response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_COMMAND_RUN,
        Some(json!({
          "threadId": "thread-1",
          "commandId": "shell-recorder::shell.summarize-session"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(review_response.error.is_none());
    assert!(shell_response.error.is_none());
    let review_result = review_response.result.expect("review result");
    let shell_result = shell_response.result.expect("shell result");
    let review_items = review_result["items"]
      .as_array()
      .expect("review items")
      .clone();
    let shell_items = shell_result["items"]
      .as_array()
      .expect("shell items")
      .clone();
    assert_eq!(review_items[1]["kind"], "pluginResult");
    assert_eq!(
      review_items[1]["attributes"]["executionKind"],
      "builtin.reviewDiffSummary"
    );
    assert_eq!(shell_items[1]["kind"], "pluginResult");
    assert_eq!(
      shell_items[1]["attributes"]["executionKind"],
      "builtin.shellSessionSummary"
    );
  }

  #[test]
  fn plugin_command_run_rejects_commands_without_execution_contract() {
    let mut context = RuntimeContext::new_in_memory();
    let source_root =
      create_temp_plugin_bundle("plugin-command-contract", "prompt-only", "Prompt Only");
    let workspace = create_temp_workspace("plugin-command-contract-workspace");
    let plugin_manifest = source_root.join("pith-plugin.json");
    context.plugins = vec![PluginCatalogEntry {
      id: "prompt-only".to_string(),
      name: "prompt-only".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Prompt Only".to_string(),
      status: "ready".to_string(),
      description: "Prompt-only command plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["command:prompt-only.run".to_string()],
      permissions: vec!["file.read".to_string()],
      manifest_path: plugin_manifest.display().to_string(),
      provenance: "test".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Plugin Contract Thread"
        })),
      ),
    );

    let response = handle_request(
      &mut context,
      request(
        methods::PLUGIN_COMMAND_RUN,
        Some(json!({
          "threadId": "thread-1",
          "commandId": "prompt-only::prompt-only.run"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");
    fs::remove_dir_all(source_root.parent().expect("plugin root")).expect("cleanup plugin source");

    let error = response.error.expect("command contract error");
    assert_eq!(error.code, -32053);
    assert!(error
      .message
      .contains("requires an explicit execution contract"));
  }

  #[test]
  fn file_reads_require_plugin_permission() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("permission-read");
    fs::write(workspace.join("README.md"), "# Permission Gate\n").expect("write readme");
    context.plugins = vec![PluginCatalogEntry {
      id: "shell-recorder".to_string(),
      name: "shell-recorder".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Shell Recorder".to_string(),
      status: "ready".to_string(),
      description: "No file access".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: false,
      default_enabled: false,
      capabilities: vec!["hook:shell.recorder".to_string()],
      permissions: vec!["shell.exec".to_string()],
      manifest_path: "plugins/bundled/shell-recorder/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Permission Thread"
        })),
      ),
    );

    let response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Read README.md"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(response.error.is_none());
    let result = response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");
    assert_eq!(items[2]["title"], "Plugin Permission Required");
    assert_eq!(items[2]["attributes"]["requiredPermission"], "file.read");
    assert_eq!(items[3]["kind"], "assistantMessage");
  }

  #[test]
  fn shell_requests_require_plugin_permission_before_approval() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("permission-shell");
    context.plugins = vec![PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "No shell access".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["prompt_pack:workspace.notes".to_string()],
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Shell Permission Thread"
        })),
      ),
    );

    let response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Run shell: ls"
        })),
      ),
    );

    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(response.error.is_none());
    let result = response.result.expect("turn result");
    let items = result["items"].as_array().expect("items");
    assert_eq!(items[2]["title"], "Plugin Permission Required");
    assert_eq!(items[2]["attributes"]["requiredPermission"], "shell.exec");
    assert!(result["pendingApprovals"]
      .as_array()
      .expect("pending approvals")
      .is_empty());
  }

  #[test]
  fn approval_resolution_rechecks_plugin_permissions() {
    let mut context = RuntimeContext::new_in_memory();
    let workspace = create_temp_workspace("approval-permission-recheck");
    context.plugins = vec![PluginCatalogEntry {
      id: "workspace-notes".to_string(),
      name: "workspace-notes".to_string(),
      version: "0.1.0".to_string(),
      display_name: "Workspace Notes".to_string(),
      status: "ready".to_string(),
      description: "Write access plugin".to_string(),
      author_name: Some("Pith".to_string()),
      enabled: true,
      default_enabled: true,
      capabilities: vec!["prompt_pack:workspace.notes".to_string()],
      permissions: vec!["file.read".to_string(), "file.write".to_string()],
      manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
      provenance: "bundled".to_string(),
      validation_error: None,
      validation_hint: None,
    }];

    let _ = handle_request(
      &mut context,
      request(
        methods::WORKSPACE_OPEN,
        Some(json!({
          "path": workspace.display().to_string()
        })),
      ),
    );
    let _ = handle_request(
      &mut context,
      request(
        methods::THREAD_START,
        Some(json!({
          "title": "Approval Permission Thread"
        })),
      ),
    );

    let turn_response = handle_request(
      &mut context,
      request(
        methods::TURN_START,
        Some(json!({
          "threadId": "thread-1",
          "message": "Write docs/output.txt: gated content"
        })),
      ),
    );
    let approval_id = turn_response.result.expect("turn result")["pendingApprovals"][0]["id"]
      .as_str()
      .expect("approval id")
      .to_string();

    context.plugins[0].enabled = false;

    let approval_response = handle_request(
      &mut context,
      request(
        methods::APPROVAL_RESPOND,
        Some(json!({
          "approvalId": approval_id,
          "decision": "approved"
        })),
      ),
    );

    let written_file = workspace.join("docs").join("output.txt");
    fs::remove_dir_all(&workspace).expect("cleanup temp workspace");

    assert!(approval_response.error.is_none());
    let approval_result = approval_response.result.expect("approval result");
    let items = approval_result["items"].as_array().expect("approval items");
    assert_eq!(items[1]["title"], "Plugin Permission Required");
    assert_eq!(items[1]["attributes"]["requiredPermission"], "file.write");
    assert!(!written_file.exists());
  }

  #[test]
  fn capability_registry_only_includes_ready_enabled_plugins() {
    let plugins = vec![
      PluginCatalogEntry {
        id: "workspace-notes".to_string(),
        name: "workspace-notes".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Workspace Notes".to_string(),
        status: "ready".to_string(),
        description: "Test plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: true,
        default_enabled: true,
        capabilities: vec![
          "prompt_pack:workspace.notes".to_string(),
          "settings:workspace.preferences".to_string(),
        ],
        permissions: vec!["file.read".to_string(), "file.write".to_string()],
        manifest_path: "plugins/bundled/workspace-notes/pith-plugin.json".to_string(),
        provenance: "bundled".to_string(),
        validation_error: None,
        validation_hint: None,
      },
      PluginCatalogEntry {
        id: "shell-recorder".to_string(),
        name: "shell-recorder".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Shell Recorder".to_string(),
        status: "ready".to_string(),
        description: "Disabled plugin".to_string(),
        author_name: Some("Pith".to_string()),
        enabled: false,
        default_enabled: false,
        capabilities: vec!["hook:shell.recorder".to_string()],
        permissions: vec!["shell.exec".to_string()],
        manifest_path: "plugins/bundled/shell-recorder/pith-plugin.json".to_string(),
        provenance: "bundled".to_string(),
        validation_error: None,
        validation_hint: None,
      },
      PluginCatalogEntry {
        id: "broken-plugin".to_string(),
        name: "broken-plugin".to_string(),
        version: "0.1.0".to_string(),
        display_name: "Broken Plugin".to_string(),
        status: "invalid".to_string(),
        description: "Invalid plugin".to_string(),
        author_name: None,
        enabled: false,
        default_enabled: false,
        capabilities: vec![],
        permissions: vec![],
        manifest_path: "plugins/bundled/broken/pith-plugin.json".to_string(),
        provenance: "bundled".to_string(),
        validation_error: Some("plugin capability kind `memory` is not supported".to_string()),
        validation_hint: Some(
          "Use one of the supported capability kinds: command, agent, prompt_pack, hook, tool, mcp_server, skill, connector, settings.".to_string(),
        ),
      },
    ];

    let result = build_protocol_capability_registry(&plugins);

    assert_eq!(result.summary.enabled_plugin_count, 1);
    assert_eq!(result.summary.total_capability_count, 2);
    assert_eq!(
      result.summary.capability_counts_by_kind.get("prompt_pack"),
      Some(&1)
    );
    assert_eq!(
      result.summary.capability_counts_by_kind.get("settings"),
      Some(&1)
    );
    assert_eq!(result.capabilities.len(), 2);
    assert_eq!(result.capabilities[0].kind, "prompt_pack");
    assert_eq!(result.capabilities[0].plugin_id, "workspace-notes");
    assert_eq!(result.capabilities[1].kind, "settings");
  }
}
