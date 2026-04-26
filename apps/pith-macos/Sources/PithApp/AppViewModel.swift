import AppKit
import Foundation

@MainActor
final class AppViewModel: ObservableObject {
  private static let lastWorkspacePathKey = "pith.lastWorkspacePath"
  private static let selectedSetupModelIDKey = "pith.selectedSetupModelID"
  private let setupStepCount = 4

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
  @Published var localModels: [LocalModelSummary]
  @Published var selectedSetupModelID: String {
    didSet {
      Self.storeSelectedSetupModelID(selectedSetupModelID)
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
  private var lastRuntimeFailureDetail: String?
  private var workspaceSearchRequestID: UUID?
  private var modelDownloadTask: Task<Void, Never>?
  private var modelDownloadTransfer: ModelDownloadTransfer?
  private var modelDownloadResumeData: Data?
  private var announcedSetupCompleteThreadIDs: Set<String>

  init(runtimeBridge: RuntimeBridge = RuntimeBridge()) {
    let initialTimeline = Self.welcomeTimeline()

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
      ?? Self.storedSelectedSetupModelID(matching: initialLocalModels)
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
    self.workspaceSearchRequestID = nil
    self.modelDownloadTask = nil
    self.modelDownloadTransfer = nil
    self.modelDownloadResumeData = pausedDownload?.resumeData
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
        if currentWorkspace == nil, let lastWorkspacePath = storedLastWorkspacePath() {
          if isRestorableWorkspacePath(lastWorkspacePath) {
            do {
              currentWorkspace = try await runtimeBridge.openWorkspace(path: lastWorkspacePath)
              restoredWorkspace = true
            } catch {
              workspaceRestoreError = error
            }
          } else {
            skippedWorkspaceRestorePath = lastWorkspacePath
            clearLastWorkspacePath()
          }
        }
        let threadList = try await runtimeBridge.listThreads()

        runtimeState = .ready
        await refreshModelHealthState(serverLabel: "\(session.serverName) \(session.serverVersion)")

        if let runtimeMemoryStatus {
          memoryStatus = MemoryStatusSummary(
            noteCount: runtimeMemoryStatus.noteCount,
            latestTitle: runtimeMemoryStatus.latestTitle,
            summary: runtimeMemoryStatus.summary
          )
        } else {
          memoryStatus = nil
        }
        memoryNotes = (runtimeMemoryNotes ?? []).map { note in
          MemoryNoteSummary(
            id: note.id,
            title: note.title,
            body: note.body,
            scope: note.scope,
            source: note.source,
            createdAt: note.createdAt,
            tags: note.tags
          )
        }

        await refreshPluginState()

        if let currentWorkspace {
          workspace = WorkspaceSummary(
            rootPath: currentWorkspace.rootPath,
            displayName: currentWorkspace.displayName
          )
          resetWorkspaceSearch()
          storeLastWorkspacePath(currentWorkspace.rootPath)
        }

        if workspace != nil {
          try await refreshWorkspaceThreadSelection(from: threadList, createIfEmpty: isLocalModelReady())
        } else {
          resetToWelcomeThread()
        }
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
        if let runtimeMemoryStatus {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Memory Ready",
              body: runtimeMemoryStatus.summary,
              attributes: [
                "noteCount": String(runtimeMemoryStatus.noteCount)
              ]
            )
          )
        }
        if !plugins.isEmpty {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Plugins Ready",
              body: "Discovered \(plugins.count) plugin(s): \(plugins.map(\.displayName).joined(separator: ", ")).",
              attributes: [:]
            )
          )
        }
        if let registrySummary = pluginCapabilityRegistrySummary,
           registrySummary.totalCapabilityCount > 0 {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Capability Registry Ready",
              body:
                "Registered \(registrySummary.totalCapabilityCount) capability(ies) across \(registrySummary.enabledPluginCount) enabled plugin(s).",
              attributes: [
                "enabledPluginCount": String(registrySummary.enabledPluginCount),
                "totalCapabilityCount": String(registrySummary.totalCapabilityCount)
              ]
            )
          )
        }
        if !pluginCommands.isEmpty {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Plugin Commands Ready",
              body:
                "Loaded \(pluginCommands.count) plugin command(s): \(pluginCommands.map(\.title).joined(separator: ", ")).",
              attributes: [:]
            )
          )
        }
        if !pluginHooks.isEmpty {
          appendEntry(
            to: selectedThreadID,
            TimelineEntry(
              id: UUID().uuidString,
              kind: .system,
              title: "Plugin Hooks Ready",
              body:
                "Loaded \(pluginHooks.count) plugin hook(s): \(pluginHooks.map(\.title).joined(separator: ", ")).",
              attributes: [:]
            )
          )
        }
        announceSetupCompleteIfNeeded()
      } catch {
        runtimeState = .failed
        runtimeDetail = error.localizedDescription
        modelHealth = nil
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

  func inspectorSessionTitle() -> String {
    InspectorSessionPresenter.title(inspectorSessionSnapshot())
  }

  func inspectorSessionDetail() -> String {
    InspectorSessionPresenter.detail(inspectorSessionSnapshot())
  }

  func inspectorSessionMetaSummary() -> String {
    InspectorSessionPresenter.metaSummary(inspectorSessionSnapshot())
  }

  func shouldShowModelSetupCallout() -> Bool {
    runtimeState == .ready && !isLocalModelReady()
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

    return localModelSetupGuidance().detail
  }

  func modelSetupCalloutTone() -> StatusTone {
    localModelSetupGuidance().tone
  }

  func modelSetupCalloutActionTitle() -> String? {
    if modelDownloadID != nil {
      return "Pause Download"
    }
    if pausedModelDownloadID != nil {
      return "Continue Download"
    }
    if canDownloadLocalModel() {
      return defaultModelDownloadButtonTitle()
    }
    if canBootstrapModelPackMetadata() {
      return "Install Metadata"
    }

    return nil
  }

  func canRunModelSetupCalloutAction() -> Bool {
    if modelDownloadID != nil {
      return canPauseModelDownload()
    }
    if let pausedModelDownloadID {
      return canDownloadRecommendedModel(modelID: pausedModelDownloadID)
    }

    return canDownloadLocalModel() || canBootstrapModelPackMetadata()
  }

  func runModelSetupCalloutAction() {
    if modelDownloadID != nil {
      pauseModelDownload()
      return
    }
    if let pausedModelDownloadID,
       canDownloadRecommendedModel(modelID: pausedModelDownloadID)
    {
      downloadRecommendedModel(modelID: pausedModelDownloadID, activateAfterDownload: !isLocalModelReady())
      return
    }
    if canDownloadLocalModel() {
      downloadLocalModel()
      return
    }
    if canBootstrapModelPackMetadata() {
      bootstrapModelPackMetadata()
    }
  }

  func modelSetupCalloutSecondaryActionTitle() -> String? {
    guard modelDownloadID != nil || pausedModelDownloadID != nil else {
      return nil
    }

    return "Cancel Download"
  }

  func canRunModelSetupCalloutSecondaryAction() -> Bool {
    canCancelModelDownload()
  }

  func runModelSetupCalloutSecondaryAction() {
    cancelModelDownload()
  }

  func runtimePrimaryActionTitle() -> String? {
    switch runtimeState {
    case .disconnected, .failed, .launching:
      return runtimeLaunchButtonTitle()
    case .ready:
      if activeTurnID != nil {
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
      if activeTurnID != nil {
        return canCancelActiveTurn()
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
      if activeTurnID != nil {
        cancelActiveTurn()
      }
    }
  }

  func canLaunchRuntime() -> Bool {
    runtimeState != .launching
  }

  func canOpenWorkspace() -> Bool {
    runtimeState == .ready
  }

  func canCreateThread() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && isLocalModelReady()
      && activeTurnID == nil
  }

  func canInstallPlugin() -> Bool {
    runtimeState == .ready
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
    runtimeState == .ready && isTurnStreaming()
  }

  func canUseComposer() -> Bool {
    runtimeState == .ready
      && workspace != nil
      && isLocalModelReady()
      && hasRuntimeThreadSelection()
      && activeTurnID == nil
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
    let query = workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
    guard runtimeState == .ready, workspace != nil, !query.isEmpty else {
      return
    }

    isWorkspaceSearching = true
    let requestID = UUID()
    workspaceSearchRequestID = requestID
    workspaceSearchStatus = "Searching for \"\(query)\"..."
    Task {
      do {
        let matches = try await runtimeBridge.searchWorkspace(query: query)
        guard workspaceSearchRequestID == requestID else {
          return
        }
        guard workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines) == query else {
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
        workspaceSearchStatus = matches.isEmpty
          ? "No matches found for \"\(query)\"."
          : "Found \(matches.count) match(es) for \"\(query)\"."
      } catch {
        guard workspaceSearchRequestID == requestID else {
          return
        }
        guard workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines) == query else {
          finishChangedWorkspaceSearch()
          return
        }
        workspaceSearchResults = []
        workspaceSearchStatus = "Workspace search failed: \(error.localizedDescription)"
      }
      workspaceSearchRequestID = nil
      isWorkspaceSearching = false
    }
  }

  func clearWorkspaceSearch() {
    workspaceSearchQuery = ""
    resetWorkspaceSearch()
  }

  func workspaceSearchEmptyStateSummary() -> String? {
    if isWorkspaceSearching || !workspaceSearchResults.isEmpty {
      return nil
    }
    if runtimeState != .ready {
      return "Launch the runtime to search workspace files."
    }
    if workspace == nil {
      return "Open a workspace to search local files."
    }
    if workspaceSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
      return "Search file contents or symbols, then press Return."
    }
    if workspaceSearchStatus.hasPrefix("No matches found") {
      return "No results yet. Try a shorter query, filename, or symbol name."
    }
    if workspaceSearchStatus.hasPrefix("Workspace search failed") {
      return "Search failed. Check the runtime status, then try again."
    }
    return nil
  }

  func workspaceSearchOverflowSummary() -> String? {
    guard workspaceSearchResults.count > 8 else {
      return nil
    }

    return "Showing the first 8 matches. Narrow the query to focus the review."
  }

  func openWorkspace() {
    guard runtimeState == .ready else {
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
        storeLastWorkspacePath(openedWorkspace.rootPath)
        await refreshMemoryState()
        let threadList = try await runtimeBridge.listThreads()
        try await refreshWorkspaceThreadSelection(from: threadList, createIfEmpty: isLocalModelReady())
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
        announceSetupCompleteIfNeeded()
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
    guard runtimeState == .ready else {
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
        threadTimelines[thread.id] = defaultTimeline(for: thread.title)
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
        announceSetupCompleteIfNeeded()
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
          activeTurnID == nil
    else {
      return
    }

    draftMessage = ""

    Task {
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
        if draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
          draftMessage = message
        }
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
  }

  func respondToApproval(approvalID: String, decision: String) {
    guard runtimeState == .ready else {
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
    guard runtimeState == .ready else {
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
    guard runtimeState == .ready,
          let plugin = plugins.first(where: { $0.id == pluginID }),
          plugin.provenance == "local"
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
    guard runtimeState == .ready,
          isLocalModelReady(),
          let threadID = selectedThreadID,
          activeTurnID == nil
    else {
      return
    }

    Task {
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
      announceSetupCompleteIfNeeded()
    }
  }

  func saveWorkspaceMemoryNote() {
    let title = memoryNoteTitle.trimmingCharacters(in: .whitespacesAndNewlines)
    let body = memoryNoteBody.trimmingCharacters(in: .whitespacesAndNewlines)

    guard runtimeState == .ready,
          workspace != nil,
          !title.isEmpty,
          !body.isEmpty
    else {
      return
    }

    Task {
      do {
        let note = try await runtimeBridge.createMemoryNote(title: title, body: body)
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
    guard runtimeState == .ready,
          let activeTurnID,
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
    guard runtimeState == .ready else {
      return nil
    }
    if modelDownloadID != nil {
      return "Pause Download"
    }
    if pausedModelDownloadID != nil {
      return "Continue Download"
    }
    if !isLocalModelReady() {
      if canDownloadLocalModel() {
        return defaultModelDownloadButtonTitle()
      }
      if canBootstrapModelPackMetadata() {
        return "Install Metadata"
      }
    }

    return nil
  }

  func canRunLocalModelPrimaryAction() -> Bool {
    guard runtimeState == .ready else {
      return false
    }
    if modelDownloadID != nil {
      return canPauseModelDownload()
    }
    if let pausedModelDownloadID {
      return canDownloadRecommendedModel(modelID: pausedModelDownloadID)
    }
    if !isLocalModelReady() {
      return canDownloadLocalModel() || canBootstrapModelPackMetadata()
    }

    return false
  }

  func runLocalModelPrimaryAction() {
    if modelDownloadID != nil {
      pauseModelDownload()
      return
    }
    if let pausedModelDownloadID,
       canDownloadRecommendedModel(modelID: pausedModelDownloadID)
    {
      downloadRecommendedModel(modelID: pausedModelDownloadID, activateAfterDownload: !isLocalModelReady())
      return
    }
    if !isLocalModelReady() {
      if canDownloadLocalModel() {
        downloadLocalModel()
        return
      }
      if canBootstrapModelPackMetadata() {
        bootstrapModelPackMetadata()
      }
    }
  }

  func localModelSecondaryActionTitle() -> String? {
    canCancelModelDownload() ? "Cancel Download" : nil
  }

  func canRunLocalModelSecondaryAction() -> Bool {
    canCancelModelDownload()
  }

  func runLocalModelSecondaryAction() {
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
          activeTurnID == nil,
          modelDownloadTask == nil,
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
      && activeTurnID == nil
      && modelDownloadTask == nil
      && runtimeBridge.activeLocalModelPath() != nil
  }

  func canCancelModelDownload() -> Bool {
    modelDownloadTask != nil || pausedModelDownloadID != nil
  }

  func canPauseModelDownload() -> Bool {
    modelDownloadTask != nil
  }

  func pauseModelDownload() {
    let displayName = modelDownloadID
      .flatMap { id in localModels.first(where: { $0.id == id })?.displayName }
      ?? "local model"
    runtimeDetail = "Pausing \(displayName) download..."
    modelDownloadTransfer?.pause()
  }

  func cancelModelDownload() {
    if let modelDownloadTask {
      let displayName = modelDownloadID
        .flatMap { id in localModels.first(where: { $0.id == id })?.displayName }
        ?? "local model"
      runtimeDetail = "Cancelling \(displayName) download..."
      modelDownloadTask.cancel()
      modelDownloadTransfer?.cancel()
      return
    }

    guard let pausedModelDownloadID else {
      return
    }

    guard let model = localModels.first(where: { $0.id == pausedModelDownloadID }) else {
      clearPausedModelDownload()
      removeIncompleteModelFile(modelID: pausedModelDownloadID)
      modelDownloadProgress = nil
      runtimeDetail = "Cancelled local model download and cleared partial state."
      refreshLocalModelCatalog()
      return
    }

    applyModelDownloadInterruptionPlan(
      LocalModelDownloadInterruptionPlanner.cancellationPlan(model: model),
      model: model
    )
    refreshLocalModelCatalog()
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
      resumeData: modelDownloadResumeData,
      currentProgress: modelDownloadProgress
    )
    if !startPlan.isResuming {
      guard confirmModelDownload(
        displayName: model.displayName,
        downloadURL: downloadURL,
        targetPath: model.installPath,
        sizeSummary: formattedByteCount(model.sizeBytes)
      ) else {
        runtimeDetail = "Local model download was cancelled."
        return
      }
    }

    modelDownloadID = model.id
    pausedModelDownloadID = nil
    modelDownloadResumeData = nil
    LocalModelCatalog.clearPausedDownload()
    modelDownloadProgress = startPlan.progress
    let shouldActivateAfterDownload = activateAfterDownload || !isLocalModelReady()
    appendModelEvent(
      title: startPlan.timelineTitle,
      body: startPlan.timelineBody,
      model: model,
      attributes: startPlan.attributes
    )
    modelDownloadTask = Task {
      defer {
        modelDownloadID = nil
        modelDownloadTask = nil
        modelDownloadTransfer = nil
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

        let canActivateDownloadedModel = activeTurnID == nil
        let manifestPath: String?
        if shouldActivateAfterDownload && canActivateDownloadedModel {
          let modelManifestPath = try LocalModelCatalog.writePackManifest(for: model)
          runtimeBridge.configureActiveLocalModel(
            manifestPath: modelManifestPath,
            modelPath: model.installPath
          )
          manifestPath = modelManifestPath
        } else {
          manifestPath = nil
        }

        let completionPlan = LocalModelDownloadCompletionPlanner.plan(
          model: model,
          sourceURL: downloadURL,
          activationRequested: shouldActivateAfterDownload,
          canActivateNow: canActivateDownloadedModel,
          manifestPath: manifestPath
        )

        applyModelDownloadCompletionPlan(completionPlan, model: model)
      } catch {
        let interruptionPlan = LocalModelDownloadInterruptionPlanner.plan(model: model, error: error)
        applyModelDownloadInterruptionPlan(interruptionPlan, model: model)
      }
    }
  }

  func activateRecommendedModel(modelID: String) {
    guard activeTurnID == nil else {
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
      let manifestPath = try LocalModelCatalog.writePackManifest(for: model)
      runtimeBridge.configureActiveLocalModel(
        manifestPath: manifestPath,
        modelPath: model.installPath
      )
      selectedSetupModelID = model.id
      refreshLocalModelCatalog()
      applyLocalModelActivationPlan(
        LocalModelActivationPlanner.selectionPlan(model: model, manifestPath: manifestPath)
      )
    } catch {
      runtimeDetail = LocalModelActivationPlanner.selectionFailureDetail(error: error)
    }
  }

  func resetActiveLocalModel() {
    guard activeTurnID == nil else {
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

    revealFilePath(model.installPath, successDetail: "Revealed \(model.displayName).")
  }

  func revealSuggestedModelDirectory() {
    revealSuggestedPath(
      metricKey: "suggestedModelPath",
      successDetail: "Opened the suggested local model folder."
    )
  }

  func revealSuggestedBinaryDirectory() {
    revealSuggestedPath(
      metricKey: "suggestedBinaryPath",
      successDetail: "Opened the suggested llama.cpp binary folder."
    )
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
      runtimeDetail = "The selected local model is not ready to download."
      return
    }

    downloadRecommendedModel(modelID: modelID, activateAfterDownload: true)
  }

  func canBootstrapModelPackMetadata() -> Bool {
    runtimeState == .ready && modelDownloadTask == nil
  }

  func bootstrapModelPackMetadata() {
    guard runtimeState == .ready else {
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
    plugin.provenance == "local"
  }

  func revealPluginManifest(pluginID: String) {
    guard let plugin = plugins.first(where: { $0.id == pluginID }) else {
      runtimeDetail = "Plugin manifest path is unavailable."
      return
    }

    revealFilePath(plugin.manifestPath, successDetail: "Revealed \(plugin.displayName) manifest.")
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
    runtimeState == .ready
      && workspace != nil
      && !memoryNoteTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      && !memoryNoteBody.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
  }

  func isLocalModelReady() -> Bool {
    guard runtimeState == .ready,
          let modelHealth,
          modelHealth.status == "ready"
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
    runtimeState == .launching || activeTurnID != nil
  }

  func isTurnStreaming() -> Bool {
    activeTurnID != nil
  }

  func isPendingApproval(_ entry: TimelineEntry) -> Bool {
    guard entry.kind == .approval,
          let selectedThreadID,
          let approvalID = entry.attributes["approvalId"]
    else {
      return false
    }

    return threadPendingApprovalIDs[selectedThreadID, default: Set<String>()].contains(approvalID)
  }

  func approvalID(for entry: TimelineEntry) -> String? {
    entry.attributes["approvalId"]
  }

  private func appendItemsToTimeline(
    threadID: String,
    items: [RuntimeBridge.RuntimeTimelineItemResult]
  ) {
    let newEntries = items.map { item in
      TimelineEntry(
        id: UUID().uuidString,
        kind: timelineKind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }

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
      .map { threadSummary(from: $0) }

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
        (thread.id, threadTimelines[thread.id] ?? defaultTimeline(for: thread.title))
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
      announceSetupCompleteIfNeeded()
    }
  }

  private func threadSummary(from runtimeThread: RuntimeBridge.RuntimeThreadSummary) -> ThreadSummary {
    ThreadSummary(
      id: runtimeThread.id,
      title: runtimeThread.title,
      preview: runtimeThread.status,
      workspaceRootPath: runtimeThread.workspaceRootPath,
      workspaceDisplayName: runtimeThread.workspaceDisplayName
    )
  }

  private func resetToWelcomeThread() {
    let welcomeThread = ThreadSummary(
      id: "local-welcome",
      title: "Welcome to Pith",
      preview: "Open a workspace to begin the local agent loop.",
      workspaceRootPath: nil,
      workspaceDisplayName: nil
    )
    let welcomeTimeline = Self.welcomeTimeline()

    threads = [welcomeThread]
    threadTimelines = [welcomeThread.id: welcomeTimeline]
    selectedThreadID = welcomeThread.id
    timeline = welcomeTimeline
    selectedEntryID = welcomeTimeline.first?.id
  }

  private static func welcomeTimeline() -> [TimelineEntry] {
    [
      TimelineEntry(
        id: "welcome-start-local-setup",
        kind: .system,
        title: "Start Local Setup",
        body: "Launch the runtime, choose a local model, open a workspace, then create or select a thread.",
        attributes: [
          "path": "runtime -> model -> workspace -> thread"
        ]
      ),
      TimelineEntry(
        id: "welcome-local-first-agent-loop",
        kind: .assistantMessage,
        title: "Local-First Agent Loop",
        body:
          "Pith runs the core agent loop against local workspaces and does not call external model APIs for core responses.",
        attributes: [
          "model": "local"
        ]
      ),
    ]
  }

  private func appendEntry(to threadID: String?, _ entry: TimelineEntry) {
    guard let threadID else {
      timeline.insert(entry, at: 0)
      if selectedEntryID == nil {
        selectedEntryID = entry.id
      }
      return
    }

    var entries = threadTimelines[threadID] ?? defaultTimeline(for: threadTitle(for: threadID))
    entries.insert(entry, at: 0)
    threadTimelines[threadID] = entries

    if selectedThreadID == threadID {
      let previousSelectionID = selectedEntryID
      timeline = entries
      selectedEntryID = bestTimelineSelectionID(
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
      ?? defaultTimeline(for: threadTitle(for: selectedThreadID))
    threadTimelines[selectedThreadID] = timeline
    selectedEntryID = bestTimelineSelectionID(
      previousSelectionID: previousSelectionID,
      entries: timeline
    )
  }

  private func defaultTimeline(for title: String) -> [TimelineEntry] {
    [
      TimelineEntry(
        id: "default-thread-ready:\(title)",
        kind: .system,
        title: "Thread Ready",
        body: "\(title) is ready after runtime, model, workspace, and thread setup are complete.",
        attributes: [
          "setup": "runtime, model, workspace, thread"
        ]
      ),
    ]
  }

  private func threadTitle(for threadID: String) -> String {
    threads.first(where: { $0.id == threadID })?.title ?? "Thread"
  }

  private func loadThreadHistory(threadID: String) async {
    do {
      let result = try await runtimeBridge.readThread(threadID: threadID)
      let previousSelectionID = selectedThreadID == threadID ? selectedEntryID : nil
      let entries = timelineEntries(from: result.items, fallbackThreadID: threadID)
      threadTimelines[threadID] = entries
      updatePendingApprovals(threadID: threadID, approvals: result.pendingApprovals)
      updateActiveTurn(threadID: threadID, activeTurnID: result.activeTurnID)
      refreshThreadPreview(threadID: threadID, preview: result.status)

      if selectedThreadID == threadID {
        timeline = entries
        selectedEntryID = bestTimelineSelectionID(
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

  private func timelineKind(for rawKind: String) -> TimelineEntry.Kind {
    switch rawKind {
    case "userMessage":
      return .userMessage
    case "assistantMessage":
      return .assistantMessage
    case "plan":
      return .plan
    case "diffArtifact":
      return .diff
    case "toolStart", "toolResult", "pluginCommand", "pluginResult":
      return .tool
    case "approvalRequested", "approvalResolved":
      return .approval
    case "warning":
      return .warning
    default:
      return .system
    }
  }

  private func selectedEntry() -> TimelineEntry? {
    guard let selectedEntryID else {
      return nil
    }

    return timeline.first(where: { $0.id == selectedEntryID })
  }

  private func bestTimelineSelectionID(
    previousSelectionID: TimelineEntry.ID?,
    entries: [TimelineEntry]
  ) -> TimelineEntry.ID? {
    if let previousSelectionID,
       entries.contains(where: { $0.id == previousSelectionID }) {
      return previousSelectionID
    }

    return entries.first?.id
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

  private func refreshModelHealthState(serverLabel: String? = nil) async {
    let runtimeModel = try? await runtimeBridge.modelHealth()
    if let runtimeModel {
      modelHealth = ModelHealthSummary(
        packID: runtimeModel.packID,
        displayName: runtimeModel.displayName,
        backend: runtimeModel.backend,
        status: runtimeModel.status,
        detail: runtimeModel.detail,
        source: runtimeModel.source,
        binaryPath: runtimeModel.binaryPath,
        modelPath: runtimeModel.modelPath,
        manifestPath: runtimeModel.manifestPath,
        metrics: runtimeModel.metrics
      )
      if let serverLabel {
        runtimeDetail = "\(serverLabel) | \(runtimeModel.displayName)"
      }
      refreshLocalModelCatalog()
    } else {
      modelHealth = nil
      refreshLocalModelCatalog()
      if let serverLabel {
        runtimeDetail = serverLabel
      }
    }
    announceSetupCompleteIfNeeded()
  }

  private func refreshLocalModelCatalog() {
    let activeModelPath = runtimeBridge.activeLocalModelPath() ?? modelHealth?.modelPath
    localModels = LocalModelCatalog.summaries(
      storageRootPath: runtimeBridge.localModelStorageRootPath(),
      activeModelPath: activeModelPath
    )
    if !localModels.contains(where: { $0.id == selectedSetupModelID }) {
      selectedSetupModelID = LocalModelCatalog.defaultFirstUseModelID
    }
  }

  private static func storedSelectedSetupModelID(matching models: [LocalModelSummary]) -> String? {
    guard let modelID = UserDefaults.standard.string(forKey: selectedSetupModelIDKey),
          models.contains(where: { $0.id == modelID })
    else {
      return nil
    }

    return modelID
  }

  private static func storeSelectedSetupModelID(_ modelID: String) {
    UserDefaults.standard.set(modelID, forKey: selectedSetupModelIDKey)
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
      hasActiveTurn: activeTurnID != nil,
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
      stepCount: setupStepCount,
      runtimeState: runtimeState,
      showsRuntimeActivity: showsRuntimeActivity(),
      isLocalModelReady: isModelReady,
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: activeTurnID != nil,
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
      hasActiveTurn: activeTurnID != nil
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
      hasActiveTurn: activeTurnID != nil,
      setupReadyStepCount: setupReadyStepCount(),
      setupStepCount: setupStepCount,
      setupProgressDetail: setupProgressDetail(),
      isWaitingForFirstMessage: selectedThreadIsWaitingForFirstMessage()
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

  private func composerStatusSnapshot() -> ComposerStatusSnapshot {
    let modelGuidance = localModelSetupGuidance()
    return ComposerStatusSnapshot(
      runtimeState: runtimeState,
      modelSetupTitle: modelGuidance.title,
      modelSetupSummary: modelGuidance.summary,
      isLocalModelReady: isLocalModelReady(),
      hasWorkspace: workspace != nil,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      hasActiveTurn: activeTurnID != nil,
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
      selectedSetupModel: selectedSetupModel()
    )
  }

  private func setupReadyStepCount() -> Int {
    var readyCount = 0
    if runtimeState == .ready {
      readyCount += 1
    }
    if isLocalModelReady() {
      readyCount += 1
    }
    if workspace != nil {
      readyCount += 1
    }
    if isLocalModelReady() && workspace != nil && hasRuntimeThreadSelection() {
      readyCount += 1
    }
    return readyCount
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
    return !entries.contains { $0.kind == .userMessage }
  }

  private func selectedSetupModel() -> LocalModelSummary? {
    localModels.first(where: { $0.id == selectedSetupModelID })
      ?? localModels.first(where: { $0.id == LocalModelCatalog.defaultFirstUseModelID })
      ?? localModels.first
  }

  private func localModelSetupGuidance() -> LocalModelSetupGuidance {
    LocalModelOperationPresenter.setupGuidance(localModelOperationSnapshot())
  }

  private func localModelRequiredTimelineSummary() -> String {
    localModelSetupGuidance().summary
  }

  private func announceSetupCompleteIfNeeded() {
    guard setupReadyStepCount() == setupStepCount,
          let threadID = selectedThreadID,
          !threadID.hasPrefix("local-"),
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
        title: "Local Setup Complete",
        body: "Runtime, local model, workspace, and thread are ready. Ask Pith to inspect files, review diffs, or make a small change.",
        attributes: [
          "setup": "complete"
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
      hasActiveTurn: activeTurnID != nil,
      downloadingModel: modelDownloadID
        .flatMap { id in localModels.first(where: { $0.id == id }) },
      pausedModel: pausedModelDownloadID
        .flatMap { id in localModels.first(where: { $0.id == id }) },
      selectedSetupModel: selectedSetupModel(),
      downloadedModelCount: downloadedModels.count,
      totalModelCount: localModels.count,
      activeModelDisplayName: localModels.first(where: { $0.active })?.displayName,
      downloadedLocalSizeBytes: downloadedLocalSize
    )
  }

  private func localModelDownloadRequestPlan(
    for model: LocalModelSummary
  ) -> LocalModelDownloadRequestPlan {
    LocalModelDownloadRequestPlanner.plan(
      model: model,
      isDownloadRunning: modelDownloadTask != nil,
      pausedModelID: pausedModelDownloadID,
      hasResumeData: modelDownloadResumeData != nil
    )
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
      modelDownloadResumeData = resumeData
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
      modelHealth = nil
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
      modelHealth = nil
    case .launching:
      break
    }
  }

  private func revealSuggestedPath(metricKey: String, successDetail: String) {
    guard let value = modelHealth?.metrics[metricKey], !value.isEmpty else {
      runtimeDetail = "Local model guidance is unavailable until the runtime reports model health."
      return
    }

    let targetURL = URL(fileURLWithPath: value)
    let directoryURL: URL
    var isDirectory = ObjCBool(false)
    if FileManager.default.fileExists(atPath: targetURL.path, isDirectory: &isDirectory) {
      directoryURL = isDirectory.boolValue ? targetURL : targetURL.deletingLastPathComponent()
    } else {
      directoryURL = targetURL.deletingLastPathComponent()
      do {
        try FileManager.default.createDirectory(
          at: directoryURL,
          withIntermediateDirectories: true
        )
      } catch {
        runtimeDetail = "Failed to prepare \(directoryURL.path): \(error.localizedDescription)"
        return
      }
    }

    if NSWorkspace.shared.open(directoryURL) {
      runtimeDetail = successDetail
    } else {
      runtimeDetail = "Failed to open \(directoryURL.path)"
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
    modelDownloadTransfer = transfer
    try await transfer.start(from: sourceURL, resumeData: resumeData)
  }

  private func updateModelDownloadProgress(
    modelID: String,
    bytesReceived: Int64,
    totalBytes: Int64
  ) {
    guard modelDownloadID == modelID,
          var progress = modelDownloadProgress
    else {
      return
    }

    progress.bytesReceived = max(bytesReceived, progress.bytesReceived)
    progress.totalBytes = totalBytes > 0 ? totalBytes : progress.totalBytes
    progress.updatedAt = Date()
    modelDownloadProgress = progress
    runtimeDetail = modelDownloadProgressSummary()
  }

  private func clearPausedModelDownload() {
    pausedModelDownloadID = nil
    modelDownloadResumeData = nil
    LocalModelCatalog.clearPausedDownload()
  }

  private func persistPausedModelDownload(modelID: String, resumeData: Data) {
    guard !resumeData.isEmpty else {
      return
    }

    let progress = modelDownloadProgress
    LocalModelCatalog.savePausedDownload(
      modelID: modelID,
      resumeData: resumeData,
      bytesReceived: progress?.bytesReceived ?? 0,
      totalBytes: progress?.totalBytes ?? 0,
      updatedAt: progress?.updatedAt ?? Date()
    )
  }

  private func removeIncompleteModelFile(modelID: String) {
    guard let model = localModels.first(where: { $0.id == modelID }) else {
      return
    }

    let targetURL = URL(fileURLWithPath: model.installPath)
    let manager = FileManager.default
    if manager.fileExists(atPath: targetURL.path) {
      try? manager.removeItem(at: targetURL)
    }
  }

  private func revealFilePath(_ path: String, successDetail: String) {
    guard !path.isEmpty else {
      runtimeDetail = "The requested file path is unavailable."
      return
    }

    let fileURL = URL(fileURLWithPath: path)
    let manager = FileManager.default
    if manager.fileExists(atPath: fileURL.path) {
      NSWorkspace.shared.activateFileViewerSelecting([fileURL])
      runtimeDetail = successDetail
      return
    }

    let parentURL = fileURL.deletingLastPathComponent()
    if manager.fileExists(atPath: parentURL.path) {
      NSWorkspace.shared.activateFileViewerSelecting([parentURL])
      runtimeDetail = "Revealed the closest available folder for \(path)."
    } else {
      runtimeDetail = "Failed to locate \(path)"
    }
  }

  private func storedLastWorkspacePath() -> String? {
    guard let path = UserDefaults.standard.string(forKey: Self.lastWorkspacePathKey),
          !path.isEmpty
    else {
      return nil
    }

    return path
  }

  private func storeLastWorkspacePath(_ path: String) {
    guard !path.isEmpty else {
      return
    }

    UserDefaults.standard.set(path, forKey: Self.lastWorkspacePathKey)
  }

  private func clearLastWorkspacePath() {
    UserDefaults.standard.removeObject(forKey: Self.lastWorkspacePathKey)
  }

  private func resetWorkspaceSearch() {
    workspaceSearchRequestID = nil
    workspaceSearchResults = []
    workspaceSearchStatus = workspace == nil
      ? "Open a workspace before searching."
      : "Search the open workspace by text."
    isWorkspaceSearching = false
  }

  private func finishChangedWorkspaceSearch() {
    workspaceSearchResults = []
    workspaceSearchStatus = "Query changed. Press Return to search again."
    workspaceSearchRequestID = nil
    isWorkspaceSearching = false
  }

  private func isRestorableWorkspacePath(_ path: String) -> Bool {
    var isDirectory = ObjCBool(false)
    return FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory)
      && isDirectory.boolValue
  }

  private func refreshMemoryState() async {
    let runtimeMemoryStatus = try? await runtimeBridge.memoryStatus()
    let runtimeMemoryNotes = try? await runtimeBridge.listMemoryNotes()

    if let runtimeMemoryStatus {
      memoryStatus = MemoryStatusSummary(
        noteCount: runtimeMemoryStatus.noteCount,
        latestTitle: runtimeMemoryStatus.latestTitle,
        summary: runtimeMemoryStatus.summary
      )
    }
    if let runtimeMemoryNotes {
      memoryNotes = runtimeMemoryNotes.map { note in
        MemoryNoteSummary(
          id: note.id,
          title: note.title,
          body: note.body,
          scope: note.scope,
          source: note.source,
          createdAt: note.createdAt,
          tags: note.tags
        )
      }
    }
  }

  private func refreshPluginState() async {
    let runtimePlugins = try? await runtimeBridge.listPlugins()
    let runtimeRegistry = try? await runtimeBridge.pluginCapabilityRegistry()
    let runtimeCommands = try? await runtimeBridge.listPluginCommands()
    let runtimeConnectors = try? await runtimeBridge.listPluginConnectors()
    let runtimeHooks = try? await runtimeBridge.listPluginHooks()

    if let runtimePlugins {
      plugins = runtimePlugins.map { pluginSummary(from: $0) }
    }
    if let runtimeRegistry {
      pluginCapabilityRegistrySummary = PluginCapabilityRegistrySummary(
        enabledPluginCount: runtimeRegistry.summary.enabledPluginCount,
        totalCapabilityCount: runtimeRegistry.summary.totalCapabilityCount,
        capabilityCountsByKind: runtimeRegistry.summary.capabilityCountsByKind
      )
      pluginCapabilities = runtimeRegistry.capabilities.map { capability in
        PluginCapabilitySummary(
          id: capability.capabilityID,
          kind: capability.kind,
          identifier: capability.identifier,
          pluginID: capability.pluginID,
          pluginDisplayName: capability.pluginDisplayName,
          permissions: capability.permissions,
          manifestPath: capability.manifestPath,
          metadata: capability.metadata
        )
      }
    } else if runtimePlugins != nil {
      pluginCapabilityRegistrySummary = PluginCapabilityRegistrySummary(
        enabledPluginCount: plugins.filter { $0.status == "ready" && $0.enabled }.count,
        totalCapabilityCount: 0,
        capabilityCountsByKind: [:]
      )
      pluginCapabilities = []
    }
    if let runtimeCommands {
      pluginCommands = runtimeCommands.map { command in
        PluginCommandSummary(
          id: command.commandID,
          title: command.title,
          description: command.description,
          pluginID: command.pluginID,
          pluginDisplayName: command.pluginDisplayName,
          permissions: command.permissions,
          sourcePath: command.sourcePath,
          executionKind: command.executionKind,
          memorySummary: command.memorySummary
        )
      }
    } else if runtimePlugins != nil {
      pluginCommands = []
    }
    if let runtimeConnectors {
      pluginConnectors = runtimeConnectors.map { connector in
        PluginConnectorSummary(
          id: connector.connectorID,
          displayName: connector.displayName,
          service: connector.service,
          pluginID: connector.pluginID,
          pluginDisplayName: connector.pluginDisplayName,
          enabled: connector.enabled,
          status: connector.status,
          permissions: connector.permissions,
          manifestPath: connector.manifestPath,
          homepage: connector.homepage,
          authType: connector.authType,
          authRequired: connector.authRequired,
          authScopes: connector.authScopes,
          credentialStore: connector.credentialStore
        )
      }
    } else if runtimePlugins != nil {
      pluginConnectors = []
    }
    if let runtimeHooks {
      pluginHooks = runtimeHooks.map { hook in
        PluginHookSummary(
          id: hook.hookID,
          title: hook.title,
          description: hook.description,
          event: hook.event,
          pluginID: hook.pluginID,
          pluginDisplayName: hook.pluginDisplayName,
          permissions: hook.permissions,
          sourcePath: hook.sourcePath,
          memorySummary: hook.memorySummary
        )
      }
    } else if runtimePlugins != nil {
      pluginHooks = []
    }
  }

  private func applyRuntimeThreadUpdate(_ state: RuntimeBridge.RuntimeThreadState) {
    let previousSelectionID = selectedThreadID == state.id ? selectedEntryID : nil
    let entries = timelineEntries(from: state.items, fallbackThreadID: state.id)

    threadTimelines[state.id] = entries
    updatePendingApprovals(threadID: state.id, approvals: state.pendingApprovals)
    updateActiveTurn(threadID: state.id, activeTurnID: state.activeTurnID)
    refreshThreadPreview(threadID: state.id, preview: state.status)

    if selectedThreadID == state.id {
      timeline = entries
      selectedEntryID = bestTimelineSelectionID(
        previousSelectionID: previousSelectionID,
        entries: entries
      )
    }
  }

  private func timelineEntries(from items: [RuntimeBridge.RuntimeTimelineItemResult]) -> [TimelineEntry] {
    items.enumerated().map { index, item in
      TimelineEntry(
        id: runtimeTimelineID(for: item, index: index),
        kind: timelineKind(for: item.kind),
        title: item.title,
        body: item.content,
        attributes: item.attributes
      )
    }
  }

  private func timelineEntries(
    from items: [RuntimeBridge.RuntimeTimelineItemResult],
    fallbackThreadID threadID: String
  ) -> [TimelineEntry] {
    let entries = timelineEntries(from: items)
    if entries.isEmpty {
      let existingEntries = threadTimelines[threadID] ?? []
      if !existingEntries.isEmpty {
        return existingEntries
      }

      return defaultTimeline(for: threadTitle(for: threadID))
    }

    return entries
  }

  private func runtimeTimelineID(for item: RuntimeBridge.RuntimeTimelineItemResult, index: Int) -> String {
    if let approvalID = item.attributes["approvalId"] {
      return "approval:\(approvalID):\(item.kind):\(item.title)"
    }
    if let turnID = item.attributes["turnId"] {
      return "turn:\(turnID):\(item.kind):\(item.title)"
    }
    return "runtime:\(index):\(item.kind):\(item.title)"
  }

  private func pluginSummary(from plugin: RuntimeBridge.RuntimePlugin) -> PluginSummary {
    PluginSummary(
      id: plugin.id,
      name: plugin.name,
      version: plugin.version,
      displayName: plugin.displayName,
      status: plugin.status,
      description: plugin.description,
      authorName: plugin.authorName,
      enabled: plugin.enabled,
      defaultEnabled: plugin.defaultEnabled,
      capabilities: plugin.capabilities,
      permissions: plugin.permissions,
      manifestPath: plugin.manifestPath,
      provenance: plugin.provenance,
      validationError: plugin.validationError,
      validationHint: plugin.validationHint
    )
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

  private func confirmModelDownload(
    displayName: String,
    downloadURL: URL,
    targetPath: String,
    sizeSummary: String
  ) -> Bool {
    let alert = NSAlert()
    alert.alertStyle = .informational
    alert.messageText = "Download Local Model?"
    alert.informativeText = """
      Model: \(displayName)
      Size: \(sizeSummary)
      Source: \(LocalModelDisplayPresenter.sourceName(downloadURL))
      File: \(URL(fileURLWithPath: targetPath).lastPathComponent)

      Pith stores the model locally in app data and runs one local model at a time.
      """
    alert.addButton(withTitle: "Download")
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
