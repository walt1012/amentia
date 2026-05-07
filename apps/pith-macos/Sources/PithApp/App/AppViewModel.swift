import Combine
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var timelineState: TimelineRuntimeState
  @Published var runtimeConnectionState: RuntimeConnectionStateStore
  @Published var draftMessage: String
  @Published var workspace: WorkspaceSummary?
  @Published var workspaceSearchState: WorkspaceSearchRuntimeState
  @Published private var localModelReadinessState: LocalModelReadinessState
  @Published var modelDownloadState: LocalModelDownloadRuntimeState
  @Published private var memoryState: MemoryRuntimeState
  @Published private var pluginState: PluginRuntimeState

  let runtimeBridge: RuntimeBridge
  let pendingTurnRequest = PendingTurnRequestState()
  let workspaceSearchSession = WorkspaceSearchSession()
  let modelDownloadCoordinator: LocalModelDownloadCoordinator
  let localModelDownloadRequestPlanCache = LocalModelDownloadRequestPlanCache()

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let launchState = AppLaunchState.make(runtimeBridge: runtimeBridge)

    self.runtimeBridge = runtimeBridge
    self.timelineState = TimelineRuntimeState(welcomeState: launchState.welcomeState)
    self.runtimeConnectionState = RuntimeConnectionStateStore(
      state: runtimeBridge.connectionState,
      detail: launchState.runtimeDetail
    )
    self.draftMessage = ""
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

  func openWorkspace() {
    guard canOpenWorkspace() else {
      return
    }

    guard let url = AppFilePicker.chooseWorkspace() else {
      return
    }

    Task {
      do {
        let bootstrap = try await WorkspaceOpenBootstrapLoader.load(
          runtimeBridge: runtimeBridge,
          path: url.path
        )
        try await applyWorkspaceOpenBootstrap(bootstrap)
        announceFirstRequestReadyIfNeeded()
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.workspaceOpenFailed(error: error)
        )
      }
    }
  }

  private func applyWorkspaceOpenBootstrap(_ bootstrap: WorkspaceOpenBootstrap) async throws {
    workspace = WorkspaceSummary(
      rootPath: bootstrap.workspace.rootPath,
      displayName: bootstrap.workspace.displayName
    )
    resetWorkspaceSearch()
    AppPreferences.storeLastWorkspacePath(bootstrap.workspace.rootPath)
    applyMemoryStateRefresh(bootstrap.memoryRefresh, clearsMissing: false)
    try await refreshWorkspaceThreadSelection(
      from: bootstrap.threadList,
      createIfEmpty: isLocalModelReady()
    )
    await refreshRuntimeReadiness()
    appendWorkspaceOpenedEvent(bootstrap.workspace)
  }

  private func appendWorkspaceOpenedEvent(_ workspace: RuntimeBridge.RuntimeWorkspace) {
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.workspaceOpened(workspace)
    )
  }

  func createThread() {
    guard canCreateThread() else {
      return
    }

    Task {
      do {
        let thread = try await runtimeBridge.startThread(title: "Thread \(threads.count + 1)")
        await applyCreatedThread(thread)
        announceFirstRequestReadyIfNeeded()
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.threadCreationFailed(error: error)
        )
      }
    }
  }

  private func applyCreatedThread(_ thread: ThreadSummary) async {
    updateTimelineState { state in
      state.applyCreatedThread(thread)
    }
    await loadThreadHistory(threadID: thread.id)
    appendEntry(
      to: thread.id,
      TimelineEventPresenter.threadCreated(thread)
    )
  }

  func sendDraftMessage() {
    guard let draftTurn = SessionActionPlanner.preparedDraftTurn(
      snapshot: sessionActionSnapshot(),
      selectedThreadID: selectedThreadID,
      draftMessage: draftMessage
    ) else {
      return
    }

    let threadID = draftTurn.threadID
    let message = draftTurn.message
    let requestID = beginPendingLocalTurn(threadID: threadID)

    let task = Task {
      defer {
        pendingTurnRequest.clear(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.startTurn(threadID: threadID, message: message)
        await applyRuntimeTurnResult(result)
      } catch {
        if Task.isCancelled {
          applyPendingTurnCancellation(threadID: threadID)
          return
        }
        applyPendingTurnFailure(threadID: threadID, message: message, error: error)
      }
    }
    pendingTurnRequest.bind(task: task, requestID: requestID)
  }

  private func beginPendingLocalTurn(threadID: String) -> UUID {
    draftMessage = ""
    runtimeDetail = TimelineEventPresenter.generatingLocalResponseDetail
    return pendingTurnRequest.begin(threadID: threadID)
  }

  private func applyPendingTurnCancellation(threadID: String) {
    runtimeDetail = TimelineEventPresenter.pendingTurnCancelledDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancelledResponsePreview
    )
    appendEntry(
      to: threadID,
      TimelineEventPresenter.pendingTurnCancelled()
    )
  }

  private func applyPendingTurnFailure(
    threadID: String,
    message: String,
    error: Error
  ) {
    if draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      draftMessage = message
    }
    runtimeDetail = error.localizedDescription
    appendEntry(
      to: threadID,
      TimelineEventPresenter.turnFailed(error: error)
    )
  }

  func respondToApproval(approvalID: String, decision: String) {
    guard canRespondToApproval(approvalID: approvalID),
          decision == "approved" || decision == "denied"
    else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.respondToApproval(
          approvalID: approvalID,
          decision: decision
        )
        await applyApprovalResponse(result)
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.approvalResponseFailed(error: error)
        )
      }
    }
  }

  func runPluginCommand(commandID: String) {
    let snapshot = pluginActionSnapshot()
    if PluginActionPlanner.commandNeedsExecutionContract(commandID: commandID, snapshot: snapshot) {
      runtimeDetail = TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail
      return
    }

    guard PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot),
          let threadID = selectedThreadID
    else {
      return
    }

    runtimeDetail = TimelineEventPresenter.runningPluginCommandDetail
    let requestID = pendingTurnRequest.begin(threadID: threadID)

    let task = Task {
      defer {
        pendingTurnRequest.clear(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.runPluginCommand(threadID: threadID, commandID: commandID)
        await applyRuntimeTurnResult(result, refreshMemory: true)
      } catch {
        if Task.isCancelled {
          runtimeDetail = TimelineEventPresenter.pendingPluginCommandCancelledDetail
          refreshThreadPreview(
            threadID: threadID,
            preview: TimelineEventPresenter.cancelledPluginCommandPreview
          )
          appendEntry(
            to: threadID,
            TimelineEventPresenter.pluginCommandCancelled()
          )
          return
        }
        runtimeDetail = error.localizedDescription
        appendEntry(
          to: threadID,
          TimelineEventPresenter.pluginCommandFailed(error: error)
        )
      }
    }
    pendingTurnRequest.bind(task: task, requestID: requestID)
  }

  func canRunPluginCommand(commandID: String) -> Bool {
    PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: pluginActionSnapshot())
  }

  func selectThread(id: String?) {
    updateTimelineState { state in
      state.selectedThreadID = id
      state.syncVisibleTimeline()
    }

    guard runtimeState == .ready,
          let threadID = id,
          !threadID.hasPrefix("local-")
    else {
      return
    }

    Task {
      await loadThreadHistory(threadID: threadID)
      announceFirstRequestReadyIfNeeded()
    }
  }

  func cancelActiveTurn() {
    guard canCancelActiveTurn() else {
      return
    }

    if cancelPendingTurnRequest() {
      return
    }

    guard let activeTurnID,
          let activeTurnThreadID = timelineState.activeTurnThreadID
    else {
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.cancelTurn(turnID: activeTurnID)
        await applyTurnCancellation(result, previewThreadID: activeTurnThreadID)
      } catch {
        appendEntry(
          to: activeTurnThreadID,
          TimelineEventPresenter.turnCancelFailed(error: error)
        )
      }
    }
  }

  func isLocalModelReady() -> Bool {
    guard runtimeState == .ready,
          let modelHealth,
          modelHealth.status == "ready",
          hasActiveCatalogModel()
    else {
      return false
    }

    return (modelHealth.metrics["readiness"] ?? "unknown") == "configured"
  }

  func composerPlaceholder() -> String {
    ComposerStatusPresenter.placeholder(composerStatusSnapshot())
  }

  func composerStatusSummary() -> String {
    ComposerStatusPresenter.statusSummary(composerStatusSnapshot())
  }

  func showsComposerActivity() -> Bool {
    runtimeState == .launching || hasActiveOrPendingTurn()
  }

  func isTurnStreaming() -> Bool {
    hasActiveOrPendingTurn()
  }

  func hasActiveOrPendingTurn() -> Bool {
    activeTurnID != nil || pendingTurnRequest.isPending
  }

  private func appendItemsToTimeline(
    threadID: String,
    items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) {
    let newEntries = TimelineEntryFactory.transientEntries(from: items)

    for entry in newEntries.reversed() {
      appendEntry(to: threadID, entry)
    }
  }

  private func updatePendingApprovals(
    threadID: String,
    approvals: [RuntimeBridge.RuntimeApproval]
  ) {
    updateTimelineState { state in
      state.updatePendingApprovals(threadID: threadID, approvals: approvals)
    }
  }

  private func applyRuntimeTurnResult(
    _ result: RuntimeBridge.RuntimeTurnResult,
    refreshMemory: Bool = false
  ) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
    updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
    refreshThreadPreview(
      threadID: result.threadID,
      preview: TimelineEventPresenter.turnPreview(
        turnID: result.turnID,
        activeTurnID: result.activeTurnID
      )
    )

    if refreshMemory {
      await refreshMemoryState()
    }
  }

  private func applyApprovalResponse(_ result: RuntimeBridge.RuntimeApprovalResponse) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
    await refreshMemoryState()
    await loadThreadHistory(threadID: result.threadID)
  }

  private func applyTurnCancellation(
    _ result: RuntimeBridge.RuntimeTurnCancellation,
    previewThreadID: String
  ) async {
    appendItemsToTimeline(threadID: result.threadID, items: result.items)
    updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
    refreshThreadPreview(
      threadID: previewThreadID,
      preview: TimelineEventPresenter.cancelledResponsePreview
    )
    await loadThreadHistory(threadID: result.threadID)
  }

  private func refreshThreadPreview(threadID: String, preview: String) {
    updateTimelineState { state in
      state.refreshThreadPreview(threadID: threadID, preview: preview)
    }
  }

  func refreshWorkspaceThreadSelection(
    from runtimeThreads: [RuntimeBridge.RuntimeThreadSummary],
    createIfEmpty: Bool
  ) async throws {
    guard let workspace else {
      resetToWelcomeThread()
      return
    }

    let workspaceThreads = try await WorkspaceThreadSelectionLoader.load(
      workspace: workspace,
      runtimeThreads: runtimeThreads,
      createIfEmpty: createIfEmpty,
      startThread: { [runtimeBridge] title in
        try await runtimeBridge.startThread(title: title)
      }
    )

    if workspaceThreads.isEmpty {
      resetToWelcomeThread()
      return
    }

    applyWorkspaceThreadSelection(workspaceThreads)
    if let selectedThreadID {
      await loadThreadHistory(threadID: selectedThreadID)
      announceFirstRequestReadyIfNeeded()
    }
  }

  private func applyWorkspaceThreadSelection(_ workspaceThreads: [ThreadSummary]) {
    updateTimelineState { state in
      state.applyWorkspaceThreads(workspaceThreads)
    }
  }

  func resetToWelcomeThread() {
    updateTimelineState { state in
      state.resetToWelcomeState(TimelineSessionState.welcomeState())
    }
  }

  func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
    updateTimelineState { state in
      state.appendEntry(to: threadID, entry)
    }
  }

  private func applyThreadEntries(threadID: String, entries: [TimelineEntry]) {
    updateTimelineState { state in
      state.applyThreadEntries(threadID: threadID, entries: entries)
    }
  }

  private func threadTitle(for threadID: String) -> String {
    timelineState.threadTitle(for: threadID)
  }

  private func loadThreadHistory(threadID: String) async {
    do {
      let result = try await runtimeBridge.readThread(threadID: threadID)
      let entries = TimelineEntryFactory.runtimeEntries(
        from: result.items,
        existingEntries: timelineState.threadTimelines[threadID],
        fallbackTitle: threadTitle(for: threadID)
      )
      applyThreadEntries(threadID: threadID, entries: entries)
      updatePendingApprovals(threadID: threadID, approvals: result.pendingApprovals)
      updateActiveTurn(threadID: threadID, activeTurnID: result.activeTurnID)
      refreshThreadPreview(threadID: threadID, preview: result.status)
    } catch {
      appendEntry(
        to: threadID,
        TimelineEventPresenter.threadLoadFailed(error: error)
      )
    }
  }

  func selectedEntry() -> TimelineEntry? {
    timelineState.selectedEntry()
  }

  private func updateActiveTurn(threadID: String, activeTurnID: String?) {
    updateTimelineState { state in
      state.updateActiveTurn(threadID: threadID, activeTurnID: activeTurnID)
    }
  }

  private func cancelPendingTurnRequest() -> Bool {
    guard let threadID = pendingTurnRequest.cancel() else {
      return false
    }

    runtimeDetail = TimelineEventPresenter.cancellingTurnDetail
    refreshThreadPreview(
      threadID: threadID,
      preview: TimelineEventPresenter.cancellingResponsePreview
    )
    return true
  }

  func refreshLocalModelCatalog() {
    let refreshPlan = LocalModelCatalogRefreshPlanner.plan(
      LocalModelCatalogRefreshSnapshot(
        storageRootPath: runtimeBridge.localModelStorageRootPath(),
        configuredActiveModelPath: runtimeBridge.activeLocalModelPath(),
        runtimeModelPath: modelHealth?.modelPath,
        selectedSetupModelID: selectedSetupModelID
      )
    )
    if refreshPlan.shouldClearConfiguredActiveModel {
      runtimeBridge.clearActiveLocalModel()
    }
    updateLocalModelReadinessState { state in
      state.applyCatalogRefresh(refreshPlan)
    }
    AppPreferences.storeSelectedSetupModelID(refreshPlan.selectedSetupModelID)
  }

  func pluginActionSnapshot() -> PluginActionSnapshot {
    PluginActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      selectedThreadID: selectedThreadID,
      hasActiveOrPendingTurn: hasActiveOrPendingTurn(),
      plugins: plugins,
      commands: pluginCommands
    )
  }

  private func composerStatusSnapshot() -> ComposerStatusSnapshot {
    let modelGuidance = localModelSetupGuidance()
    return ComposerStatusSnapshot(
      runtimeState: runtimeState,
      modelSetupTitle: modelGuidance.title,
      modelSetupSummary: modelGuidance.summary,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty
    )
  }

  func localModelStatusSnapshot() -> LocalModelStatusSnapshot {
    return LocalModelStatusSnapshot(
      runtimeState: runtimeState,
      modelHealth: modelHealth,
      modelDownloadID: modelDownloadState.activeModelID,
      pausedModelDownloadID: modelDownloadState.pausedModelID,
      modelDownloadProgress: modelDownloadState.progress,
      selectedSetupModelID: selectedSetupModelID,
      selectedSetupModel: selectedSetupModel(),
      hasActiveCatalogModel: hasActiveCatalogModel()
    )
  }

  func selectedSetupModel() -> LocalModelSummary? {
    localModel(for: selectedSetupModelID)
      ?? localModel(for: LocalModelCatalog.defaultFirstUseModelID)
      ?? localModels.first
  }

  func localModel(for modelID: String?) -> LocalModelSummary? {
    guard let modelID else {
      return nil
    }

    return localModels.first(where: { $0.id == modelID })
  }

  private func activeLocalModel() -> LocalModelSummary? {
    localModels.first(where: { $0.active })
  }

  private func hasActiveCatalogModel() -> Bool {
    localModels.contains(where: { $0.active })
  }

  func localModelSetupGuidance() -> LocalModelSetupGuidance {
    LocalModelOperationPresenter.setupGuidance(localModelOperationSnapshot())
  }

  func localModelOperationSnapshot() -> LocalModelOperationSnapshot {
    let downloadedModels = localModels.filter { $0.downloaded }
    let downloadedLocalSize = downloadedModels
      .compactMap { $0.localSizeBytes }
      .reduce(Int64(0), +)

    return LocalModelOperationSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      downloadingModel: localModel(for: modelDownloadState.activeModelID),
      pausedModel: localModel(for: modelDownloadState.pausedModelID),
      selectedSetupModel: selectedSetupModel(),
      selectedDownloadBlockedDetail: selectedSetupModelDownloadBlockedDetail(),
      downloadedModelCount: downloadedModels.count,
      totalModelCount: localModels.count,
      activeModelDisplayName: activeLocalModel()?.displayName,
      downloadedLocalSizeBytes: downloadedLocalSize
    )
  }

  func localModelActionSnapshot() -> LocalModelActionSnapshot {
    let canDownloadPausedModel = modelDownloadState.pausedModelID
      .map { canDownloadRecommendedModel(modelID: $0) }
      ?? false

    return LocalModelActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasModelDownload: modelDownloadState.hasActiveDownload,
      pausedModelDownloadID: modelDownloadState.pausedModelID,
      selectedDownloadBlockedDetail: selectedSetupModelDownloadBlockedDetail(),
      canPauseDownload: canPauseModelDownload(),
      canDownloadPausedModel: canDownloadPausedModel,
      canDownloadSelectedModel: canDownloadLocalModel(),
      canBootstrapModelPackMetadata: canBootstrapModelPackMetadata(),
      canCancelDownload: canCancelModelDownload(),
      defaultDownloadTitle: defaultModelDownloadButtonTitle()
    )
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

  func refreshMemoryState() async {
    let memoryRefresh = await MemoryStateLoader.refresh(using: runtimeBridge)
    applyMemoryStateRefresh(memoryRefresh, clearsMissing: false)
  }

  func applyMemoryStateRefresh(
    _ memoryRefresh: MemoryStateRefresh,
    clearsMissing: Bool
  ) {
    updateMemoryState { state in
      state.apply(memoryRefresh, clearsMissing: clearsMissing)
    }
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

  private func applyRuntimeThreadUpdate(_ state: RuntimeBridge.RuntimeThreadState) {
    let entries = TimelineEntryFactory.runtimeEntries(
      from: state.items,
      existingEntries: timelineState.threadTimelines[state.id],
      fallbackTitle: threadTitle(for: state.id)
    )

    applyThreadEntries(threadID: state.id, entries: entries)
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)
  }

}
