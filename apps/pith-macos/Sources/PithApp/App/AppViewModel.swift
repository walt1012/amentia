import Combine
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published var threads: [ThreadSummary]
  @Published var selectedThreadID: ThreadSummary.ID?
  @Published var timeline: [TimelineEntry]
  @Published var selectedEntryID: TimelineEntry.ID?
  @Published var activeTurnID: String?
  @Published var runtimeState: RuntimeBridge.ConnectionState
  @Published var runtimeDetail: String
  @Published var draftMessage: String
  @Published var workspace: WorkspaceSummary?
  @Published var workspaceSearchQuery: String
  @Published var workspaceSearchResults: [WorkspaceSearchMatchSummary]
  @Published var workspaceSearchStatus: String
  @Published var isWorkspaceSearching: Bool
  @Published var modelHealth: ModelHealthSummary?
  @Published var runtimeReadiness: RuntimeReadinessSummary?
  @Published var localModels: [LocalModelSummary]
  @Published var selectedSetupModelID: String {
    didSet {
      AppPreferences.storeSelectedSetupModelID(selectedSetupModelID)
    }
  }
  @Published private var modelDownloadState: LocalModelDownloadRuntimeState
  @Published var memoryStatus: MemoryStatusSummary?
  @Published var memoryNotes: [MemoryNoteSummary]
  @Published var memoryNoteTitle: String
  @Published var memoryNoteBody: String
  @Published var plugins: [PluginSummary]
  @Published var pluginCapabilityRegistrySummary: PluginCapabilityRegistrySummary?
  @Published var pluginCapabilities: [PluginCapabilitySummary]
  @Published var pluginConnectors: [PluginConnectorSummary]
  @Published var pluginCommands: [PluginCommandSummary]
  @Published var pluginHooks: [PluginHookSummary]

  private let runtimeBridge: RuntimeBridge
  private var threadTimelines: [String: [TimelineEntry]]
  private var threadPendingApprovalIDs: [String: Set<String>]
  private var activeTurnThreadID: String?
  private let pendingTurnRequest = PendingTurnRequestState()
  private var lastRuntimeFailureDetail: String?
  private let workspaceSearchSession = WorkspaceSearchSession()
  private let modelDownloadCoordinator: LocalModelDownloadCoordinator
  private let localModelDownloadRequestPlanCache = LocalModelDownloadRequestPlanCache()
  private var announcedSetupCompleteThreadIDs: Set<String>

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let launchState = AppLaunchState.make(runtimeBridge: runtimeBridge)
    let initialTimeline = launchState.welcomeState.timeline
    let initialThreads = [launchState.welcomeState.thread]

    self.runtimeBridge = runtimeBridge
    self.runtimeState = runtimeBridge.connectionState
    self.runtimeDetail = launchState.runtimeDetail
    self.draftMessage = ""
    self.workspace = nil
    self.workspaceSearchQuery = ""
    self.workspaceSearchResults = []
    self.workspaceSearchStatus = "Search the open workspace by text."
    self.isWorkspaceSearching = false
    self.modelHealth = nil
    self.runtimeReadiness = nil
    self.localModels = launchState.localModels
    self.selectedSetupModelID = launchState.selectedSetupModelID
    self.modelDownloadState = LocalModelDownloadRuntimeState(
      activeModelID: nil,
      pausedModelID: launchState.pausedDownload?.modelID,
      progress: launchState.modelDownloadProgress
    )
    self.memoryStatus = nil
    self.memoryNotes = []
    self.memoryNoteTitle = ""
    self.memoryNoteBody = ""
    self.plugins = []
    self.pluginCapabilityRegistrySummary = nil
    self.pluginCapabilities = []
    self.pluginConnectors = []
    self.pluginCommands = []
    self.pluginHooks = []
    self.threads = initialThreads
    self.timeline = initialTimeline
    self.selectedEntryID = initialTimeline.first?.id
    self.activeTurnID = nil
    self.threadTimelines = [launchState.welcomeState.thread.id: initialTimeline]
    self.threadPendingApprovalIDs = [:]
    self.lastRuntimeFailureDetail = nil
    self.modelDownloadCoordinator = LocalModelDownloadCoordinator(
      resumeData: launchState.pausedDownload?.resumeData
    )
    self.announcedSetupCompleteThreadIDs = Set<String>()
    self.selectedThreadID = initialThreads.first?.id
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

  func launchRuntime(launchDetail: String = "Launching local runtime") {
    guard runtimeState != .launching else {
      return
    }

    if runtimeState == .ready {
      runtimeBridge.stopRuntime(detail: "Relaunching local runtime...")
    }

    runtimeState = .launching
    runtimeDetail = launchDetail
    lastRuntimeFailureDetail = nil

    Task {
      do {
        let bootstrap = try await RuntimeLaunchBootstrapLoader.load(
          runtimeBridge: runtimeBridge,
          launchDetail: launchDetail,
          lastWorkspacePath: AppPreferences.storedLastWorkspacePath(),
          isRestorablePath: isRestorableWorkspacePath,
          clearStoredWorkspace: AppPreferences.clearLastWorkspacePath
        )
        try await applyRuntimeLaunchBootstrap(bootstrap)
        announceFirstRequestReadyIfNeeded()
      } catch {
        applyRuntimeLaunchFailure(error)
      }
    }
  }

  private func applyRuntimeLaunchBootstrap(_ bootstrap: RuntimeLaunchBootstrap) async throws {
    let currentWorkspace = bootstrap.workspaceRestore.workspace

    runtimeState = .ready
    await refreshModelHealthState(
      serverLabel: "\(bootstrap.session.serverName) \(bootstrap.session.serverVersion)"
    )
    applyMemoryStateRefresh(bootstrap.memoryRefresh, clearsMissing: true)
    await refreshPluginState()

    if let currentWorkspace {
      workspace = WorkspaceSummary(
        rootPath: currentWorkspace.rootPath,
        displayName: currentWorkspace.displayName
      )
      resetWorkspaceSearch()
      AppPreferences.storeLastWorkspacePath(currentWorkspace.rootPath)
    }

    if workspace != nil {
      try await refreshWorkspaceThreadSelection(
        from: bootstrap.threadList,
        createIfEmpty: isLocalModelReady()
      )
    } else {
      resetToWelcomeThread()
    }
    await refreshRuntimeReadiness()
    appendRuntimeLaunchAnnotations(bootstrap, currentWorkspace: currentWorkspace)
  }

  private func appendRuntimeLaunchAnnotations(
    _ bootstrap: RuntimeLaunchBootstrap,
    currentWorkspace: RuntimeBridge.RuntimeWorkspace?
  ) {
    let restoredWorkspaceSummary = bootstrap.workspaceRestore.restoredWorkspace
      ? currentWorkspace.map {
        WorkspaceSummary(rootPath: $0.rootPath, displayName: $0.displayName)
      }
      : nil

    RuntimeLaunchAnnotationFactory.entries(
      RuntimeLaunchAnnotationSnapshot(
        serverName: bootstrap.session.serverName,
        serverVersion: bootstrap.session.serverVersion,
        shouldAnnotateSetupLaunch: shouldAnnotateLaunchWithSetupEvents(),
        restoredWorkspace: restoredWorkspaceSummary,
        skippedWorkspaceRestorePath: bootstrap.workspaceRestore.skippedWorkspaceRestorePath,
        workspaceRestoreErrorDetail: bootstrap.workspaceRestore.restoreErrorDetail,
        modelHealth: modelHealth,
        isLocalModelReady: isLocalModelReady(),
        localModelRequiredSummary: localModelRequiredTimelineSummary()
      )
    ).forEach { entry in
      appendEntry(to: selectedThreadID, entry)
    }
  }

  func runtimeLaunchButtonTitle() -> String {
    SessionActionPlanner.runtimeLaunchButtonTitle(sessionActionSnapshot())
  }

  func shouldShowRuntimeToolbarAction() -> Bool {
    SessionActionPlanner.shouldShowRuntimeToolbarAction(sessionActionSnapshot())
  }

  func runtimeStatusSummary() -> String {
    RuntimeHeaderPresenter.statusSummary(runtimeHeaderSnapshot())
  }

  func runtimeStatusTone() -> StatusTone {
    RuntimeHeaderPresenter.statusTone(runtimeHeaderSnapshot())
  }

  func showsRuntimeActivity() -> Bool {
    RuntimeHeaderPresenter.showsActivity(runtimeHeaderSnapshot())
  }

  func shouldShowRuntimeHeaderDetail() -> Bool {
    RuntimeHeaderPresenter.shouldShowDetail(runtimeHeaderSnapshot())
  }

  func runtimeReadinessSteps() -> [ReadinessStepSummary] {
    RuntimeReadinessPresenter.steps(runtimeReadinessSnapshot())
  }

  func readinessStepActionTitle(_ step: ReadinessStepSummary) -> String? {
    let snapshot = runtimeReadinessActionSnapshot()
    return RuntimeReadinessActionPlanner.title(
      for: RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot),
      snapshot: snapshot
    )
  }

  func canRunReadinessStepAction(_ step: ReadinessStepSummary) -> Bool {
    let snapshot = runtimeReadinessActionSnapshot()
    return RuntimeReadinessActionPlanner.canRun(
      RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot),
      snapshot: snapshot
    )
  }

  func runReadinessStepAction(_ step: ReadinessStepSummary) {
    let snapshot = runtimeReadinessActionSnapshot()
    guard let action = RuntimeReadinessActionPlanner.action(for: step, snapshot: snapshot),
          RuntimeReadinessActionPlanner.canRun(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .launchRuntime:
      launchRuntime()
    case .setupModel:
      runModelSetupCalloutAction()
    case .openWorkspace:
      openWorkspace()
    case .createThread:
      createThread()
    case .useFirstRequestPrompt:
      useFirstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
    case .sendFirstRequest:
      sendDraftMessage()
    }
  }

  func setupProgressSummary() -> String {
    SetupProgressPresenter.summary(setupProgressSnapshot())
  }

  func setupProgressDetail() -> String {
    SetupProgressPresenter.detail(setupProgressSnapshot())
  }

  func setupProgressValue() -> Double {
    SetupProgressPresenter.value(setupProgressSnapshot())
  }

  func setupProgressTone() -> StatusTone {
    SetupProgressPresenter.tone(setupProgressSnapshot())
  }

  func shouldShowSetupProgress() -> Bool {
    let snapshot = setupProgressSnapshot()
    return snapshot.readyStepCount < snapshot.stepCount
      || snapshot.runtimeState == .launching
      || modelDownloadState.hasAnyDownloadState
  }

  func shouldShowReadinessSteps() -> Bool {
    shouldShowSetupProgress()
  }

  func inspectorSessionTitle() -> String {
    InspectorSessionPresenter.title(inspectorSessionSnapshot())
  }

  func inspectorSessionDetail() -> String {
    InspectorSessionPresenter.detail(inspectorSessionSnapshot())
  }

  func inspectorSessionMetaSummary() -> String {
    InspectorSessionPresenter.metaSummary(inspectorSessionSnapshot())
  }

  func shouldShowSetupCallout() -> Bool {
    runtimeState == .ready
      && (!isLocalModelReady() || workspace == nil || !hasRuntimeThreadSelection())
  }

  func setupCalloutTitle() -> String {
    SetupCalloutPresenter.title(setupCalloutSnapshot())
  }

  func setupCalloutSummary() -> String {
    SetupCalloutPresenter.summary(setupCalloutSnapshot())
  }

  func setupCalloutDetail() -> String {
    SetupCalloutPresenter.detail(setupCalloutSnapshot())
  }

  func setupCalloutTone() -> StatusTone {
    SetupCalloutPresenter.tone(setupCalloutSnapshot())
  }

  func setupCalloutActionTitle() -> String? {
    SetupCalloutPresenter.primaryActionTitle(setupCalloutSnapshot())
  }

  func canRunSetupCalloutAction() -> Bool {
    let snapshot = setupCalloutActionSnapshot()
    return SetupCalloutActionPlanner.canRun(
      SetupCalloutActionPlanner.primaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runSetupCalloutAction() {
    let snapshot = setupCalloutActionSnapshot()
    guard let action = SetupCalloutActionPlanner.primaryAction(snapshot),
          SetupCalloutActionPlanner.canRun(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .setupModel:
      runModelSetupCalloutAction()
    case .openWorkspace:
      openWorkspace()
    case .createThread:
      createThread()
    }
  }

  func setupCalloutSecondaryActionTitle() -> String? {
    SetupCalloutPresenter.secondaryActionTitle(setupCalloutSnapshot())
  }

  func canRunSetupCalloutSecondaryAction() -> Bool {
    let snapshot = setupCalloutActionSnapshot()
    return SetupCalloutActionPlanner.canRun(
      SetupCalloutActionPlanner.secondaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runSetupCalloutSecondaryAction() {
    let snapshot = setupCalloutActionSnapshot()
    guard let action = SetupCalloutActionPlanner.secondaryAction(snapshot),
          SetupCalloutActionPlanner.canRun(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .setupModelSecondary:
      runModelSetupCalloutSecondaryAction()
    }
  }

  func shouldShowFirstRequestCallout() -> Bool {
    canUseComposer()
      && trimmedDraftMessage.isEmpty
      && selectedThreadIsWaitingForFirstMessage()
  }

  func firstRequestCalloutTitle() -> String {
    "First Local Request"
  }

  func firstRequestCalloutSummary() -> String {
    FirstRequestPromptPresenter.calloutSummary()
  }

  func firstRequestCalloutDetail() -> String {
    FirstRequestPromptPresenter.calloutDetail(workspaceDisplayName: workspace?.displayName)
  }

  func firstRequestCalloutActionTitle() -> String? {
    FirstRequestPromptPresenter.primaryActionTitle(
      for: firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
    )
  }

  func canRunFirstRequestCalloutAction() -> Bool {
    firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID) != nil
  }

  func runFirstRequestCalloutAction() {
    guard canRunFirstRequestCalloutAction() else {
      return
    }

    useFirstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
  }

  func firstRequestCalloutSecondaryActionTitle() -> String? {
    FirstRequestPromptPresenter.secondaryActionTitle(
      for: firstRequestSuggestion(id: FirstRequestPromptPresenter.reviewChangesID)
    )
  }

  func canRunFirstRequestCalloutSecondaryAction() -> Bool {
    firstRequestSuggestion(id: FirstRequestPromptPresenter.reviewChangesID) != nil
  }

  func runFirstRequestCalloutSecondaryAction() {
    guard canRunFirstRequestCalloutSecondaryAction() else {
      return
    }

    useFirstRequestSuggestion(id: FirstRequestPromptPresenter.reviewChangesID)
  }

  func shouldShowSetupModelChoice() -> Bool {
    runtimeState == .ready
      && !isLocalModelReady()
      && !modelDownloadState.hasAnyDownloadState
      && !localModels.isEmpty
  }

  func canChangeSetupModelChoice() -> Bool {
    shouldShowSetupModelChoice()
  }

  func setupModelChoiceDetail() -> String {
    LocalModelOperationPresenter.setupModelChoiceDetail(
      localModelOperationSnapshot(),
      defaultModelID: LocalModelCatalog.defaultFirstUseModelID
    )
  }

  func setupDefaultModelID() -> String {
    LocalModelCatalog.defaultFirstUseModelID
  }

  func modelSetupCalloutTitle() -> String {
    localModelSetupGuidance().title
  }

  func modelSetupCalloutSummary() -> String {
    localModelSetupGuidance().summary
  }

  func modelSetupCalloutDetail() -> String {
    if shouldShowModelDownloadProgress() {
      return modelDownloadProgressSummary()
    }
    if let blockedDetail = selectedSetupModelDownloadBlockedDetail() {
      return blockedDetail
    }

    return localModelSetupGuidance().detail
  }

  func modelSetupCalloutTone() -> StatusTone {
    localModelSetupGuidance().tone
  }

  func modelSetupCalloutActionTitle() -> String? {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.primaryTitle(
      for: LocalModelActionPlanner.setupPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func canRunModelSetupCalloutAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.setupPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runModelSetupCalloutAction() {
    let snapshot = localModelActionSnapshot()
    let action = LocalModelActionPlanner.setupPrimaryAction(snapshot)
    guard LocalModelActionPlanner.canRun(action, snapshot: snapshot) else {
      return
    }

    runLocalModelPrimaryAction(action)
  }

  func modelSetupCalloutSecondaryActionTitle() -> String? {
    LocalModelActionPlanner.secondaryTitle(
      for: LocalModelActionPlanner.setupSecondaryAction(localModelActionSnapshot())
    )
  }

  func canRunModelSetupCalloutSecondaryAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.setupSecondaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runModelSetupCalloutSecondaryAction() {
    guard canRunModelSetupCalloutSecondaryAction() else {
      return
    }

    cancelModelDownload()
  }

  func runtimePrimaryActionTitle() -> String? {
    let snapshot = sessionActionSnapshot()
    return SessionActionPlanner.runtimePrimaryActionTitle(
      for: SessionActionPlanner.runtimePrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func canRunRuntimePrimaryAction() -> Bool {
    let snapshot = sessionActionSnapshot()
    return SessionActionPlanner.canRunRuntimePrimaryAction(
      SessionActionPlanner.runtimePrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runRuntimePrimaryAction() {
    let snapshot = sessionActionSnapshot()
    guard let action = SessionActionPlanner.runtimePrimaryAction(snapshot),
          SessionActionPlanner.canRunRuntimePrimaryAction(action, snapshot: snapshot)
    else {
      return
    }

    switch action {
    case .launchRuntime:
      launchRuntime()
    case .cancelTurn:
      cancelActiveTurn()
    }
  }

  func canLaunchRuntime() -> Bool {
    SessionActionPlanner.canLaunchRuntime(sessionActionSnapshot())
  }

  func canOpenWorkspace() -> Bool {
    SessionActionPlanner.canOpenWorkspace(sessionActionSnapshot())
  }

  func canCreateThread() -> Bool {
    SessionActionPlanner.canCreateThread(sessionActionSnapshot())
  }

  func canInstallPlugin() -> Bool {
    SessionActionPlanner.canInstallPlugin(sessionActionSnapshot())
  }

  func canSendDraftMessage() -> Bool {
    SessionActionPlanner.canSendDraftMessage(sessionActionSnapshot())
  }

  func canCancelActiveTurn() -> Bool {
    SessionActionPlanner.canCancelActiveTurn(sessionActionSnapshot())
  }

  func canRespondToApproval(approvalID: String) -> Bool {
    SessionActionPlanner.canRespondToApproval(
      approvalID: approvalID,
      snapshot: sessionActionSnapshot()
    )
  }

  func canUseComposer() -> Bool {
    SessionActionPlanner.canUseComposer(sessionActionSnapshot())
  }

  private func firstRequestSuggestion(id: String) -> ComposerSuggestionSummary? {
    guard canUseComposer(),
          trimmedDraftMessage.isEmpty,
          selectedThreadIsWaitingForFirstMessage()
    else {
      return nil
    }

    return FirstRequestPromptPresenter.suggestion(id: id, workspaceDisplayName: workspace?.displayName)
  }

  private func useFirstRequestSuggestion(id: String) {
    guard let suggestion = firstRequestSuggestion(id: id) else {
      return
    }

    draftMessage = suggestion.message
  }

  func canSearchWorkspace() -> Bool {
    WorkspaceSearchSession.canSearch(workspaceSearchSnapshot())
  }

  func searchWorkspace() {
    guard canSearchWorkspace() else {
      return
    }

    let query = WorkspaceSearchSession.trimmedQuery(workspaceSearchQuery)
    let requestToken = workspaceSearchSession.begin(query: query)

    isWorkspaceSearching = true
    workspaceSearchStatus = requestToken.status
    Task {
      do {
        let matches = try await runtimeBridge.searchWorkspace(query: requestToken.query)
        guard workspaceSearchSession.isCurrent(requestToken) else {
          return
        }
        guard !WorkspaceSearchSession.queryChanged(
          currentQuery: workspaceSearchQuery,
          token: requestToken
        ) else {
          finishChangedWorkspaceSearch()
          return
        }
        workspaceSearchResults = WorkspaceSearchSession.matchSummaries(from: matches)
        workspaceSearchStatus = WorkspaceSearchSession.successStatus(
          query: requestToken.query,
          matchCount: matches.count
        )
      } catch {
        guard workspaceSearchSession.isCurrent(requestToken) else {
          return
        }
        guard !WorkspaceSearchSession.queryChanged(
          currentQuery: workspaceSearchQuery,
          token: requestToken
        ) else {
          finishChangedWorkspaceSearch()
          return
        }
        workspaceSearchResults = []
        workspaceSearchStatus = WorkspaceSearchSession.failureStatus(error: error)
      }
      workspaceSearchSession.finish()
      isWorkspaceSearching = false
    }
  }

  func clearWorkspaceSearch() {
    workspaceSearchQuery = ""
    resetWorkspaceSearch()
  }

  func workspaceSearchEmptyStateSummary() -> String? {
    WorkspaceSearchSession.emptyStateSummary(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      query: workspaceSearchQuery,
      status: workspaceSearchStatus,
      isSearching: isWorkspaceSearching,
      hasResults: !workspaceSearchResults.isEmpty
    )
  }

  func workspaceSearchOverflowSummary() -> String? {
    WorkspaceSearchSession.overflowSummary(resultCount: workspaceSearchResults.count)
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

  func installPlugin() {
    guard canInstallPlugin() else {
      return
    }

    guard let url = AppFilePicker.choosePluginSource() else {
      return
    }

    let preview: PluginInstallPreview
    do {
      preview = try PluginInstallInspector.preview(
        for: url,
        installRootPath: runtimeBridge.localPluginInstallRootPath()
      )
    } catch {
      let repairHint = PluginInstallDialogPresenter.repairHint(for: error)
      appendEntry(
        to: selectedThreadID,
        TimelineEventPresenter.pluginInstallPreviewFailed(
          error: error,
          repairHint: repairHint
        )
      )
      return
    }

    guard PluginInstallDialogPresenter.confirmInstall(preview: preview) else {
      runtimeDetail = "Plugin install was cancelled."
      return
    }

    Task {
      do {
        let installedPlugin = try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginInstalled(installedPlugin, preview: preview)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginInstallFailed(error: error)
        )
      }
    }
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
    threads.insert(thread, at: 0)
    threadTimelines[thread.id] = TimelineEntryFactory.defaultTimeline(for: thread.title)
    threadPendingApprovalIDs[thread.id] = Set<String>()
    selectThread(id: thread.id)
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

  func setPluginEnabled(pluginID: String, enabled: Bool) {
    guard canSetPluginEnabled(pluginID: pluginID) else {
      return
    }

    Task {
      do {
        let updatedPlugin = try await runtimeBridge.setPluginEnabled(pluginID: pluginID, enabled: enabled)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginUpdated(updatedPlugin, enabled: enabled)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginUpdateFailed(pluginID: pluginID, error: error)
        )
      }
    }
  }

  func removePlugin(pluginID: String) {
    guard canRemovePlugin(pluginID: pluginID),
          let plugin = plugins.first(where: { $0.id == pluginID })
    else {
      return
    }

    guard PluginInstallDialogPresenter.confirmRemoval(plugin: plugin) else {
      runtimeDetail = "Plugin removal was cancelled."
      return
    }

    Task {
      do {
        let removedPlugin = try await runtimeBridge.removePlugin(manifestPath: plugin.manifestPath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginRemoved(removedPlugin)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginRemovalFailed(pluginID: pluginID, error: error)
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
    selectedThreadID = id
    syncVisibleTimeline()

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

  func saveWorkspaceMemoryNote() {
    guard let draft = MemoryActionPlanner.preparedDraft(memoryActionSnapshot()) else {
      return
    }

    Task {
      do {
        let note = try await runtimeBridge.createMemoryNote(title: draft.title, body: draft.body)
        memoryNoteTitle = ""
        memoryNoteBody = ""
        await refreshMemoryState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.memoryNoteSaved(note)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.memoryNoteFailed(error: error)
        )
      }
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
          let activeTurnThreadID
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

  func selectedThreadTitle() -> String {
    SessionOverviewPresenter.selectedThreadTitle(sessionOverviewSnapshot())
  }

  func selectedThreadPreview() -> String {
    SessionOverviewPresenter.selectedThreadPreview(sessionOverviewSnapshot())
  }

  func selectTimelineEntry(id: TimelineEntry.ID) {
    selectedEntryID = id
  }

  func selectedEntryTitle() -> String {
    TimelineInspectorPresenter.selectedEntryTitle(timelineInspectorSnapshot())
  }

  func selectedEntryBody() -> String {
    TimelineInspectorPresenter.selectedEntryBody(timelineInspectorSnapshot())
  }

  func selectedEntryMetadata() -> String {
    TimelineInspectorPresenter.selectedEntryMetadata(timelineInspectorSnapshot())
  }

  func selectedDiffSummary() -> String? {
    TimelineInspectorPresenter.selectedDiffSummary(timelineInspectorSnapshot())
  }

  func selectedDiffLines() -> [DiffLineSummary] {
    TimelineInspectorPresenter.selectedDiffLines(timelineInspectorSnapshot())
  }

  func selectedEntryMemorySummary() -> String? {
    TimelineInspectorPresenter.selectedEntryMemorySummary(timelineInspectorSnapshot())
  }

  func shouldShowSelectedEntryInspector() -> Bool {
    SessionOverviewPresenter.shouldShowSelectedEntryInspector(sessionOverviewSnapshot())
  }

  func workspaceDisplayName() -> String {
    SessionOverviewPresenter.workspaceDisplayName(sessionOverviewSnapshot())
  }

  func workspacePath() -> String {
    SessionOverviewPresenter.workspacePath(sessionOverviewSnapshot())
  }

  func modelDisplayName() -> String {
    LocalModelStatusPresenter.displayName(localModelStatusSnapshot())
  }

  func modelStatusSummary() -> String {
    LocalModelStatusPresenter.statusSummary(localModelStatusSnapshot())
  }

  func modelActionSummary() -> String {
    localModelSetupGuidance().actionSummary
  }

  func showsModelActivity() -> Bool {
    LocalModelStatusPresenter.showsActivity(localModelStatusSnapshot())
  }

  func isModelActionBlocking() -> Bool {
    LocalModelOperationPresenter.isActionBlocking(localModelOperationSnapshot())
  }

  func localModelPrimaryActionTitle() -> String? {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.primaryTitle(
      for: LocalModelActionPlanner.managerPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func canRunLocalModelPrimaryAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.managerPrimaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runLocalModelPrimaryAction() {
    let snapshot = localModelActionSnapshot()
    let action = LocalModelActionPlanner.managerPrimaryAction(snapshot)
    guard LocalModelActionPlanner.canRun(action, snapshot: snapshot) else {
      return
    }

    runLocalModelPrimaryAction(action)
  }

  func localModelSecondaryActionTitle() -> String? {
    LocalModelActionPlanner.secondaryTitle(
      for: LocalModelActionPlanner.managerSecondaryAction(localModelActionSnapshot())
    )
  }

  func canRunLocalModelSecondaryAction() -> Bool {
    let snapshot = localModelActionSnapshot()
    return LocalModelActionPlanner.canRun(
      LocalModelActionPlanner.managerSecondaryAction(snapshot),
      snapshot: snapshot
    )
  }

  func runLocalModelSecondaryAction() {
    guard canRunLocalModelSecondaryAction() else {
      return
    }

    cancelModelDownload()
  }

  func modelDetailSummary() -> String {
    LocalModelStatusPresenter.detailSummary(localModelStatusSnapshot())
  }

  func modelSourceSummary() -> String {
    LocalModelStatusPresenter.sourceSummary(localModelStatusSnapshot())
  }

  func modelMetricsSummary() -> String {
    LocalModelStatusPresenter.metricsSummary(localModelStatusSnapshot())
  }

  func modelReadinessSummary() -> String {
    LocalModelStatusPresenter.readinessSummary(localModelStatusSnapshot())
  }

  func modelInstallHintSummary() -> String {
    LocalModelStatusPresenter.installHintSummary(localModelStatusSnapshot())
  }

  func modelSuggestedPathSummary() -> String {
    LocalModelStatusPresenter.suggestedPathSummary(localModelStatusSnapshot())
  }

  func modelArtifactPathSummary() -> String {
    LocalModelStatusPresenter.artifactPathSummary(localModelStatusSnapshot())
  }

  func modelManagerSummary() -> String {
    LocalModelOperationPresenter.managerSummary(localModelOperationSnapshot())
  }

  func localModelManagerRuleSummary() -> String {
    LocalModelStatusPresenter.managerRuleSummary(localModelStatusSnapshot())
  }

  func shouldShowModelDownloadProgress() -> Bool {
    LocalModelStatusPresenter.shouldShowDownloadProgress(localModelStatusSnapshot())
  }

  func modelDownloadProgressValue() -> Double? {
    LocalModelStatusPresenter.downloadProgressValue(localModelStatusSnapshot())
  }

  func modelDownloadProgressSummary() -> String {
    LocalModelStatusPresenter.downloadProgressSummary(localModelStatusSnapshot())
  }

  func localModelStatusSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.localModelStatusSummary(model, snapshot: localModelStatusSnapshot())
  }

  func localModelChoiceSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.localModelChoiceSummary(
      model,
      snapshot: localModelStatusSnapshot(),
      defaultModelID: LocalModelCatalog.defaultFirstUseModelID
    )
  }

  func defaultModelDownloadButtonTitle() -> String {
    LocalModelStatusPresenter.defaultDownloadButtonTitle(localModelStatusSnapshot())
  }

  func localModelDownloadButtonTitle(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.downloadButtonTitle(model, snapshot: localModelStatusSnapshot())
  }

  func localModelTagSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.tagSummary(model)
  }

  func localModelPathSummary(_ model: LocalModelSummary) -> String {
    LocalModelStatusPresenter.pathSummary(model)
  }

  func canDownloadRecommendedModel(modelID: String) -> Bool {
    guard let model = localModel(for: modelID),
          !model.downloaded
    else {
      return false
    }

    return localModelDownloadRequestPlan(for: model).canStart
  }

  func canActivateRecommendedModel(modelID: String) -> Bool {
    guard runtimeState != .launching,
          !hasActiveOrPendingTurn(),
          !modelDownloadCoordinator.isDownloading,
          !modelDownloadState.hasPausedDownload
    else {
      return false
    }
    guard let model = localModel(for: modelID) else {
      return false
    }

    return model.downloaded && !model.active
  }

  func canResetActiveLocalModel() -> Bool {
    runtimeState != .launching
      && !hasActiveOrPendingTurn()
      && !modelDownloadCoordinator.isDownloading
      && runtimeBridge.activeLocalModelPath() != nil
  }

  func canCancelModelDownload() -> Bool {
    modelDownloadCoordinator.isDownloading || modelDownloadState.hasPausedDownload
  }

  func canPauseModelDownload() -> Bool {
    modelDownloadCoordinator.canPause
  }

  func pauseModelDownload() {
    guard canPauseModelDownload() else {
      return
    }

    runtimeDetail = LocalModelDownloadControlPlanner.pauseDetail(
      activeModelID: modelDownloadState.activeModelID,
      models: localModels
    )
    modelDownloadCoordinator.pauseActiveTransfer()
  }

  func cancelModelDownload() {
    guard canCancelModelDownload(),
          let cancelPlan = LocalModelDownloadControlPlanner.cancelPlan(
            isDownloading: modelDownloadCoordinator.isDownloading,
            activeModelID: modelDownloadState.activeModelID,
            pausedModelID: modelDownloadState.pausedModelID,
            models: localModels
          )
    else {
      return
    }

    applyModelDownloadCancelPlan(cancelPlan)
  }

  private func applyModelDownloadCancelPlan(_ cancelPlan: LocalModelDownloadCancelPlan) {
    switch cancelPlan.mode {
    case .running:
      runtimeDetail = cancelPlan.runtimeDetail
      modelDownloadCoordinator.cancelActiveDownload()
    case .orphanedPaused(let modelID):
      clearPausedModelDownload()
      removeIncompleteModelFile(modelID: modelID)
      modelDownloadState.clearProgress()
      runtimeDetail = cancelPlan.runtimeDetail
      refreshLocalModelCatalog()
    case .paused(let model):
      applyModelDownloadInterruptionPlan(
        LocalModelDownloadInterruptionPlanner.cancellationPlan(model: model),
        model: model
      )
      refreshLocalModelCatalog()
    }
  }

  func downloadRecommendedModel(modelID: String, activateAfterDownload: Bool = false) {
    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    let requestPlan = localModelDownloadRequestPlan(for: model)
    guard let downloadURL = requestPlan.downloadURL else {
      runtimeDetail = requestPlan.blockedDetail ?? "The selected local model is not ready to download."
      return
    }

    let startPlan = LocalModelDownloadStartPlanner.plan(
      model: model,
      sourceURL: downloadURL,
      pausedModelID: modelDownloadState.pausedModelID,
      resumeData: modelDownloadCoordinator.resumeData,
      currentProgress: modelDownloadState.progress
    )
    let sessionState = LocalModelDownloadSessionPlanner.startState(
      model: model,
      startPlan: startPlan,
      activateAfterDownload: activateAfterDownload,
      isLocalModelReady: isLocalModelReady()
    )
    applyModelDownloadStartState(sessionState)
    appendModelEvent(
      title: startPlan.timelineTitle,
      body: startPlan.timelineBody,
      model: model,
      attributes: startPlan.attributes
    )
    startModelDownloadTask(
      model: model,
      downloadURL: downloadURL,
      startPlan: startPlan,
      shouldActivateAfterDownload: sessionState.shouldActivateAfterDownload
    )
  }

  private func startModelDownloadTask(
    model: LocalModelSummary,
    downloadURL: URL,
    startPlan: LocalModelDownloadStartPlan,
    shouldActivateAfterDownload: Bool
  ) {
    let task = Task {
      defer {
        modelDownloadState.clearActiveDownload()
        modelDownloadCoordinator.finishActiveDownload()
        refreshLocalModelCatalog()
      }
      do {
        runtimeDetail = startPlan.runtimeDetail
        try await downloadModelFile(
          from: downloadURL,
          resumeData: startPlan.resumeData,
          modelID: model.id,
          expectedBytes: model.sizeBytes,
          to: URL(fileURLWithPath: model.installPath)
        )
        let completionState: LocalModelDownloadSessionCompletionState
        do {
          completionState = try LocalModelDownloadSessionPlanner.completionState(
            model: model,
            sourceURL: downloadURL,
            activationRequested: shouldActivateAfterDownload,
            hasActiveOrPendingTurn: hasActiveOrPendingTurn()
          )
        } catch LocalModelActivationPreparationError.integrityCheckFailed(let error) {
          removeIncompleteModelFile(modelID: model.id)
          throw LocalModelActivationPreparationError.integrityCheckFailed(error)
        }

        if let preparedActivation = completionState.preparedActivation {
          runtimeBridge.configureActiveLocalModel(
            manifestPath: preparedActivation.manifestPath,
            modelPath: model.installPath
          )
        }

        applyModelDownloadCompletionPlan(completionState.completionPlan, model: model)
      } catch {
        let interruptionPlan = LocalModelDownloadInterruptionPlanner.plan(model: model, error: error)
        applyModelDownloadInterruptionPlan(interruptionPlan, model: model)
      }
    }
    modelDownloadCoordinator.start(task)
  }

  func activateRecommendedModel(modelID: String) {
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = "Finish or cancel the current local turn before switching models."
      return
    }

    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    guard model.downloaded else {
      runtimeDetail = "Download \(model.displayName) before using it."
      return
    }

    do {
      let preparedActivation = try LocalModelActivationPreparer.prepare(model: model)
      runtimeBridge.configureActiveLocalModel(
        manifestPath: preparedActivation.manifestPath,
        modelPath: model.installPath
      )
      selectedSetupModelID = model.id
      refreshLocalModelCatalog()
      applyLocalModelActivationPlan(
        LocalModelActivationPlanner.selectionPlan(
          model: model,
          manifestPath: preparedActivation.manifestPath
        )
      )
    } catch {
      applyLocalModelActivationFailure(
        LocalModelActivationPlanner.failurePlan(error: error),
        model: model
      )
    }
  }

  func resetActiveLocalModel() {
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = "Finish or cancel the current local turn before resetting model selection."
      return
    }

    runtimeBridge.clearActiveLocalModel()
    refreshLocalModelCatalog()
    applyLocalModelActivationPlan(LocalModelActivationPlanner.resetPlan())
  }

  func revealRecommendedModel(modelID: String) {
    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      model.installPath,
      successDetail: "Revealed \(model.displayName)."
    )
  }

  func revealSuggestedModelDirectory() {
    runtimeDetail = FileRevealService.revealSuggestedPath(
      metricKey: "suggestedModelPath",
      modelHealth: modelHealth,
      successDetail: "Opened the suggested local model folder."
    )
  }

  func canRevealSuggestedModelDirectory() -> Bool {
    FileRevealService.hasSuggestedPath(metricKey: "suggestedModelPath", modelHealth: modelHealth)
  }

  func revealSuggestedBinaryDirectory() {
    runtimeDetail = FileRevealService.revealSuggestedPath(
      metricKey: "suggestedBinaryPath",
      modelHealth: modelHealth,
      successDetail: "Opened the suggested llama.cpp binary folder."
    )
  }

  func canRevealSuggestedBinaryDirectory() -> Bool {
    FileRevealService.hasSuggestedPath(metricKey: "suggestedBinaryPath", modelHealth: modelHealth)
  }

  func canDownloadLocalModel() -> Bool {
    LocalModelSelectedActionPlanner.canRun(selectedLocalModelAction())
  }

  func downloadLocalModel() {
    switch selectedLocalModelAction() {
    case .activate(let modelID):
      activateRecommendedModel(modelID: modelID)
    case .download(let modelID):
      downloadRecommendedModel(modelID: modelID, activateAfterDownload: true)
    case .blocked(let detail):
      runtimeDetail = detail
    }
  }

  func canBootstrapModelPackMetadata() -> Bool {
    runtimeState == .ready && !modelDownloadCoordinator.isDownloading
  }

  func bootstrapModelPackMetadata() {
    guard canBootstrapModelPackMetadata() else {
      runtimeDetail = "Launch the runtime before preparing local model metadata."
      return
    }

    Task {
      do {
        let result = try await runtimeBridge.bootstrapModelPack()
        await refreshModelHealthState()
        let copiedSummary = result.copiedFiles.isEmpty
          ? "Pack metadata was already present."
          : "Prepared \(result.copiedFiles.count) local model metadata file(s)."
        runtimeDetail = "\(copiedSummary) Manifest: \(result.manifestPath)"
      } catch {
        runtimeDetail = "Model metadata bootstrap failed: \(error.localizedDescription)"
      }
    }
  }

  func pluginCountSummary() -> String {
    PluginDashboardPresenter.pluginCountSummary(pluginDashboardSnapshot())
  }

  func localPluginCountSummary() -> String {
    PluginDashboardPresenter.localPluginCountSummary(pluginDashboardSnapshot())
  }

  func pluginDetailSummary() -> String {
    PluginDashboardPresenter.pluginDetailSummary(pluginDashboardSnapshot())
  }

  func pluginPermissionCountSummary() -> String {
    PluginDashboardPresenter.permissionCountSummary(pluginDashboardSnapshot())
  }

  func pluginPermissionDetailSummary() -> String {
    PluginDashboardPresenter.permissionDetailSummary(pluginDashboardSnapshot())
  }

  func pluginPermissionPreview() -> [PluginSummary] {
    PluginDashboardPresenter.permissionPreview(pluginDashboardSnapshot())
  }

  func invalidPluginCountSummary() -> String {
    PluginDashboardPresenter.invalidPluginCountSummary(pluginDashboardSnapshot())
  }

  func invalidPluginDetailSummary() -> String {
    PluginDashboardPresenter.invalidPluginDetailSummary(pluginDashboardSnapshot())
  }

  func invalidPlugins() -> [PluginSummary] {
    PluginDashboardPresenter.invalidPlugins(pluginDashboardSnapshot())
  }

  func isRemovablePlugin(_ plugin: PluginSummary) -> Bool {
    PluginActionPlanner.isRemovable(plugin)
  }

  func canSetPluginEnabled(pluginID: String) -> Bool {
    PluginActionPlanner.canSetEnabled(pluginID: pluginID, snapshot: pluginActionSnapshot())
  }

  func canRemovePlugin(pluginID: String) -> Bool {
    PluginActionPlanner.canRemove(pluginID: pluginID, snapshot: pluginActionSnapshot())
  }

  func revealPluginManifest(pluginID: String) {
    guard let plugin = plugins.first(where: { $0.id == pluginID }) else {
      runtimeDetail = "Plugin manifest path is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      plugin.manifestPath,
      successDetail: "Revealed \(plugin.displayName) manifest."
    )
  }

  func pluginRegistryCountSummary() -> String {
    PluginDashboardPresenter.registryCountSummary(pluginDashboardSnapshot())
  }

  func pluginRegistryDetailSummary() -> String {
    PluginDashboardPresenter.registryDetailSummary(pluginDashboardSnapshot())
  }

  func pluginCapabilityPreview() -> [PluginCapabilitySummary] {
    PluginDashboardPresenter.capabilityPreview(pluginDashboardSnapshot())
  }

  func pluginConnectorCountSummary() -> String {
    PluginDashboardPresenter.connectorCountSummary(pluginDashboardSnapshot())
  }

  func pluginConnectorDetailSummary() -> String {
    PluginDashboardPresenter.connectorDetailSummary(pluginDashboardSnapshot())
  }

  func pluginConnectorPreview() -> [PluginConnectorSummary] {
    PluginDashboardPresenter.connectorPreview(pluginDashboardSnapshot())
  }

  func pluginCommandCountSummary() -> String {
    PluginDashboardPresenter.commandCountSummary(pluginDashboardSnapshot())
  }

  func pluginCommandDetailSummary() -> String {
    PluginDashboardPresenter.commandDetailSummary(pluginDashboardSnapshot())
  }

  func pluginHookCountSummary() -> String {
    PluginDashboardPresenter.hookCountSummary(pluginDashboardSnapshot())
  }

  func pluginHookDetailSummary() -> String {
    PluginDashboardPresenter.hookDetailSummary(pluginDashboardSnapshot())
  }

  func memoryCountSummary() -> String {
    MemoryPresenter.countSummary(memorySnapshot())
  }

  func memoryDetailSummary() -> String {
    MemoryPresenter.detailSummary(memorySnapshot())
  }

  func memoryLatestSummary() -> String {
    MemoryPresenter.latestSummary(memorySnapshot())
  }

  func canSaveWorkspaceMemoryNote() -> Bool {
    MemoryActionPlanner.canSave(memoryActionSnapshot())
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

  private func hasActiveOrPendingTurn() -> Bool {
    activeTurnID != nil || pendingTurnRequest.isPending
  }

  func isPendingApproval(_ entry: TimelineEntry) -> Bool {
    guard entry.kind == .approval,
          let approvalID = entry.attributes["approvalId"]
    else {
      return false
    }

    return canRespondToApproval(approvalID: approvalID)
  }

  func approvalID(for entry: TimelineEntry) -> String? {
    entry.attributes["approvalId"]
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
    threadPendingApprovalIDs[threadID] = TimelineMutationState.pendingApprovalIDs(from: approvals)
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
    threads = TimelineMutationState.threadsByRefreshingPreview(
      threads,
      threadID: threadID,
      preview: preview
    )
  }

  private func refreshWorkspaceThreadSelection(
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
    threads = workspaceThreads
    threadTimelines = TimelineMutationState.threadTimelines(
      for: workspaceThreads,
      existingTimelines: threadTimelines
    )
    threadPendingApprovalIDs = TimelineMutationState.pendingApprovalIDsByRetainingWorkspaceThreads(
      threadPendingApprovalIDs,
      workspaceThreads: workspaceThreads
    )

    selectedThreadID = TimelineMutationState.selectedThreadID(
      workspaceThreads: workspaceThreads,
      currentSelectionID: selectedThreadID
    )
    syncVisibleTimeline()
  }

  private func resetToWelcomeThread() {
    let welcomeState = TimelineSessionState.welcomeState()
    let welcomeThread = welcomeState.thread
    let welcomeTimeline = welcomeState.timeline

    threads = [welcomeThread]
    threadTimelines = [welcomeThread.id: welcomeTimeline]
    selectedThreadID = welcomeThread.id
    timeline = welcomeTimeline
    selectedEntryID = welcomeTimeline.first?.id
  }

  private func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
    guard let threadID else {
      timeline.insert(entry, at: 0)
      if selectedEntryID == nil {
        selectedEntryID = entry.id
      }
      return
    }

    let entries = TimelineMutationState.entriesByAppending(
      entry: entry,
      existingEntries: threadTimelines[threadID],
      fallbackTitle: threadTitle(for: threadID)
    )
    applyThreadEntries(threadID: threadID, entries: entries)
  }

  private func applyThreadEntries(threadID: String, entries: [TimelineEntry]) {
    threadTimelines[threadID] = entries

    if let visibleState = TimelineMutationState.visibleTimelineUpdate(
      updatedThreadID: threadID,
      selectedThreadID: selectedThreadID,
      entries: entries,
      previousSelectionID: selectedEntryID
    ) {
      timeline = visibleState.timeline
      selectedEntryID = visibleState.selectedEntryID
    }
  }

  private func syncVisibleTimeline() {
    let visibleState = TimelineMutationState.visibleTimeline(
      selectedThreadID: selectedThreadID,
      threadTimelines: threadTimelines,
      threads: threads,
      previousSelectionID: selectedEntryID
    )
    timeline = visibleState.timeline
    selectedEntryID = visibleState.selectedEntryID

    if let selectedThreadID {
      threadTimelines[selectedThreadID] = visibleState.timeline
    }
  }

  private func threadTitle(for threadID: String) -> String {
    TimelineSessionState.threadTitle(for: threadID, threads: threads)
  }

  private func loadThreadHistory(threadID: String) async {
    do {
      let result = try await runtimeBridge.readThread(threadID: threadID)
      let entries = TimelineEntryFactory.runtimeEntries(
        from: result.items,
        existingEntries: threadTimelines[threadID],
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

  private func selectedEntry() -> TimelineEntry? {
    TimelineSessionState.selectedEntry(
      selectedEntryID: selectedEntryID,
      timeline: timeline
    )
  }

  private func updateActiveTurn(threadID: String, activeTurnID: String?) {
    let activeTurnSelection = TimelineMutationState.activeTurnSelection(
      currentActiveTurnID: self.activeTurnID,
      currentActiveTurnThreadID: activeTurnThreadID,
      threadID: threadID,
      runtimeActiveTurnID: activeTurnID
    )
    self.activeTurnID = activeTurnSelection.activeTurnID
    activeTurnThreadID = activeTurnSelection.activeTurnThreadID
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

  private func refreshModelHealthState(serverLabel: String? = nil) async {
    let modelRefresh = await RuntimeStateLoader.refreshModelHealth(
      using: runtimeBridge,
      serverLabel: serverLabel
    )
    modelHealth = modelRefresh.modelHealth
    if let runtimeDetail = modelRefresh.runtimeDetail {
      self.runtimeDetail = runtimeDetail
    }
    refreshLocalModelCatalog()
    await refreshRuntimeReadiness()
    announceFirstRequestReadyIfNeeded()
  }

  private func refreshRuntimeReadiness() async {
    runtimeReadiness = await RuntimeStateLoader.refreshRuntimeReadiness(using: runtimeBridge)
  }

  private func refreshLocalModelCatalog() {
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
    localModels = refreshPlan.models
    selectedSetupModelID = refreshPlan.selectedSetupModelID
  }

  private func runtimeHeaderSnapshot() -> RuntimeHeaderSnapshot {
    let isModelReady = isLocalModelReady()
    let modelSetupSummary = runtimeState == .ready && !isModelReady
      ? localModelSetupGuidance().summary
      : ""
    return RuntimeHeaderSnapshot(
      runtimeState: runtimeState,
      runtimeDetail: runtimeDetail,
      modelSetupSummary: modelSetupSummary,
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      isWorkspaceSearching: isWorkspaceSearching,
      hasModelDownload: modelDownloadState.hasActiveDownload,
      hasPausedModelDownload: modelDownloadState.hasPausedDownload
    )
  }

  private func setupProgressSnapshot() -> SetupProgressSnapshot {
    let isModelReady = isLocalModelReady()
    let modelReadinessDetail = runtimeState == .ready && !isModelReady
      ? localModelSetupGuidance().readinessDetail
      : ""
    return SetupProgressSnapshot(
      readyStepCount: setupReadyStepCount(),
      stepCount: SetupFlowState.stepCount,
      runtimeState: runtimeState,
      showsRuntimeActivity: showsRuntimeActivity(),
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraft: !trimmedDraftMessage.isEmpty,
      modelReadinessDetail: modelReadinessDetail
    )
  }

  private func runtimeReadinessSnapshot() -> RuntimeReadinessSnapshot {
    let isModelReady = isLocalModelReady()
    let modelGuidance = runtimeState == .ready
      ? localModelSetupGuidance()
      : nil
    return RuntimeReadinessSnapshot(
      runtimeState: runtimeState,
      modelReadinessDetail: modelGuidance?.readinessDetail ?? "Waiting",
      modelTone: modelGuidance?.tone ?? .neutral,
      workspaceDisplayName: workspace?.displayName,
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty
    )
  }

  private func inspectorSessionSnapshot() -> InspectorSessionSnapshot {
    return InspectorSessionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      workspaceDisplayName: workspace?.displayName,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      selectedThreadTitle: selectedThreadTitle(),
      hasActiveTurn: hasActiveOrPendingTurn(),
      setupReadyStepCount: setupReadyStepCount(),
      setupStepCount: SetupFlowState.stepCount,
      setupProgressDetail: setupProgressDetail(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      runtimeReadinessStatus: runtimeReadiness?.status
    )
  }

  private func setupCalloutSnapshot() -> SetupCalloutSnapshot {
    let modelProgressDetail: String?
    if shouldShowModelDownloadProgress() {
      modelProgressDetail = modelDownloadProgressSummary()
    } else {
      modelProgressDetail = nil
    }

    return SetupCalloutSnapshot(
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      modelGuidance: localModelSetupGuidance(),
      modelProgressDetail: modelProgressDetail,
      modelPrimaryActionTitle: modelSetupCalloutActionTitle(),
      modelSecondaryActionTitle: modelSetupCalloutSecondaryActionTitle()
    )
  }

  private func setupCalloutActionSnapshot() -> SetupCalloutActionSnapshot {
    SetupCalloutActionSnapshot(
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      canRunModelSetupAction: canRunModelSetupCalloutAction(),
      canRunModelSetupSecondaryAction: canRunModelSetupCalloutSecondaryAction(),
      canOpenWorkspace: canOpenWorkspace(),
      canCreateThread: canCreateThread()
    )
  }

  private func pluginDashboardSnapshot() -> PluginDashboardSnapshot {
    return PluginDashboardSnapshot(
      plugins: plugins,
      registrySummary: pluginCapabilityRegistrySummary,
      capabilities: pluginCapabilities,
      connectors: pluginConnectors,
      commands: pluginCommands,
      hooks: pluginHooks
    )
  }

  private func memorySnapshot() -> MemorySnapshot {
    return MemorySnapshot(
      status: memoryStatus,
      notes: memoryNotes
    )
  }

  private func pluginActionSnapshot() -> PluginActionSnapshot {
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

  private func timelineInspectorSnapshot() -> TimelineInspectorSnapshot {
    return TimelineInspectorSnapshot(selectedEntry: selectedEntry())
  }

  private func localModelStatusSnapshot() -> LocalModelStatusSnapshot {
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

  private func setupFlowSnapshot() -> SetupFlowSnapshot {
    SetupFlowSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage()
    )
  }

  private func setupReadyStepCount() -> Int {
    SetupFlowState.readyStepCount(setupFlowSnapshot())
  }

  private func hasRuntimeThreadSelection() -> Bool {
    TimelineSessionState.hasRuntimeThreadSelection(
      selectedThreadID: selectedThreadID,
      threads: threads,
      workspace: workspace
    )
  }

  private func sessionActionSnapshot() -> SessionActionSnapshot {
    let pendingApprovalIDs = selectedThreadID.map {
      threadPendingApprovalIDs[$0, default: Set<String>()]
    } ?? Set<String>()

    return SessionActionSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      isLocalModelReady: isLocalModelReady(),
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveOrPendingTurn: hasActiveOrPendingTurn(),
      hasCancelableTurn: (activeTurnID != nil && activeTurnThreadID != nil)
        || pendingTurnRequest.canCancel,
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      pendingApprovalIDs: pendingApprovalIDs
    )
  }

  private func sessionOverviewSnapshot() -> SessionOverviewSnapshot {
    let selectedThread = selectedThreadID.flatMap { threadID in
      threads.first(where: { $0.id == threadID })
    }

    return SessionOverviewSnapshot(
      selectedThread: selectedThread,
      workspace: workspace,
      selectedEntry: selectedEntry()
    )
  }

  private func workspaceSearchSnapshot() -> WorkspaceSearchSnapshot {
    WorkspaceSearchSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      isSearching: isWorkspaceSearching,
      query: workspaceSearchQuery
    )
  }

  private func runtimeReadinessActionSnapshot() -> RuntimeReadinessActionSnapshot {
    RuntimeReadinessActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      canLaunchRuntime: canLaunchRuntime(),
      canRunModelSetupAction: canRunModelSetupCalloutAction(),
      canOpenWorkspace: canOpenWorkspace(),
      canCreateThread: canCreateThread(),
      canUseComposer: canUseComposer(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage(),
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      hasFirstRequestSuggestion: firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID) != nil,
      runtimeLaunchButtonTitle: runtimeLaunchButtonTitle(),
      modelSetupActionTitle: modelSetupCalloutActionTitle()
    )
  }

  private var trimmedDraftMessage: String {
    draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  private func selectedThreadIsWaitingForFirstMessage() -> Bool {
    TimelineSessionState.isWaitingForFirstMessage(
      selectedThreadID: selectedThreadID,
      threadTimelines: threadTimelines,
      visibleTimeline: timeline
    )
  }

  private func shouldAnnotateLaunchWithSetupEvents() -> Bool {
    SetupFlowState.shouldAnnotateLaunch(setupFlowSnapshot())
  }

  private func selectedSetupModel() -> LocalModelSummary? {
    localModel(for: selectedSetupModelID)
      ?? localModel(for: LocalModelCatalog.defaultFirstUseModelID)
      ?? localModels.first
  }

  private func localModel(for modelID: String?) -> LocalModelSummary? {
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

  private func localModelSetupGuidance() -> LocalModelSetupGuidance {
    LocalModelOperationPresenter.setupGuidance(localModelOperationSnapshot())
  }

  private func localModelRequiredTimelineSummary() -> String {
    localModelSetupGuidance().summary
  }

  private func announceFirstRequestReadyIfNeeded() {
    guard SetupFlowState.isCoreReadyForFirstRequest(setupFlowSnapshot()),
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
          selectedThreadIsWaitingForFirstMessage(),
          !announcedSetupCompleteThreadIDs.contains(threadID)
    else {
      return
    }

    announcedSetupCompleteThreadIDs.insert(threadID)
    appendEntry(
      to: threadID,
      TimelineEventPresenter.firstRequestReady()
    )
  }

  private func localModelOperationSnapshot() -> LocalModelOperationSnapshot {
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

  private func localModelActionSnapshot() -> LocalModelActionSnapshot {
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

  private func runLocalModelPrimaryAction(_ action: LocalModelPrimaryAction?) {
    guard let action else {
      return
    }

    switch action {
    case .pauseDownload:
      pauseModelDownload()
    case .continueDownload(let modelID):
      downloadRecommendedModel(modelID: modelID, activateAfterDownload: !isLocalModelReady())
    case .downloadSelectedModel:
      downloadLocalModel()
    case .blockedDownload:
      break
    case .bootstrapModelPackMetadata:
      bootstrapModelPackMetadata()
    }
  }

  private func selectedLocalModelAction() -> LocalModelSelectedAction {
    let model = selectedSetupModel()
    let requestPlan: LocalModelDownloadRequestPlan?
    if let model, !model.active, !model.downloaded {
      requestPlan = localModelDownloadRequestPlan(for: model)
    } else {
      requestPlan = nil
    }

    return LocalModelSelectedActionPlanner.action(
      LocalModelSelectedActionSnapshot(
        selectedModel: model,
        requestPlan: requestPlan,
        canActivateDownloadedModel: model.map { canActivateRecommendedModel(modelID: $0.id) } ?? false,
        activationBlockedDetail: selectedModelActivationBlockedDetail()
      )
    )
  }

  private func selectedModelActivationBlockedDetail() -> String {
    if hasActiveOrPendingTurn() {
      return "Finish or cancel the current local turn before switching models."
    }
    if runtimeState == .launching {
      return "Wait for the runtime to finish launching before switching models."
    }
    if modelDownloadCoordinator.isDownloading {
      return "Finish, pause, or cancel the current model download before switching models."
    }
    if modelDownloadState.hasPausedDownload {
      return "Continue or cancel the paused model download before switching models."
    }

    return "The selected local model is not ready to use."
  }

  private func localModelDownloadRequestPlan(
    for model: LocalModelSummary
  ) -> LocalModelDownloadRequestPlan {
    localModelDownloadRequestPlanCache.plan(
      for: model,
      isDownloadRunning: modelDownloadCoordinator.isDownloading,
      pausedModelID: modelDownloadState.pausedModelID,
      resumeData: modelDownloadCoordinator.resumeData,
      currentProgress: modelDownloadState.progress
    )
  }

  private func selectedSetupModelDownloadBlockedDetail() -> String? {
    guard let model = selectedSetupModel(),
          !model.downloaded
    else {
      return nil
    }

    return localModelDownloadRequestPlan(for: model).blockedDetail
  }

  private func appendModelEvent(
    title: String,
    body: String,
    model: LocalModelSummary,
    kind: TimelineEntry.Kind = .system,
    attributes: [String: String] = [:]
  ) {
    var eventAttributes = attributes
    eventAttributes["modelId"] = model.id
    eventAttributes["modelPath"] = model.installPath
    eventAttributes["modelLicense"] = model.license
    appendEntry(
      to: selectedThreadID,
      TimelineEntryFactory.entry(
        kind: kind,
        title: title,
        body: body,
        attributes: eventAttributes
      )
    )
  }

  private func applyModelDownloadStartState(_ sessionState: LocalModelDownloadSessionStartState) {
    modelDownloadState.applyStart(sessionState)
    if sessionState.clearsPausedState {
      LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
    }
  }

  private func applyModelDownloadCompletionPlan(
    _ plan: LocalModelDownloadCompletionPlan,
    model: LocalModelSummary
  ) {
    switch plan.mode {
    case .activated, .waitingForTurn:
      selectedSetupModelID = model.id
    case .downloadedOnly:
      break
    }

    runtimeDetail = plan.runtimeDetail
    modelDownloadState.clearProgress()
    refreshLocalModelCatalog()
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelDownloaded(plan)
    )

    if let relaunchRunningDetail = plan.relaunchRunningDetail,
       let relaunchIdleDetail = plan.relaunchIdleDetail
    {
      relaunchRuntimeIfNeeded(
        runningDetail: relaunchRunningDetail,
        idleDetail: relaunchIdleDetail
      )
    }
  }

  private func applyModelDownloadInterruptionPlan(
    _ plan: LocalModelDownloadInterruptionPlan,
    model: LocalModelSummary
  ) {
    switch plan.mode {
    case .paused(let resumeData):
      modelDownloadCoordinator.resumeData = resumeData
      modelDownloadState.markPaused(modelID: model.id)
      persistPausedModelDownload(modelID: model.id, resumeData: resumeData)
    case .cancelled, .failed:
      if plan.clearsPausedState {
        clearPausedModelDownload()
      }
      if plan.removesPartialFile {
        removeIncompleteModelFile(modelID: model.id)
      }
    }

    if plan.clearsProgress {
      modelDownloadState.clearProgress()
    }
    runtimeDetail = plan.runtimeDetail
    appendModelEvent(
      title: plan.timelineTitle,
      body: plan.timelineBody,
      model: model,
      kind: plan.timelineKind,
      attributes: plan.attributes
    )
  }

  private func applyLocalModelActivationPlan(_ plan: LocalModelActivationPlan) {
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelActivated(plan)
    )
    relaunchRuntimeIfNeeded(
      runningDetail: plan.relaunchRunningDetail,
      idleDetail: plan.relaunchIdleDetail
    )
  }

  private func applyLocalModelActivationFailure(
    _ plan: LocalModelActivationFailurePlan,
    model: LocalModelSummary
  ) {
    if plan.removesModelFile {
      removeIncompleteModelFile(modelID: model.id)
    }
    if plan.refreshesCatalog {
      refreshLocalModelCatalog()
    }
    runtimeDetail = plan.runtimeDetail
  }

  private func relaunchRuntimeIfNeeded(runningDetail: String, idleDetail: String) {
    let plan = RuntimeRelaunchPlanner.plan(
      runtimeState: runtimeState,
      runningDetail: runningDetail,
      idleDetail: idleDetail
    )
    runtimeDetail = plan.runtimeDetail

    switch plan.action {
    case .stopAndLaunch:
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
    case .stopAndLaunchAfterCurrentLaunchSettles:
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      Task {
        for _ in 0..<10 {
          if runtimeState != .launching {
            break
          }
          try? await Task.sleep(nanoseconds: 200_000_000)
        }
        if runtimeState == .launching {
          runtimeDetail = plan.launchTimeoutDetail ?? idleDetail
          return
        }
        launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
      }
    case .updateIdleDetail:
      break
    }
  }

  private func handleRuntimeConnectionStateChange(_ state: RuntimeBridge.ConnectionState, detail: String) {
    let plan = RuntimeConnectionStateReducer.plan(
      RuntimeConnectionStateSnapshot(
        previousState: runtimeState,
        nextState: state,
        detail: detail,
        lastFailureDetail: lastRuntimeFailureDetail
      )
    )
    runtimeState = state
    runtimeDetail = detail

    if plan.clearsActiveTurnState {
      activeTurnID = nil
      activeTurnThreadID = nil
      pendingTurnRequest.clear()
    }

    if plan.clearsModelReadinessState {
      modelHealth = nil
      runtimeReadiness = nil
    }

    if plan.resetsLastFailureDetail {
      lastRuntimeFailureDetail = nil
    }

    if plan.shouldAppendFailureNotice {
      appendEntry(
        to: selectedThreadID,
        TimelineEventPresenter.runtimeDisconnected(detail: detail)
      )
    }

    if let updatedLastFailureDetail = plan.updatedLastFailureDetail {
      lastRuntimeFailureDetail = updatedLastFailureDetail
    }
  }

  private func applyRuntimeLaunchFailure(_ error: Error) {
    runtimeState = .failed
    runtimeDetail = error.localizedDescription
    modelHealth = nil
    runtimeReadiness = nil
    memoryStatus = nil
    memoryNotes = []
    plugins = []
    pluginCapabilityRegistrySummary = nil
    pluginCapabilities = []
    pluginConnectors = []
    pluginCommands = []
    pluginHooks = []
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.runtimeLaunchFailed(error: error)
    )
  }

  private func downloadModelFile(
    from sourceURL: URL,
    resumeData: Data?,
    modelID: String,
    expectedBytes: Int64,
    to targetURL: URL
  ) async throws {
    let transfer = ModelDownloadTransfer(targetURL: targetURL) { [weak self] bytesReceived, totalBytes in
      Task { @MainActor [weak self] in
        self?.updateModelDownloadProgress(
          modelID: modelID,
          bytesReceived: bytesReceived,
          totalBytes: totalBytes > 0 ? totalBytes : expectedBytes
        )
      }
    }
    modelDownloadCoordinator.attachTransfer(transfer)
    try await transfer.start(from: sourceURL, resumeData: resumeData)
  }

  private func updateModelDownloadProgress(
    modelID: String,
    bytesReceived: Int64,
    totalBytes: Int64
  ) {
    guard let progress = LocalModelDownloadProgressUpdater.updatedProgress(
      LocalModelDownloadProgressUpdate(
        modelID: modelID,
        activeModelID: modelDownloadState.activeModelID,
        currentProgress: modelDownloadState.progress,
        bytesReceived: bytesReceived,
        totalBytes: totalBytes,
        updatedAt: Date()
      )
    ) else {
      return
    }

    modelDownloadState.updateProgress(progress)
    runtimeDetail = modelDownloadProgressSummary()
  }

  private func clearPausedModelDownload() {
    modelDownloadState.clearPausedDownload()
    LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
  }

  private func persistPausedModelDownload(modelID: String, resumeData: Data) {
    LocalModelDownloadStateStore.persistPausedDownload(
      modelID: modelID,
      resumeData: resumeData,
      progress: modelDownloadState.progress
    )
  }

  private func removeIncompleteModelFile(modelID: String) {
    LocalModelDownloadStateStore.removeIncompleteModelFile(modelID: modelID, models: localModels)
  }

  private func resetWorkspaceSearch() {
    workspaceSearchResults = []
    workspaceSearchStatus = workspaceSearchSession.resetStatus(hasWorkspace: workspace != nil)
    isWorkspaceSearching = false
  }

  private func finishChangedWorkspaceSearch() {
    workspaceSearchResults = []
    workspaceSearchStatus = workspaceSearchSession.changedQueryStatus()
    isWorkspaceSearching = false
  }

  private func isRestorableWorkspacePath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(false)
    return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
      && isDirectory.boolValue
  }

  private func refreshMemoryState() async {
    let memoryRefresh = await MemoryStateLoader.refresh(using: runtimeBridge)
    applyMemoryStateRefresh(memoryRefresh, clearsMissing: false)
  }

  private func applyMemoryStateRefresh(
    _ memoryRefresh: MemoryStateRefresh,
    clearsMissing: Bool
  ) {
    if clearsMissing || memoryRefresh.status != nil {
      memoryStatus = memoryRefresh.status
    }
    if clearsMissing || memoryRefresh.notes != nil {
      memoryNotes = memoryRefresh.notes ?? []
    }
  }

  private func memoryActionSnapshot() -> MemoryActionSnapshot {
    MemoryActionSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      title: memoryNoteTitle,
      body: memoryNoteBody
    )
  }

  private func refreshPluginState() async {
    let pluginRefresh = await PluginStateLoader.refresh(using: runtimeBridge)

    if let refreshedPlugins = pluginRefresh.plugins {
      plugins = refreshedPlugins
    }
    if let registrySummary = pluginRefresh.registrySummary {
      pluginCapabilityRegistrySummary = registrySummary
    }
    if let capabilities = pluginRefresh.capabilities {
      pluginCapabilities = capabilities
    }
    if let commands = pluginRefresh.commands {
      pluginCommands = commands
    }
    if let connectors = pluginRefresh.connectors {
      pluginConnectors = connectors
    }
    if let hooks = pluginRefresh.hooks {
      pluginHooks = hooks
    }
    await refreshRuntimeReadiness()
  }

  private func applyRuntimeThreadUpdate(_ state: RuntimeBridge.RuntimeThreadState) {
    let entries = TimelineEntryFactory.runtimeEntries(
      from: state.items,
      existingEntries: threadTimelines[state.id],
      fallbackTitle: threadTitle(for: state.id)
    )

    applyThreadEntries(threadID: state.id, entries: entries)
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)
  }

}
