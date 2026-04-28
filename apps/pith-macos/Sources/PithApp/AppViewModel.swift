import AppKit
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
  @Published var modelDownloadID: String?
  @Published var pausedModelDownloadID: String?
  @Published private var modelDownloadProgress: ModelDownloadProgress?
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
    let initialTimeline = TimelineEntryFactory.welcomeTimeline()

    let initialThreads = [
      ThreadSummary(
        id: "local-welcome",
        title: "Welcome to Pith",
        preview: "Open a workspace to begin the local agent loop.",
        workspaceRootPath: nil,
        workspaceDisplayName: nil
      ),
    ]

    let initialLocalModels = LocalModelCatalog.summaries(
      storageRootPath: runtimeBridge.localModelStorageRootPath(),
      activeModelPath: runtimeBridge.activeLocalModelPath()
    )
    let pausedDownload = LocalModelCatalog.loadPausedDownload(matching: initialLocalModels)
    let initialSelectedSetupModelID =
      pausedDownload?.modelID
      ?? AppPreferences.storedSelectedSetupModelID(matching: initialLocalModels)
      ?? LocalModelCatalog.defaultFirstUseModelID

    self.runtimeBridge = runtimeBridge
    self.runtimeState = runtimeBridge.connectionState
    if pausedDownload == nil {
      self.runtimeDetail = "Runtime not launched"
    } else {
      self.runtimeDetail = "Runtime not launched | paused model download available"
    }
    self.draftMessage = ""
    self.workspace = nil
    self.workspaceSearchQuery = ""
    self.workspaceSearchResults = []
    self.workspaceSearchStatus = "Search the open workspace by text."
    self.isWorkspaceSearching = false
    self.modelHealth = nil
    self.runtimeReadiness = nil
    self.localModels = initialLocalModels
    self.selectedSetupModelID = initialSelectedSetupModelID
    self.modelDownloadID = nil
    self.pausedModelDownloadID = pausedDownload?.modelID
    self.modelDownloadProgress = LocalModelCatalog.restoredProgress(
      from: pausedDownload,
      localModels: initialLocalModels
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
    self.threadTimelines = ["local-welcome": initialTimeline]
    self.threadPendingApprovalIDs = [:]
    self.lastRuntimeFailureDetail = nil
    self.modelDownloadCoordinator = LocalModelDownloadCoordinator(resumeData: pausedDownload?.resumeData)
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
        let session = try await runtimeBridge.launchAndInitialize(launchDetail: launchDetail)
        let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
        let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()
        var currentWorkspace = try? await runtimeBridge.currentWorkspace()
        var restoredWorkspace = false
        var workspaceRestoreError: Error?
        var skippedWorkspaceRestorePath: String?
        if currentWorkspace == nil, let lastWorkspacePath = AppPreferences.storedLastWorkspacePath() {
          if isRestorableWorkspacePath(lastWorkspacePath) {
            do {
              currentWorkspace = try await runtimeBridge.openWorkspace(path: lastWorkspacePath)
              restoredWorkspace = true
            } catch {
              workspaceRestoreError = error
            }
          } else {
            skippedWorkspaceRestorePath = lastWorkspacePath
            AppPreferences.clearLastWorkspacePath()
          }
        }
        let threadList = try await runtimeBridge.listThreads()

        runtimeState = .ready
        await refreshModelHealthState(serverLabel: "\(session.serverName) \(session.serverVersion)")

        if let runtimeMemoryStatus {
          memoryStatus = RuntimeSummaryMapper.memoryStatusSummary(from: runtimeMemoryStatus)
        } else {
          memoryStatus = nil
        }
        memoryNotes = (runtimeMemoryNotes ?? []).map {
          RuntimeSummaryMapper.memoryNoteSummary(from: $0)
        }

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
          try await refreshWorkspaceThreadSelection(from: threadList, createIfEmpty: isLocalModelReady())
        } else {
          resetToWelcomeThread()
        }
        await refreshRuntimeReadiness()
        let shouldAnnotateSetupLaunch = shouldAnnotateLaunchWithSetupEvents()
        if shouldAnnotateSetupLaunch {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Runtime Connected",
              body: "Connected to \(session.serverName) \(session.serverVersion) over stdio.",
              attributes: [:]
            )
          )
        }
        if restoredWorkspace, let currentWorkspace {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Workspace Restored",
              body: "Restored \(currentWorkspace.displayName) at \(currentWorkspace.rootPath).",
              attributes: [
                "workspacePath": currentWorkspace.rootPath
              ]
            )
          )
        }
        if let skippedWorkspaceRestorePath {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Workspace Restore Skipped",
              body: "The last workspace no longer exists. Open a workspace to continue.",
              attributes: [
                "workspacePath": skippedWorkspaceRestorePath
              ]
            )
          )
        }
        if let workspaceRestoreError {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Workspace Restore Failed",
              body: workspaceRestoreError.localizedDescription,
              attributes: [:]
            )
          )
        }
        if let runtimeModel = modelHealth {
          if isLocalModelReady() {
            if shouldAnnotateSetupLaunch {
              appendEntry(
                to: selectedThreadID,
                TimelineEntry(
                  id: UUID().uuidString,
                  kind: .system,
                  title: "Local Model Ready",
                  body:
                    "\(runtimeModel.displayName) is running in \(runtimeModel.backend) mode with status \(runtimeModel.status).",
                  attributes: [
                    "modelId": runtimeModel.packID,
                    "modelBackend": runtimeModel.backend,
                    "modelStatus": runtimeModel.status,
                    "modelSource": runtimeModel.source,
                  ]
                )
              )
            }
          } else {
            appendEntry(
              to: selectedThreadID,
              TimelineEntry(
                id: UUID().uuidString,
                kind: .warning,
                title: "Local Model Required",
                body: localModelRequiredTimelineSummary(),
                attributes: [
                  "modelId": runtimeModel.packID,
                  "modelBackend": runtimeModel.backend,
                  "modelStatus": runtimeModel.status,
                  "modelSource": runtimeModel.source,
                ]
              )
            )
          }
        } else {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Local Model Required",
              body: localModelRequiredTimelineSummary(),
              attributes: [
                "modelStatus": "unavailable"
              ]
            )
          )
        }
        announceFirstRequestReadyIfNeeded()
      } catch {
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
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Runtime Launch Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func runtimeLaunchButtonTitle() -> String {
    switch runtimeState {
    case .ready:
      return "Relaunch Runtime"
    case .failed:
      return "Relaunch Runtime"
    case .launching:
      return "Launching Runtime"
    case .disconnected:
      return "Launch Runtime"
    }
  }

  func shouldShowRuntimeToolbarAction() -> Bool {
    runtimeState == .disconnected || runtimeState == .failed
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
    switch step.id {
    case "runtime":
      if runtimeState == .disconnected || runtimeState == .failed {
        return runtimeLaunchButtonTitle()
      }
    case "model":
      if runtimeState == .ready && !isLocalModelReady() {
        return modelSetupCalloutActionTitle()
      }
    case "workspace":
      if runtimeState == .ready && workspace == nil {
        return "Open"
      }
    case "thread":
      if runtimeState == .ready
        && isLocalModelReady()
        && workspace != nil
        && !hasRuntimeThreadSelection()
      {
        return "New"
      }
    case "first-request":
      if runtimeState == .ready,
         canUseComposer(),
         selectedThreadIsWaitingForFirstMessage()
      {
        return trimmedDraftMessage.isEmpty ? "Use Prompt" : "Send"
      }
    default:
      return nil
    }

    return nil
  }

  func canRunReadinessStepAction(_ step: ReadinessStepSummary) -> Bool {
    switch step.id {
    case "runtime":
      return (runtimeState == .disconnected || runtimeState == .failed) && canLaunchRuntime()
    case "model":
      return runtimeState == .ready && !isLocalModelReady() && canRunModelSetupCalloutAction()
    case "workspace":
      return runtimeState == .ready && workspace == nil && canOpenWorkspace()
    case "thread":
      return runtimeState == .ready
        && isLocalModelReady()
        && workspace != nil
        && !hasRuntimeThreadSelection()
        && canCreateThread()
    case "first-request":
      guard runtimeState == .ready,
            canUseComposer(),
            selectedThreadIsWaitingForFirstMessage()
      else {
        return false
      }

      return !trimmedDraftMessage.isEmpty
        || firstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID) != nil
    default:
      return false
    }
  }

  func runReadinessStepAction(_ step: ReadinessStepSummary) {
    guard canRunReadinessStepAction(step) else {
      return
    }

    switch step.id {
    case "runtime":
      launchRuntime()
    case "model":
      runModelSetupCalloutAction()
    case "workspace":
      openWorkspace()
    case "thread":
      createThread()
    case "first-request":
      if trimmedDraftMessage.isEmpty {
        useFirstRequestSuggestion(id: FirstRequestPromptPresenter.mapWorkspaceID)
      } else {
        sendDraftMessage()
      }
    default:
      return
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
      || modelDownloadID != nil
      || pausedModelDownloadID != nil
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
    if !isLocalModelReady() {
      return canRunModelSetupCalloutAction()
    }
    if workspace == nil {
      return canOpenWorkspace()
    }
    if !hasRuntimeThreadSelection() {
      return canCreateThread()
    }

    return false
  }

  func runSetupCalloutAction() {
    guard canRunSetupCalloutAction() else {
      return
    }

    if !isLocalModelReady() {
      runModelSetupCalloutAction()
      return
    }
    if workspace == nil {
      openWorkspace()
      return
    }
    if !hasRuntimeThreadSelection() {
      createThread()
    }
  }

  func setupCalloutSecondaryActionTitle() -> String? {
    SetupCalloutPresenter.secondaryActionTitle(setupCalloutSnapshot())
  }

  func canRunSetupCalloutSecondaryAction() -> Bool {
    if !isLocalModelReady() {
      return canRunModelSetupCalloutSecondaryAction()
    }

    return false
  }

  func runSetupCalloutSecondaryAction() {
    guard canRunSetupCalloutSecondaryAction() else {
      return
    }

    if !isLocalModelReady() {
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
      && modelDownloadID == nil
      && pausedModelDownloadID == nil
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
    switch runtimeState {
    case .disconnected, .failed, .launching:
      return runtimeLaunchButtonTitle()
    case .ready:
      if canCancelActiveTurn() {
        return "Cancel Turn"
      }
      return nil
    }
  }

  func canRunRuntimePrimaryAction() -> Bool {
    switch runtimeState {
    case .disconnected, .failed:
      return canLaunchRuntime()
    case .launching:
      return false
    case .ready:
      if canCancelActiveTurn() {
        return true
      }
      return false
    }
  }

  func runRuntimePrimaryAction() {
    switch runtimeState {
    case .disconnected, .failed:
      launchRuntime()
    case .launching:
      return
    case .ready:
      if canCancelActiveTurn() {
        cancelActiveTurn()
      }
    }
  }

  func canLaunchRuntime() -> Bool {
    runtimeState != .launching
  }

  func canOpenWorkspace() -> Bool {
    runtimeState == .ready && !hasActiveOrPendingTurn()
  }

  func canCreateThread() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && isLocalModelReady()
      && !hasActiveOrPendingTurn()
  }

  func canInstallPlugin() -> Bool {
    runtimeState == .ready && !hasActiveOrPendingTurn()
  }

  func canSendDraftMessage() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && isLocalModelReady()
      && hasRuntimeThreadSelection()
      && !isTurnStreaming()
      && !trimmedDraftMessage.isEmpty
  }

  func canCancelActiveTurn() -> Bool {
    runtimeState == .ready
      && (
        (activeTurnID != nil && activeTurnThreadID != nil)
          || pendingTurnRequest.canCancel
      )
  }

  func canRespondToApproval(approvalID: String) -> Bool {
    guard let selectedThreadID else {
      return false
    }

    return runtimeState == .ready
      && hasRuntimeThreadSelection()
      && threadPendingApprovalIDs[selectedThreadID, default: Set<String>()].contains(approvalID)
  }

  func canUseComposer() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && isLocalModelReady()
      && hasRuntimeThreadSelection()
      && !hasActiveOrPendingTurn()
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
    runtimeState == .ready
      && workspace != nil
      && !isWorkspaceSearching
      && !workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
  }

  func searchWorkspace() {
    guard canSearchWorkspace() else {
      return
    }

    let query = workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
    let requestToken = workspaceSearchSession.begin(query: query)

    isWorkspaceSearching = true
    workspaceSearchStatus = requestToken.status
    Task {
      do {
        let matches = try await runtimeBridge.searchWorkspace(query: requestToken.query)
        guard workspaceSearchSession.isCurrent(requestToken) else {
          return
        }
        guard workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines) == requestToken.query else {
          finishChangedWorkspaceSearch()
          return
        }
        workspaceSearchResults = matches.enumerated().map { index, match in
          WorkspaceSearchMatchSummary(
            id: "\(match.relativePath):\(match.lineNumber):\(index)",
            relativePath: match.relativePath,
            lineNumber: match.lineNumber,
            line: match.line
          )
        }
        workspaceSearchStatus = WorkspaceSearchSession.successStatus(
          query: requestToken.query,
          matchCount: matches.count
        )
      } catch {
        guard workspaceSearchSession.isCurrent(requestToken) else {
          return
        }
        guard workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines) == requestToken.query else {
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

    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.allowsMultipleSelection = false
    panel.prompt = "Open Workspace"
    panel.message = "Choose a local workspace for Pith to inspect."

    guard panel.runModal() == .OK, let url = panel.url else {
      return
    }

    Task {
      do {
        let openedWorkspace = try await runtimeBridge.openWorkspace(path: url.path)
        workspace = WorkspaceSummary(
          rootPath: openedWorkspace.rootPath,
          displayName: openedWorkspace.displayName
        )
        resetWorkspaceSearch()
        AppPreferences.storeLastWorkspacePath(openedWorkspace.rootPath)
        await refreshMemoryState()
        let threadList = try await runtimeBridge.listThreads()
        try await refreshWorkspaceThreadSelection(from: threadList, createIfEmpty: isLocalModelReady())
        await refreshRuntimeReadiness()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Workspace Opened",
            body: "Opened \(openedWorkspace.displayName) at \(openedWorkspace.rootPath).",
            attributes: [:]
          )
        )
        announceFirstRequestReadyIfNeeded()
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Workspace Open Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func installPlugin() {
    guard canInstallPlugin() else {
      return
    }

    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = true
    panel.allowsMultipleSelection = false
    panel.prompt = "Install Plugin"
    panel.message = "Choose a plugin folder or a pith-plugin.json manifest."

    guard panel.runModal() == .OK, let url = panel.url else {
      return
    }

    let preview: PluginInstallPreview
    do {
      preview = try PluginInstallInspector.preview(
        for: url,
        installRootPath: runtimeBridge.localPluginInstallRootPath()
      )
    } catch {
      let repairHint = pluginInstallRepairHint(for: error)
      let body = repairHint.isEmpty ? error.localizedDescription : "\(error.localizedDescription)\n\nRepair Hint: \(repairHint)"
      appendEntry(
        to: selectedThreadID,
        TimelineEntry(
          id: UUID().uuidString,
          kind: .warning,
          title: "Plugin Install Preview Failed",
          body: body,
          attributes: [:]
        )
      )
      return
    }

    guard confirmPluginInstall(preview: preview) else {
      runtimeDetail = "Plugin install was cancelled."
      return
    }

    Task {
      do {
        let installedPlugin = try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Plugin Installed",
            body:
              "\(installedPlugin.displayName) is now available in the local plugin manager.\nSource: \(preview.sourcePath)\nInstalled To: \(preview.installPath)",
            attributes: [
              "pluginId": installedPlugin.id,
              "pluginStatus": installedPlugin.status,
              "pluginManifestPath": installedPlugin.manifestPath,
              "pluginSourcePath": preview.sourcePath,
              "pluginInstallPath": preview.installPath,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Install Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
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
        threads.insert(thread, at: 0)
        threadTimelines[thread.id] = TimelineEntryFactory.defaultTimeline(for: thread.title)
        threadPendingApprovalIDs[thread.id] = Set<String>()
        selectThread(id: thread.id)
        await loadThreadHistory(threadID: thread.id)
        appendEntry(
          to: thread.id,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Thread Created",
            body: "Created \(thread.title) in the local runtime.",
            attributes: [:]
          )
        )
        announceFirstRequestReadyIfNeeded()
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Thread Creation Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func sendDraftMessage() {
    let message = trimmedDraftMessage

    guard runtimeState == .ready,
          workspace != nil,
          isLocalModelReady(),
          hasRuntimeThreadSelection(),
          !message.isEmpty,
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
          !hasActiveOrPendingTurn()
    else {
      return
    }

    draftMessage = ""
    runtimeDetail = "Generating local response..."
    let requestID = pendingTurnRequest.begin(threadID: threadID)

    let task = Task {
      defer {
        pendingTurnRequest.clear(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.startTurn(threadID: threadID, message: message)
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
        updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
        refreshThreadPreview(
          threadID: result.threadID,
          preview: result.activeTurnID == nil ? "\(result.turnID) ready" : "Streaming response"
        )
      } catch {
        if Task.isCancelled {
          runtimeDetail = "Local turn request cancelled."
          refreshThreadPreview(threadID: threadID, preview: "Cancelled response")
          appendEntry(
            to: threadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Turn Cancelled",
              body: "The pending local turn request was cancelled before streaming started.",
              attributes: [:]
            )
          )
          return
        }
        if draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
          draftMessage = message
        }
        runtimeDetail = error.localizedDescription
        appendEntry(
          to: threadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Turn Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
    pendingTurnRequest.bind(task: task, requestID: requestID)
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
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
        await refreshMemoryState()
        await loadThreadHistory(threadID: result.threadID)
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Approval Response Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
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
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: enabled ? "Plugin Enabled" : "Plugin Disabled",
            body: "\(updatedPlugin.displayName) is now \(enabled ? "enabled" : "disabled").",
            attributes: [
              "pluginId": updatedPlugin.id,
              "pluginStatus": updatedPlugin.status,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Update Failed",
            body: error.localizedDescription,
            attributes: [
              "pluginId": pluginID
            ]
          )
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

    guard confirmPluginRemoval(plugin: plugin) else {
      runtimeDetail = "Plugin removal was cancelled."
      return
    }

    Task {
      do {
        let removedPlugin = try await runtimeBridge.removePlugin(manifestPath: plugin.manifestPath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Plugin Removed",
            body:
              "\(removedPlugin.displayName) was removed from the local plugin catalog.\nRemoved Path: \(removedPlugin.removedPath)",
            attributes: [
              "pluginId": removedPlugin.pluginID,
              "removedPath": removedPlugin.removedPath,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Removal Failed",
            body: error.localizedDescription,
            attributes: [
              "pluginId": pluginID
            ]
          )
        )
      }
    }
  }

  func runPluginCommand(commandID: String) {
    let snapshot = pluginActionSnapshot()
    if PluginActionPlanner.commandNeedsExecutionContract(commandID: commandID, snapshot: snapshot) {
      runtimeDetail = "Plugin command needs an execution contract before it can run."
      return
    }

    guard PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot),
          let threadID = selectedThreadID
    else {
      return
    }

    runtimeDetail = "Running local plugin command..."
    let requestID = pendingTurnRequest.begin(threadID: threadID)

    let task = Task {
      defer {
        pendingTurnRequest.clear(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.runPluginCommand(threadID: threadID, commandID: commandID)
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updatePendingApprovals(threadID: result.threadID, approvals: result.pendingApprovals)
        updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
        refreshThreadPreview(
          threadID: result.threadID,
          preview: result.activeTurnID == nil ? "\(result.turnID) ready" : "Streaming response"
        )
        await refreshMemoryState()
      } catch {
        if Task.isCancelled {
          runtimeDetail = "Local plugin command cancelled."
          refreshThreadPreview(threadID: threadID, preview: "Cancelled plugin command")
          appendEntry(
            to: threadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .warning,
              title: "Plugin Command Cancelled",
              body: "The pending local plugin command was cancelled before streaming started.",
              attributes: [:]
            )
          )
          return
        }
        runtimeDetail = error.localizedDescription
        appendEntry(
          to: threadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Plugin Command Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
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
          TimelineEntry(
            id: UUID().uuidString,
            kind: .system,
            title: "Memory Note Saved",
            body: "Saved built-in workspace note \(note.title).",
            attributes: [
              "memoryNoteId": note.id,
              "memoryScope": note.scope,
              "memorySource": note.source,
            ]
          )
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Memory Note Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
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
        appendItemsToTimeline(threadID: result.threadID, items: result.items)
        updateActiveTurn(threadID: result.threadID, activeTurnID: result.activeTurnID)
        refreshThreadPreview(threadID: activeTurnThreadID, preview: "Cancelled response")
        await loadThreadHistory(threadID: result.threadID)
      } catch {
        appendEntry(
          to: activeTurnThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Turn Cancel Failed",
            body: error.localizedDescription,
            attributes: [:]
          )
        )
      }
    }
  }

  func selectedThreadTitle() -> String {
    guard let selectedThreadID,
          let thread = threads.first(where: { $0.id == selectedThreadID })
    else {
      return "No Thread Selected"
    }

    return thread.title
  }

  func selectedThreadPreview() -> String {
    guard let selectedThreadID,
          let thread = threads.first(where: { $0.id == selectedThreadID })
    else {
      return "Select a thread to inspect its runtime state."
    }

    return thread.preview
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
    guard let entry = selectedEntry() else {
      return false
    }

    if entry.kind != .system {
      return true
    }

    return !entry.id.hasPrefix("welcome-")
      && !entry.id.hasPrefix("default-thread-ready:")
  }

  func workspaceDisplayName() -> String {
    workspace?.displayName ?? "No Workspace"
  }

  func workspacePath() -> String {
    workspace?.rootPath ?? "Open a local workspace to enable project-scoped tools."
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
    guard let model = localModels.first(where: { $0.id == modelID }),
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
          pausedModelDownloadID == nil
    else {
      return false
    }
    guard let model = localModels.first(where: { $0.id == modelID }) else {
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
    modelDownloadCoordinator.isDownloading || pausedModelDownloadID != nil
  }

  func canPauseModelDownload() -> Bool {
    modelDownloadCoordinator.canPause
  }

  func pauseModelDownload() {
    guard canPauseModelDownload() else {
      return
    }

    runtimeDetail = LocalModelDownloadControlPlanner.pauseDetail(
      activeModelID: modelDownloadID,
      models: localModels
    )
    modelDownloadCoordinator.pauseActiveTransfer()
  }

  func cancelModelDownload() {
    guard canCancelModelDownload(),
          let cancelPlan = LocalModelDownloadControlPlanner.cancelPlan(
            isDownloading: modelDownloadCoordinator.isDownloading,
            activeModelID: modelDownloadID,
            pausedModelID: pausedModelDownloadID,
            models: localModels
          )
    else {
      return
    }

    switch cancelPlan.mode {
    case .running:
      runtimeDetail = cancelPlan.runtimeDetail
      modelDownloadCoordinator.cancelActiveDownload()
    case .orphanedPaused(let modelID):
      clearPausedModelDownload()
      removeIncompleteModelFile(modelID: modelID)
      modelDownloadProgress = nil
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
    guard let model = localModels.first(where: { $0.id == modelID }) else {
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
      pausedModelID: pausedModelDownloadID,
      resumeData: modelDownloadCoordinator.resumeData,
      currentProgress: modelDownloadProgress
    )
    let sessionState = LocalModelDownloadSessionPlanner.startState(
      model: model,
      startPlan: startPlan,
      activateAfterDownload: activateAfterDownload,
      isLocalModelReady: isLocalModelReady()
    )
    modelDownloadID = sessionState.activeModelID
    pausedModelDownloadID = sessionState.pausedModelID
    if sessionState.clearsPausedState {
      LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
    }
    modelDownloadProgress = sessionState.progress
    let shouldActivateAfterDownload = sessionState.shouldActivateAfterDownload
    appendModelEvent(
      title: startPlan.timelineTitle,
      body: startPlan.timelineBody,
      model: model,
      attributes: startPlan.attributes
    )
    let downloadTask = Task {
      defer {
        modelDownloadID = nil
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
        let finalizationPlan: LocalModelDownloadFinalizationPlan
        do {
          finalizationPlan = try LocalModelDownloadFinalizer.prepare(
            model: model,
            activationRequested: shouldActivateAfterDownload,
            hasActiveOrPendingTurn: hasActiveOrPendingTurn()
          )
        } catch LocalModelActivationPreparationError.integrityCheckFailed(let error) {
          removeIncompleteModelFile(modelID: model.id)
          throw LocalModelActivationPreparationError.integrityCheckFailed(error)
        }

        if let preparedActivation = finalizationPlan.preparedActivation {
          runtimeBridge.configureActiveLocalModel(
            manifestPath: preparedActivation.manifestPath,
            modelPath: model.installPath
          )
        }

        let completionPlan = LocalModelDownloadCompletionPlanner.plan(
          model: model,
          sourceURL: downloadURL,
          activationRequested: shouldActivateAfterDownload,
          canActivateNow: finalizationPlan.canActivateNow,
          manifestPath: finalizationPlan.manifestPath
        )

        applyModelDownloadCompletionPlan(completionPlan, model: model)
      } catch {
        let interruptionPlan = LocalModelDownloadInterruptionPlanner.plan(model: model, error: error)
        applyModelDownloadInterruptionPlan(interruptionPlan, model: model)
      }
    }
    modelDownloadCoordinator.start(downloadTask)
  }

  func activateRecommendedModel(modelID: String) {
    guard !hasActiveOrPendingTurn() else {
      runtimeDetail = "Finish or cancel the current local turn before switching models."
      return
    }

    guard let model = localModels.first(where: { $0.id == modelID }) else {
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
    } catch LocalModelActivationPreparationError.integrityCheckFailed(let error) {
      removeIncompleteModelFile(modelID: model.id)
      refreshLocalModelCatalog()
      runtimeDetail = "Model integrity check failed: \(error.localizedDescription)"
    } catch LocalModelActivationPreparationError.manifestWriteFailed(let error) {
      runtimeDetail = LocalModelActivationPlanner.selectionFailureDetail(error: error)
    } catch {
      runtimeDetail = LocalModelActivationPlanner.selectionFailureDetail(error: error)
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
    guard let model = localModels.first(where: { $0.id == modelID }) else {
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
    guard let modelID = selectedSetupModel()?.id else {
      return false
    }

    return canDownloadRecommendedModel(modelID: modelID)
      || canActivateRecommendedModel(modelID: modelID)
  }

  func downloadLocalModel() {
    guard let modelID = selectedSetupModel()?.id else {
      runtimeDetail = "Choose a local model before downloading."
      return
    }

    if let model = localModels.first(where: { $0.id == modelID }),
       model.active
    {
      runtimeDetail = "\(model.displayName) is already the active local model."
      return
    }

    if localModels.first(where: { $0.id == modelID })?.downloaded == true {
      activateRecommendedModel(modelID: modelID)
      return
    }

    guard canDownloadRecommendedModel(modelID: modelID) else {
      if let model = localModels.first(where: { $0.id == modelID }) {
        runtimeDetail = localModelDownloadRequestPlan(for: model).blockedDetail
          ?? "The selected local model is not ready to download."
      } else {
        runtimeDetail = "The selected local model is not ready to download."
      }
      return
    }

    downloadRecommendedModel(modelID: modelID, activateAfterDownload: true)
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
    threadPendingApprovalIDs[threadID] = Set(approvals.map(\.id))
  }

  private func refreshThreadPreview(threadID: String, preview: String) {
    guard let index = threads.firstIndex(where: { $0.id == threadID }) else {
      return
    }

    threads[index].preview = preview
  }

  private func refreshWorkspaceThreadSelection(
    from runtimeThreads: [RuntimeBridge.RuntimeThreadSummary],
    createIfEmpty: Bool
  ) async throws {
    guard let workspace else {
      resetToWelcomeThread()
      return
    }

    var workspaceThreads = runtimeThreads
      .filter { $0.workspaceRootPath == workspace.rootPath }
      .map { RuntimeSummaryMapper.threadSummary(from: $0) }

    if workspaceThreads.isEmpty && createIfEmpty {
      let thread = try await runtimeBridge.startThread(title: "\(workspace.displayName) Thread")
      workspaceThreads = [thread]
    }

    if workspaceThreads.isEmpty {
      resetToWelcomeThread()
      return
    }

    threads = workspaceThreads
    threadTimelines = Dictionary(
      uniqueKeysWithValues: workspaceThreads.map { thread in
        (
          thread.id,
          threadTimelines[thread.id]
            ?? TimelineEntryFactory.defaultTimeline(for: thread.title)
        )
      }
    )
    threadPendingApprovalIDs = threadPendingApprovalIDs.filter { entry in
      workspaceThreads.contains(where: { $0.id == entry.key })
    }

    let selectedThread = workspaceThreads.first(where: { $0.id == selectedThreadID }) ?? workspaceThreads.first
    selectedThreadID = selectedThread?.id
    syncVisibleTimeline()

    if let selectedThreadID {
      await loadThreadHistory(threadID: selectedThreadID)
      announceFirstRequestReadyIfNeeded()
    }
  }

  private func resetToWelcomeThread() {
    let welcomeThread = ThreadSummary(
      id: "local-welcome",
      title: "Welcome to Pith",
      preview: "Open a workspace to begin the local agent loop.",
      workspaceRootPath: nil,
      workspaceDisplayName: nil
    )
    let welcomeTimeline = TimelineEntryFactory.welcomeTimeline()

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

    var entries = threadTimelines[threadID]
      ?? TimelineEntryFactory.defaultTimeline(for: threadTitle(for: threadID))
    entries.insert(entry, at: 0)
    threadTimelines[threadID] = entries

    if selectedThreadID == threadID {
      let previousSelectionID = selectedEntryID
      timeline = entries
      selectedEntryID = TimelineEntryFactory.bestSelectionID(
        previousSelectionID: previousSelectionID,
        entries: entries
      )
    }
  }

  private func syncVisibleTimeline() {
    guard let selectedThreadID else {
      timeline = []
      selectedEntryID = nil
      return
    }

    let previousSelectionID = selectedEntryID
    timeline =
      threadTimelines[selectedThreadID]
      ?? TimelineEntryFactory.defaultTimeline(for: threadTitle(for: selectedThreadID))
    threadTimelines[selectedThreadID] = timeline
    selectedEntryID = TimelineEntryFactory.bestSelectionID(
      previousSelectionID: previousSelectionID,
      entries: timeline
    )
  }

  private func threadTitle(for threadID: String) -> String {
    threads.first(where: { $0.id == threadID })?.title ?? "Thread"
  }

  private func loadThreadHistory(threadID: String) async {
    do {
      let result = try await runtimeBridge.readThread(threadID: threadID)
      let previousSelectionID = selectedThreadID == threadID ? selectedEntryID : nil
      let entries = TimelineEntryFactory.runtimeEntries(
        from: result.items,
        existingEntries: threadTimelines[threadID],
        fallbackTitle: threadTitle(for: threadID)
      )
      threadTimelines[threadID] = entries
      updatePendingApprovals(threadID: threadID, approvals: result.pendingApprovals)
      updateActiveTurn(threadID: threadID, activeTurnID: result.activeTurnID)
      refreshThreadPreview(threadID: threadID, preview: result.status)

      if selectedThreadID == threadID {
        timeline = entries
        selectedEntryID = TimelineEntryFactory.bestSelectionID(
          previousSelectionID: previousSelectionID,
          entries: entries
        )
      }
    } catch {
      appendEntry(
        to: threadID,
        TimelineEntry(
          id: UUID().uuidString,
          kind: .warning,
          title: "Thread Load Failed",
          body: error.localizedDescription,
          attributes: [:]
        )
      )
    }
  }

  private func selectedEntry() -> TimelineEntry? {
    guard let selectedEntryID else {
      return nil
    }

    return timeline.first(where: { $0.id == selectedEntryID })
  }

  private func updateActiveTurn(threadID: String, activeTurnID: String?) {
    if activeTurnID == nil {
      if activeTurnThreadID == threadID {
        activeTurnThreadID = nil
      }
      self.activeTurnID = nil
      return
    }

    if self.activeTurnID == activeTurnID, activeTurnThreadID == threadID {
      return
    }

    self.activeTurnID = activeTurnID
    activeTurnThreadID = threadID
  }

  private func cancelPendingTurnRequest() -> Bool {
    guard let threadID = pendingTurnRequest.cancel() else {
      return false
    }

    runtimeDetail = "Cancelling local turn request..."
    refreshThreadPreview(threadID: threadID, preview: "Cancelling response")
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
      hasModelDownload: modelDownloadID != nil,
      hasPausedModelDownload: pausedModelDownloadID != nil
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
      modelDownloadID: modelDownloadID,
      pausedModelDownloadID: pausedModelDownloadID,
      modelDownloadProgress: modelDownloadProgress,
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
    guard let selectedThreadID,
          !selectedThreadID.hasPrefix("local-"),
          let selectedThread = threads.first(where: { $0.id == selectedThreadID }),
          let workspace
    else {
      return false
    }

    return selectedThread.workspaceRootPath == workspace.rootPath
  }

  private var trimmedDraftMessage: String {
    draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  private func selectedThreadIsWaitingForFirstMessage() -> Bool {
    guard let selectedThreadID,
          !selectedThreadID.hasPrefix("local-")
    else {
      return false
    }

    let entries = threadTimelines[selectedThreadID] ?? timeline
    return !entries.contains(where: isUserStartedTimelineEntry)
  }

  private func isUserStartedTimelineEntry(_ entry: TimelineEntry) -> Bool {
    switch entry.kind {
    case .userMessage, .assistantMessage, .plan, .tool, .diff, .approval:
      return true
    case .system, .warning:
      return false
    }
  }

  private func shouldAnnotateLaunchWithSetupEvents() -> Bool {
    SetupFlowState.shouldAnnotateLaunch(setupFlowSnapshot())
  }

  private func selectedSetupModel() -> LocalModelSummary? {
    localModels.first(where: { $0.id == selectedSetupModelID })
      ?? localModels.first(where: { $0.id == LocalModelCatalog.defaultFirstUseModelID })
      ?? localModels.first
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
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "First Request Ready",
        body: "Runtime, local model, workspace, and thread are ready. Send one short local request to finish first-use setup.",
        attributes: [
          "setup": "first-request"
        ]
      )
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
      downloadingModel: modelDownloadID
        .flatMap { id in localModels.first(where: { $0.id == id }) },
      pausedModel: pausedModelDownloadID
        .flatMap { id in localModels.first(where: { $0.id == id }) },
      selectedSetupModel: selectedSetupModel(),
      selectedDownloadBlockedDetail: selectedSetupModelDownloadBlockedDetail(),
      downloadedModelCount: downloadedModels.count,
      totalModelCount: localModels.count,
      activeModelDisplayName: localModels.first(where: { $0.active })?.displayName,
      downloadedLocalSizeBytes: downloadedLocalSize
    )
  }

  private func localModelActionSnapshot() -> LocalModelActionSnapshot {
    let canDownloadPausedModel = pausedModelDownloadID
      .map { canDownloadRecommendedModel(modelID: $0) }
      ?? false

    return LocalModelActionSnapshot(
      runtimeState: runtimeState,
      isLocalModelReady: isLocalModelReady(),
      hasModelDownload: modelDownloadID != nil,
      pausedModelDownloadID: pausedModelDownloadID,
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

  private func localModelDownloadRequestPlan(
    for model: LocalModelSummary
  ) -> LocalModelDownloadRequestPlan {
    localModelDownloadRequestPlanCache.plan(
      for: model,
      isDownloadRunning: modelDownloadCoordinator.isDownloading,
      pausedModelID: pausedModelDownloadID,
      resumeData: modelDownloadCoordinator.resumeData,
      currentProgress: modelDownloadProgress
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
      TimelineEntry(
        id: UUID().uuidString,
        kind: kind,
        title: title,
        body: body,
        attributes: eventAttributes
      )
    )
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
    modelDownloadProgress = nil
    refreshLocalModelCatalog()
    appendEntry(
      to: selectedThreadID,
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: "Local Model Downloaded",
        body: plan.timelineBody,
        attributes: plan.attributes
      )
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
      pausedModelDownloadID = model.id
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
      modelDownloadProgress = nil
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
      TimelineEntry(
        id: UUID().uuidString,
        kind: .system,
        title: plan.timelineTitle,
        body: plan.timelineBody,
        attributes: plan.attributes
      )
    )
    relaunchRuntimeIfNeeded(
      runningDetail: plan.relaunchRunningDetail,
      idleDetail: plan.relaunchIdleDetail
    )
  }

  private func relaunchRuntimeIfNeeded(runningDetail: String, idleDetail: String) {
    switch runtimeState {
    case .ready:
      runtimeDetail = runningDetail
      runtimeBridge.stopRuntime(detail: runningDetail)
      launchRuntime(launchDetail: runningDetail)
    case .launching:
      runtimeDetail = runningDetail
      runtimeBridge.stopRuntime(detail: runningDetail)
      Task {
        for _ in 0..<10 {
          if runtimeState != .launching {
            break
          }
          try? await Task.sleep(nanoseconds: 200_000_000)
        }
        if runtimeState == .launching {
          runtimeDetail = "Runtime is still launching. Relaunch it after model setup finishes."
          return
        }
        launchRuntime(launchDetail: runningDetail)
      }
    case .disconnected, .failed:
      runtimeDetail = idleDetail
    }
  }

  private func handleRuntimeConnectionStateChange(_ state: RuntimeBridge.ConnectionState, detail: String) {
    let previousState = runtimeState
    runtimeState = state
    runtimeDetail = detail

    switch state {
    case .ready:
      lastRuntimeFailureDetail = nil
    case .failed:
      activeTurnID = nil
      activeTurnThreadID = nil
      pendingTurnRequest.clear()
      modelHealth = nil
      runtimeReadiness = nil
      if previousState != .failed || lastRuntimeFailureDetail != detail {
        appendEntry(
          to: selectedThreadID,
          TimelineEntry(
            id: UUID().uuidString,
            kind: .warning,
            title: "Runtime Disconnected",
            body: "\(detail) Use Relaunch Runtime to recover the local session.",
            attributes: [
              "recovery": "relaunch-runtime"
            ]
          )
        )
      }
      lastRuntimeFailureDetail = detail
    case .disconnected:
      activeTurnID = nil
      activeTurnThreadID = nil
      pendingTurnRequest.clear()
      modelHealth = nil
      runtimeReadiness = nil
    case .launching:
      break
    }
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
        activeModelID: modelDownloadID,
        currentProgress: modelDownloadProgress,
        bytesReceived: bytesReceived,
        totalBytes: totalBytes,
        updatedAt: Date()
      )
    ) else {
      return
    }

    modelDownloadProgress = progress
    runtimeDetail = modelDownloadProgressSummary()
  }

  private func clearPausedModelDownload() {
    pausedModelDownloadID = nil
    LocalModelDownloadStateStore.clearPausedDownload(coordinator: modelDownloadCoordinator)
  }

  private func persistPausedModelDownload(modelID: String, resumeData: Data) {
    LocalModelDownloadStateStore.persistPausedDownload(
      modelID: modelID,
      resumeData: resumeData,
      progress: modelDownloadProgress
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

    if let status = memoryRefresh.status {
      memoryStatus = status
    }
    if let notes = memoryRefresh.notes {
      memoryNotes = notes
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
    let previousSelectionID = selectedThreadID == state.id ? selectedEntryID : nil
    let entries = TimelineEntryFactory.runtimeEntries(
      from: state.items,
      existingEntries: threadTimelines[state.id],
      fallbackTitle: threadTitle(for: state.id)
    )

    threadTimelines[state.id] = entries
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)

    if selectedThreadID == state.id {
      timeline = entries
      selectedEntryID = TimelineEntryFactory.bestSelectionID(
        previousSelectionID: previousSelectionID,
        entries: entries
      )
    }
  }

  private func confirmPluginInstall(preview: PluginInstallPreview) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Install Plugin?"
    alert.informativeText = """
      Plugin: \(preview.displayName) \(preview.version)
      Provenance: Local import
      Author: \(preview.authorName ?? "Unknown")
      Source: \(preview.sourcePath)
      Manifest: \(preview.manifestPath)
      Install Path: \(preview.installPath)
      Default Enabled: \(preview.defaultEnabled ? "Yes" : "No")
      Permissions: \(summaryLine(preview.permissions, empty: "none"))
      Capabilities: \(summaryLine(preview.capabilities, empty: "none"))

      \(preview.description)
      """
    alert.addButton(withTitle: "Install")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private func confirmPluginRemoval(plugin: PluginSummary) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .warning
    alert.messageText = "Remove Local Plugin?"
    alert.informativeText = """
      Plugin: \(plugin.displayName) \(plugin.version)
      Provenance: \(plugin.provenance)
      Manifest: \(plugin.manifestPath)
      Permissions: \(summaryLine(plugin.permissions, empty: "none"))
      Capabilities: \(summaryLine(plugin.capabilities, empty: "none"))

      Removing this plugin updates the local catalog and can disable related commands, hooks, and permissions.
      """
    alert.addButton(withTitle: "Remove")
    alert.addButton(withTitle: "Cancel")
    return alert.runModal() == .alertFirstButtonReturn
  }

  private func summaryLine(_ values: [String], empty: String) -> String {
    if values.isEmpty {
      return empty
    }

    return values.joined(separator: ", ")
  }

  private func formattedDownloadSize(_ value: String?) -> String {
    guard let value, let byteCount = Int64(value) else {
      return "size unavailable"
    }

    return formattedByteCount(byteCount)
  }

  private func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }

  private func pluginInstallRepairHint(for error: Error) -> String {
    let message = error.localizedDescription

    if message.contains("does not contain pith-plugin.json") {
      return "Choose a plugin folder that contains pith-plugin.json, or select the manifest file directly."
    }

    if message.contains("Select a plugin folder or a pith-plugin.json manifest") {
      return "Point the installer at a plugin directory or the manifest file itself."
    }

    if message.contains("Plugin manifest name") {
      return "Use a stable plugin name without path separators or colons, for example notion-connector."
    }

    if message.contains("correct format")
      || message.contains("is missing")
    {
      return "Check that pith-plugin.json is valid JSON and uses camelCase keys such as displayName and defaultEnabled."
    }

    return ""
  }
}
