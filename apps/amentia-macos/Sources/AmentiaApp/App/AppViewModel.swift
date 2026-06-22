import Combine
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var timelineState: TimelineRuntimeState
  @Published var runtimeConnectionState: RuntimeConnectionStateStore
  @Published var draftMessage: String
  @Published var restoredLocalExecutionDraftMessage: String?
  @Published var workspace: WorkspaceSummary?
  @Published var workspaceSearchState: WorkspaceSearchRuntimeState
  @Published private var localModelReadinessState: LocalModelReadinessState
  @Published var modelDownloadState: LocalModelDownloadRuntimeState
  @Published private var memoryState: MemoryRuntimeState
  @Published private var pluginState: PluginRuntimeState
  @Published var pluginManagerSection: PluginManagerSection
  @Published var localDataResetInProgress: Bool
  @Published var isCheckingLocalModel: Bool
  @Published var selectedLocalExecutionSafetyMode: String {
    didSet {
      AppPreferences.storeLocalExecutionSafetyMode(selectedLocalExecutionSafetyMode)
    }
  }

  let runtimeBridge: RuntimeBridge
  let runtimeLaunchCoordinator = RuntimeLaunchCoordinator()
  let workspaceOpenCoordinator = WorkspaceOpenCoordinator()
  let threadCreationCoordinator = ThreadCreationCoordinator()
  let threadHistoryLoadCoordinator = ThreadHistoryLoadCoordinator()
  let localExecutionRequests = LocalExecutionRequestCoordinator()
  let turnCancellationCoordinator = TurnCancellationCoordinator()
  let runtimeRelaunchCoordinator = RuntimeRelaunchCoordinator()
  let workspaceSearchSession = WorkspaceSearchSession()
  let localModelMetadataCoordinator = LocalModelMetadataCoordinator()
  let localModelActivationCoordinator = LocalModelActivationCoordinator()
  let pluginLifecycleOperations = PluginLifecycleOperationCoordinator()
  let modelDownloadCoordinator: LocalModelDownloadCoordinator
  let localModelDownloadRequestPlanCache = LocalModelDownloadRequestPlanCache()
  let localModelProbeCoordinator = LocalModelProbeCoordinator()

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let launchState = AppLaunchState.make(runtimeBridge: runtimeBridge)

    self.runtimeBridge = runtimeBridge
    self.timelineState = TimelineRuntimeState(welcomeState: launchState.welcomeState)
    self.runtimeConnectionState = RuntimeConnectionStateStore(
      state: runtimeBridge.connectionState,
      detail: launchState.runtimeDetail
    )
    self.draftMessage = ""
    self.restoredLocalExecutionDraftMessage = nil
    self.workspace = nil
    self.workspaceSearchState = WorkspaceSearchRuntimeState()
    self.localModelReadinessState = LocalModelReadinessState(
      models: launchState.localModels,
      selectedSetupModelID: launchState.selectedSetupModelID
    )
    self.modelDownloadState = LocalModelDownloadRuntimeState(
      activeModelID: nil,
      pausedModelID: launchState.pausedDownload?.modelID,
      progress: launchState.modelDownloadProgress
    )
    self.memoryState = MemoryRuntimeState()
    self.pluginState = PluginRuntimeState()
    self.pluginManagerSection = .catalog
    self.localDataResetInProgress = false
    self.isCheckingLocalModel = false
    self.selectedLocalExecutionSafetyMode = AppPreferences.storedLocalExecutionSafetyMode()
    self.modelDownloadCoordinator = LocalModelDownloadCoordinator(
      resumeData: launchState.pausedDownload?.resumeData
    )
    self.runtimeBridge.onThreadUpdated = { [weak self] state in
      Task { @MainActor in
        self?.applyRuntimeThreadUpdate(state)
      }
    }
    self.runtimeBridge.onConnectionStateChanged = { [weak self] state, detail in
      Task { @MainActor in
        self?.handleRuntimeConnectionStateChange(state, detail: detail)
      }
    }
  }

  func selectLocalExecutionSafetyMode(_ mode: String) {
    selectedLocalExecutionSafetyMode = LocalExecutionSafetyModePresenter.validMode(mode)
  }

  var threads: [ThreadSummary] {
    get {
      timelineState.threads
    }
    set {
      updateTimelineState { state in
        state.threads = newValue
      }
    }
  }

  var selectedThreadID: ThreadSummary.ID? {
    get {
      timelineState.selectedThreadID
    }
    set {
      updateTimelineState { state in
        state.selectedThreadID = newValue
      }
    }
  }

  var timeline: [TimelineEntry] {
    get {
      timelineState.timeline
    }
    set {
      updateTimelineState { state in
        state.timeline = newValue
      }
    }
  }

  var selectedEntryID: TimelineEntry.ID? {
    get {
      timelineState.selectedEntryID
    }
    set {
      updateTimelineState { state in
        state.selectedEntryID = newValue
      }
    }
  }

  var activeTurnID: String? {
    get {
      timelineState.activeTurnID
    }
    set {
      updateTimelineState { state in
        state.activeTurnID = newValue
      }
    }
  }

  var workspaceSearchQuery: String {
    get {
      workspaceSearchState.query
    }
    set {
      updateWorkspaceSearchState { state in
        state.query = newValue
      }
    }
  }

  var workspaceSearchResults: [WorkspaceSearchMatchSummary] {
    workspaceSearchState.results
  }

  var workspaceSearchStatus: String {
    workspaceSearchState.status
  }

  var isWorkspaceSearching: Bool {
    workspaceSearchState.isSearching
  }

  var runtimeState: RuntimeBridge.ConnectionState {
    get {
      runtimeConnectionState.state
    }
    set {
      updateRuntimeConnectionState { state in
        state.state = newValue
      }
    }
  }

  var runtimeDetail: String {
    get {
      runtimeConnectionState.detail
    }
    set {
      updateRuntimeConnectionState { state in
        state.detail = newValue
      }
    }
  }

  var modelHealth: ModelHealthSummary? {
    get {
      localModelReadinessState.modelHealth
    }
    set {
      updateLocalModelReadinessState { state in
        state.modelHealth = newValue
      }
    }
  }

  var runtimeReadiness: RuntimeReadinessSummary? {
    get {
      localModelReadinessState.runtimeReadiness
    }
    set {
      updateLocalModelReadinessState { state in
        state.runtimeReadiness = newValue
      }
    }
  }

  var localModels: [LocalModelSummary] {
    get {
      localModelReadinessState.models
    }
    set {
      updateLocalModelReadinessState { state in
        state.models = newValue
      }
    }
  }

  var selectedSetupModelID: String {
    get {
      localModelReadinessState.selectedSetupModelID
    }
    set {
      updateLocalModelReadinessState { state in
        state.selectedSetupModelID = newValue
      }
      AppPreferences.storeSelectedSetupModelID(newValue)
    }
  }

  var plugins: [PluginSummary] {
    pluginState.plugins
  }

  var pluginCapabilityRegistrySummary: PluginCapabilityRegistrySummary? {
    pluginState.registrySummary
  }

  var pluginCapabilities: [PluginCapabilitySummary] {
    pluginState.capabilities
  }

  var pluginConnectors: [PluginConnectorSummary] {
    pluginState.connectors
  }

  var pluginCommands: [PluginCommandSummary] {
    pluginState.commands
  }

  var pluginHooks: [PluginHookSummary] {
    pluginState.hooks
  }

  var pluginSkills: [PluginSkillSummary] {
    pluginState.skills
  }

  var pluginDashboardSnapshot: PluginDashboardSnapshot {
    pluginState.dashboardSnapshot
  }

  func pluginSummary(pluginID: String) -> PluginSummary? {
    pluginState.plugin(id: pluginID)
  }

  var memoryStatus: MemoryStatusSummary? {
    memoryState.status
  }

  var memoryNotes: [MemoryNoteSummary] {
    memoryState.notes
  }

  var memoryNoteTitle: String {
    get {
      memoryState.noteTitle
    }
    set {
      updateMemoryState { state in
        state.noteTitle = newValue
      }
    }
  }

  var memoryNoteBody: String {
    get {
      memoryState.noteBody
    }
    set {
      updateMemoryState { state in
        state.noteBody = newValue
      }
    }
  }

  var isSavingMemoryNote: Bool {
    memoryState.isSavingNote
  }

  func updateWorkspaceSearchState(
    _ update: (inout WorkspaceSearchRuntimeState) -> Void
  ) {
    var nextState = workspaceSearchState
    update(&nextState)
    workspaceSearchState = nextState
  }

  func updateTimelineState(_ update: (inout TimelineRuntimeState) -> Void) {
    var nextState = timelineState
    update(&nextState)
    timelineState = nextState
  }

  func updateRuntimeConnectionState(
    _ update: (inout RuntimeConnectionStateStore) -> Void
  ) {
    var nextState = runtimeConnectionState
    update(&nextState)
    runtimeConnectionState = nextState
  }

  func updateLocalModelReadinessState(
    _ update: (inout LocalModelReadinessState) -> Void
  ) {
    var nextState = localModelReadinessState
    update(&nextState)
    localModelReadinessState = nextState
  }

  func updateMemoryState(_ update: (inout MemoryRuntimeState) -> Void) {
    var nextState = memoryState
    update(&nextState)
    memoryState = nextState
  }

  func updatePluginState(_ update: (inout PluginRuntimeState) -> Void) {
    var nextState = pluginState
    update(&nextState)
    pluginState = nextState
  }

}
