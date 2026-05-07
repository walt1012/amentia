import Combine
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  @Published private var timelineState: TimelineRuntimeState
  @Published private var runtimeConnectionState: RuntimeConnectionStateStore
  @Published var draftMessage: String
  @Published var workspace: WorkspaceSummary?
  @Published var workspaceSearchState: WorkspaceSearchRuntimeState
  @Published private var localModelReadinessState: LocalModelReadinessState
  @Published private var modelDownloadState: LocalModelDownloadRuntimeState
  @Published private var memoryState: MemoryRuntimeState
  @Published private var pluginState: PluginRuntimeState

  let runtimeBridge: RuntimeBridge
  private let pendingTurnRequest = PendingTurnRequestState()
  let workspaceSearchSession = WorkspaceSearchSession()
  private let modelDownloadCoordinator: LocalModelDownloadCoordinator
  private let localModelDownloadRequestPlanCache = LocalModelDownloadRequestPlanCache()

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

  func launchRuntime(launchDetail: String = "Launching local runtime") {
    guard runtimeState != .launching else {
      return
    }

    if runtimeState == .ready {
      runtimeBridge.stopRuntime(detail: "Relaunching local runtime...")
    }

    runtimeState = .launching
    runtimeDetail = launchDetail
    updateRuntimeConnectionState { state in
      state.clearLastFailureDetail()
    }

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

  func saveWorkspaceMemoryNote() {
    guard let draft = MemoryActionPlanner.preparedDraft(memoryActionSnapshot()) else {
      return
    }

    Task {
      do {
        let note = try await runtimeBridge.createMemoryNote(title: draft.title, body: draft.body)
        updateMemoryState { state in
          state.clearDraft()
        }
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
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelEvent(
        title: startPlan.timelineTitle,
        body: startPlan.timelineBody,
        model: model,
        attributes: startPlan.attributes
      )
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
    updateTimelineState { state in
      state.applyWorkspaceThreads(workspaceThreads)
    }
  }

  private func resetToWelcomeThread() {
    updateTimelineState { state in
      state.resetToWelcomeState(TimelineSessionState.welcomeState())
    }
  }

  private func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
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

  private func selectedEntry() -> TimelineEntry? {
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

  private func refreshModelHealthState(serverLabel: String? = nil) async {
    let modelRefresh = await RuntimeStateLoader.refreshModelHealth(
      using: runtimeBridge,
      serverLabel: serverLabel
    )
    updateLocalModelReadinessState { state in
      state.modelHealth = modelRefresh.modelHealth
    }
    if let runtimeDetail = modelRefresh.runtimeDetail {
      self.runtimeDetail = runtimeDetail
    }
    refreshLocalModelCatalog()
    await refreshRuntimeReadiness()
    announceFirstRequestReadyIfNeeded()
  }

  private func refreshRuntimeReadiness() async {
    let readiness = await RuntimeStateLoader.refreshRuntimeReadiness(using: runtimeBridge)
    updateLocalModelReadinessState { state in
      state.runtimeReadiness = readiness
    }
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
    updateLocalModelReadinessState { state in
      state.applyCatalogRefresh(refreshPlan)
    }
    AppPreferences.storeSelectedSetupModelID(refreshPlan.selectedSetupModelID)
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
    timelineState.hasRuntimeThreadSelection(workspace: workspace)
  }

  private func sessionActionSnapshot() -> SessionActionSnapshot {
    return SessionActionSnapshot(
      runtimeState: runtimeState,
      hasWorkspace: workspace != nil,
      isLocalModelReady: isLocalModelReady(),
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveOrPendingTurn: hasActiveOrPendingTurn(),
      hasCancelableTurn: timelineState.hasCancelableRuntimeTurn || pendingTurnRequest.canCancel,
      hasDraftMessage: !trimmedDraftMessage.isEmpty,
      pendingApprovalIDs: timelineState.selectedPendingApprovalIDs
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
    timelineState.isWaitingForFirstMessage()
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
          !timelineState.hasAnnouncedSetupComplete(for: threadID)
    else {
      return
    }

    updateTimelineState { state in
      state.markSetupCompleteAnnounced(threadID: threadID)
    }
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
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelEvent(
        title: plan.timelineTitle,
        body: plan.timelineBody,
        model: model,
        kind: plan.timelineKind,
        attributes: plan.attributes
      )
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
        lastFailureDetail: runtimeConnectionState.lastFailureDetail
      )
    )
    updateRuntimeConnectionState { runtimeConnectionState in
      runtimeConnectionState.applyConnectionUpdate(
        state: state,
        detail: detail,
        plan: plan
      )
    }

    if plan.clearsActiveTurnState {
      updateTimelineState { state in
        state.activeTurnID = nil
        state.activeTurnThreadID = nil
      }
      pendingTurnRequest.clear()
    }

    if plan.clearsModelReadinessState {
      updateLocalModelReadinessState { state in
        state.clearRuntimeReadiness()
      }
    }

    if plan.shouldAppendFailureNotice {
      appendEntry(
        to: selectedThreadID,
        TimelineEventPresenter.runtimeDisconnected(detail: detail)
      )
    }
  }

  private func applyRuntimeLaunchFailure(_ error: Error) {
    runtimeState = .failed
    runtimeDetail = error.localizedDescription
    updateLocalModelReadinessState { state in
      state.clearRuntimeReadiness()
    }
    updateMemoryState { state in
      state.resetRuntimeData()
    }
    updatePluginState { state in
      state.reset()
    }
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

  func updateWorkspaceSearchState(
    _ update: (inout WorkspaceSearchRuntimeState) -> Void
  ) {
    var nextState = workspaceSearchState
    update(&nextState)
    workspaceSearchState = nextState
  }

  private func updateTimelineState(_ update: (inout TimelineRuntimeState) -> Void) {
    var nextState = timelineState
    update(&nextState)
    timelineState = nextState
  }

  private func updateRuntimeConnectionState(
    _ update: (inout RuntimeConnectionStateStore) -> Void
  ) {
    var nextState = runtimeConnectionState
    update(&nextState)
    runtimeConnectionState = nextState
  }

  private func updateLocalModelReadinessState(
    _ update: (inout LocalModelReadinessState) -> Void
  ) {
    var nextState = localModelReadinessState
    update(&nextState)
    localModelReadinessState = nextState
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
    updateMemoryState { state in
      state.apply(memoryRefresh, clearsMissing: clearsMissing)
    }
  }

  private func updateMemoryState(_ update: (inout MemoryRuntimeState) -> Void) {
    var nextState = memoryState
    update(&nextState)
    memoryState = nextState
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
    updatePluginState { state in
      state.apply(pluginRefresh)
    }
    await refreshRuntimeReadiness()
  }

  private func updatePluginState(_ update: (inout PluginRuntimeState) -> Void) {
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
