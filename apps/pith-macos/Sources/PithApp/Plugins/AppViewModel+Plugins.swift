import Foundation

@MainActor
extension AppViewModel {
  func installPlugin() {
    guard canInstallPlugin() else {
      return
    }

    guard let url = AppFilePicker.choosePluginSource() else {
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Inspecting local plugin..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before starting another."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      var confirmedPreview: PluginInstallPreview?
      do {
        let inspection = try await runtimeBridge.inspectPlugin(sourcePath: url.path)
        let preview = PluginInstallInspector.preview(
          for: url,
          inspection: inspection,
          installRootPath: runtimeBridge.localPluginInstallRootPath()
        )
        guard preview.canInstall else {
          runtimeDetail = preview.installBlocker ?? "Plugin cannot be installed yet."
          appendEntry(
            to: timelineThreadID,
            TimelineEventPresenter.pluginInstallBlocked(preview: preview)
          )
          return
        }
        guard PluginInstallDialogPresenter.confirmInstall(preview: preview) else {
          runtimeDetail = "Plugin install was cancelled."
          return
        }
        confirmedPreview = preview
        runtimeDetail = "Installing local plugin..."
        let installedPlugin = try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
        await refreshPluginState()
        focusPluginManagerSection(
          capabilities: installedPlugin.capabilities,
          permissions: installedPlugin.permissions
        )
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginInstalled(installedPlugin, preview: preview)
        )
      } catch {
        let repairHint = PluginInstallDialogPresenter.repairHint(for: error)
        if confirmedPreview == nil {
          appendEntry(
            to: timelineThreadID,
            TimelineEventPresenter.pluginInstallPreviewFailed(
              error: error,
              repairHint: repairHint,
              sourcePath: url.path
            )
          )
        } else {
          appendEntry(
            to: timelineThreadID,
            TimelineEventPresenter.pluginInstallFailed(
              error: error,
              repairHint: repairHint,
              sourcePath: confirmedPreview?.sourcePath
            )
          )
        }
      }
    }
  }

  func setPluginEnabled(pluginID: String, enabled: Bool) {
    guard canSetPluginEnabled(pluginID: pluginID) else {
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: enabled ? "Enabling plugin..." : "Disabling plugin..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before changing another plugin."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let updatedPlugin = try await runtimeBridge.setPluginEnabled(pluginID: pluginID, enabled: enabled)
        await refreshPluginState()
        focusPluginManagerSection(
          capabilities: updatedPlugin.capabilities,
          permissions: updatedPlugin.permissions
        )
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginUpdated(updatedPlugin, enabled: enabled)
        )
      } catch {
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginUpdateFailed(pluginID: pluginID, error: error)
        )
      }
    }
  }

  func removePlugin(pluginID: String) {
    guard canRemovePlugin(pluginID: pluginID),
          let plugin = pluginSummary(pluginID: pluginID)
    else {
      return
    }

    guard PluginInstallDialogPresenter.confirmRemoval(plugin: plugin) else {
      runtimeDetail = "Plugin removal was cancelled."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Removing local plugin..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before removing another plugin."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let removedPlugin = try await runtimeBridge.removePlugin(manifestPath: plugin.manifestPath)
        await refreshPluginState()
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginRemoved(removedPlugin)
        )
      } catch {
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginRemovalFailed(pluginID: pluginID, error: error)
        )
      }
    }
  }

  func authorizePluginConnector(connectorID: String) {
    guard canAuthorizePluginConnector(connectorID: connectorID) else {
      runtimeDetail = pluginConnectorAuthorizeDisabledReason(connectorID: connectorID)
        ?? "Connector cannot be authorized yet."
      return
    }
    guard let connector = pluginConnectors.first(where: { $0.id == connectorID }) else {
      runtimeDetail = "Connector is not loaded."
      return
    }
    guard let credentialInput = PluginConnectorCredentialDialogPresenter.credentialInput(
      connector: connector
    ) else {
      runtimeDetail = "Connector authorization was cancelled."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Authorizing connector..."
    ) else {
      runtimeDetail = pluginConnectorAuthorizeDisabledReason(connectorID: connectorID)
        ?? "Finish the current plugin operation before authorizing a connector."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let connector = try await runtimeBridge.authorizePluginConnector(
          connectorID: connectorID,
          credentialLabel: credentialInput.label,
          credentialSecret: credentialInput.secret
        )
        await refreshPluginState()
        focusAfterConnectorAuthorization(pluginID: connector.pluginID)
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorAuthorized(connector)
        )
      } catch {
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorAuthFailed(connectorID: connectorID, error: error)
        )
      }
    }
  }

  func clearPluginConnectorCredential(connectorID: String) {
    guard canClearPluginConnectorCredential(connectorID: connectorID) else {
      runtimeDetail = pluginConnectorClearDisabledReason(connectorID: connectorID)
        ?? "Connector credential cannot be cleared yet."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Clearing connector credential..."
    ) else {
      runtimeDetail = pluginConnectorClearDisabledReason(connectorID: connectorID)
        ?? "Finish the current plugin operation before clearing a connector credential."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let connector = try await runtimeBridge.clearPluginConnectorCredential(
          connectorID: connectorID
        )
        await refreshPluginState()
        pluginManagerSection = .connectors
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorCredentialCleared(connector)
        )
      } catch {
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorCredentialClearFailed(
            connectorID: connectorID,
            error: error
          )
        )
      }
    }
  }

  func runPluginCommand(commandID: String) {
    runPluginCommand(commandID: commandID, input: nil)
  }

  func runPluginCommandWithInput(commandID: String) {
    guard let command = pluginCommands.first(where: { $0.id == commandID }) else {
      runtimeDetail = "Plugin command is not loaded."
      return
    }
    let snapshot = pluginActionSnapshot()
    if PluginActionPlanner.commandNeedsExecutionContract(commandID: commandID, snapshot: snapshot) {
      appendBlockedPluginCommand(command, detail: TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail)
      return
    }
    if PluginActionPlanner.commandNeedsConnectorAuth(commandID: commandID, snapshot: snapshot) {
      let detail = PluginActionPlanner.commandRunBlocker(commandID: commandID, snapshot: snapshot)
        ?? TimelineEventPresenter.pluginCommandNeedsConnectorAuthDetail
      appendBlockedPluginCommand(command, detail: detail)
      return
    }
    guard let input = PluginCommandInputDialogPresenter.commandInput(command: command) else {
      runtimeDetail = "Plugin command input was cancelled."
      return
    }
    if command.requiresPlainInput && input.isEmpty {
      runtimeDetail = "Plugin command input is required."
      return
    }

    runPluginCommand(commandID: commandID, input: input)
  }

  func canRetryPluginCommand(from entry: TimelineEntry) -> Bool {
    guard isPluginCommandRetryableEntry(entry),
          let commandID = entry.attributes["commandId"]
    else {
      return false
    }

    let snapshot = pluginActionSnapshot()
    if entry.attributes["commandInput"] == nil {
      return PluginActionPlanner.directCommandRunDisabledReason(
        commandID: commandID,
        snapshot: snapshot
      ) == nil
    }

    return PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot)
  }

  func canAuthorizePluginConnector(from entry: TimelineEntry) -> Bool {
    guard let connectorID = pluginConnectorID(from: entry) else {
      return false
    }

    return canAuthorizePluginConnector(connectorID: connectorID)
  }

  func canEnablePlugin(from entry: TimelineEntry) -> Bool {
    guard isPluginRecoveryEntry(entry),
          let pluginID = pluginID(from: entry),
          let plugin = pluginSummary(pluginID: pluginID),
          !plugin.enabled
    else {
      return false
    }

    return canSetPluginEnabled(pluginID: pluginID)
  }

  func enablePlugin(from entry: TimelineEntry) {
    guard canEnablePlugin(from: entry),
          let pluginID = pluginID(from: entry)
    else {
      runtimeDetail = "Plugin enable action is unavailable."
      return
    }

    setPluginEnabled(pluginID: pluginID, enabled: true)
  }

  func canRunPluginCommandWithInput(from entry: TimelineEntry) -> Bool {
    guard isPluginCommandIssueEntry(entry),
          entry.attributes["commandInput"] == nil,
          let commandID = entry.attributes["commandId"],
          let command = pluginCommands.first(where: { $0.id == commandID }),
          command.requiresPlainInput
    else {
      return false
    }

    return PluginActionPlanner.canRunCommandWithInput(
      commandID: commandID,
      snapshot: pluginActionSnapshot()
    )
  }

  func runPluginCommandWithInput(from entry: TimelineEntry) {
    guard canRunPluginCommandWithInput(from: entry),
          let commandID = entry.attributes["commandId"]
    else {
      runtimeDetail = "Plugin command input run is unavailable."
      return
    }

    runPluginCommandWithInput(commandID: commandID)
  }

  func authorizePluginConnector(from entry: TimelineEntry) {
    guard let connectorID = pluginConnectorID(from: entry) else {
      runtimeDetail = "Plugin connector authorization is unavailable."
      return
    }

    authorizePluginConnector(connectorID: connectorID)
  }

  func retryPluginCommand(from entry: TimelineEntry) {
    guard isPluginCommandRetryableEntry(entry),
          let commandID = entry.attributes["commandId"]
    else {
      runtimeDetail = "Plugin command retry is unavailable."
      return
    }

    pluginManagerSection = .commands
    runPluginCommand(commandID: commandID, input: entry.attributes["commandInput"])
  }

  func canRevealPluginSource(from entry: TimelineEntry) -> Bool {
    isPluginSourceRevealEntry(entry)
      && pluginSourcePath(from: entry) != nil
  }

  func revealPluginSource(from entry: TimelineEntry) {
    guard let sourcePath = pluginSourcePath(from: entry) else {
      runtimeDetail = "Plugin source path is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      sourcePath,
      successDetail: "Revealed plugin source."
    )
  }

  private func runPluginCommand(commandID: String, input: String?) {
    let snapshot = pluginActionSnapshot()
    if PluginActionPlanner.commandNeedsExecutionContract(commandID: commandID, snapshot: snapshot) {
      let detail = TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail
      if let command = pluginCommands.first(where: { $0.id == commandID }) {
        appendBlockedPluginCommand(command, detail: detail, input: input)
      } else {
        runtimeDetail = detail
      }
      return
    }
    if PluginActionPlanner.commandNeedsConnectorAuth(commandID: commandID, snapshot: snapshot) {
      let detail = PluginActionPlanner.commandRunBlocker(
        commandID: commandID,
        snapshot: snapshot
      ) ?? TimelineEventPresenter.pluginCommandNeedsConnectorAuthDetail
      if let command = pluginCommands.first(where: { $0.id == commandID }) {
        appendBlockedPluginCommand(command, detail: detail, input: input)
      } else {
        runtimeDetail = detail
      }
      return
    }
    if input == nil,
       let reason = PluginActionPlanner.directCommandRunDisabledReason(
         commandID: commandID,
         snapshot: snapshot
       ) {
      runtimeDetail = reason
      return
    }

    guard PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot),
          let threadID = selectedThreadID
    else {
      runtimeDetail = PluginActionPlanner.commandRunDisabledReason(
        commandID: commandID,
        snapshot: snapshot
      ) ?? "Plugin command cannot run yet."
      return
    }

    pluginManagerSection = .commands
    runtimeDetail = TimelineEventPresenter.runningPluginCommandDetail
    let requestID = localExecutionRequests.beginAgentRequest(threadID: threadID)
    let trimmedInput = input?.trimmingCharacters(in: .whitespacesAndNewlines)
    let commandInput = trimmedInput?.isEmpty == true ? nil : trimmedInput

    let task = Task {
      defer {
        localExecutionRequests.clearAgentRequest(requestID: requestID)
      }
      do {
        let result = try await runtimeBridge.runPluginCommand(
          threadID: threadID,
          commandID: commandID,
          input: commandInput
        )
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
    localExecutionRequests.bindAgentRequest(task: task, requestID: requestID)
  }

  private func appendBlockedPluginCommand(
    _ command: PluginCommandSummary,
    detail: String,
    input: String? = nil
  ) {
    runtimeDetail = detail
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.pluginCommandBlocked(command, detail: detail, input: input)
    )
  }

  func canRunPluginCommand(commandID: String) -> Bool {
    PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: pluginActionSnapshot())
  }

  func pluginCommandRunDisabledReason(commandID: String) -> String? {
    PluginActionPlanner.commandRunDisabledReason(
      commandID: commandID,
      snapshot: pluginActionSnapshot()
    )
  }

  func hasPluginLifecycleOperation() -> Bool {
    pluginDashboardSnapshot.hasLifecycleOperation
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

  func canAuthorizePluginConnector(connectorID: String) -> Bool {
    PluginActionPlanner.canAuthorizeConnector(
      connectorID: connectorID,
      snapshot: pluginActionSnapshot()
    )
  }

  func pluginConnectorAuthorizeDisabledReason(connectorID: String) -> String? {
    PluginActionPlanner.connectorAuthorizeDisabledReason(
      connectorID: connectorID,
      snapshot: pluginActionSnapshot()
    )
  }

  func canClearPluginConnectorCredential(connectorID: String) -> Bool {
    PluginActionPlanner.canClearConnectorCredential(
      connectorID: connectorID,
      snapshot: pluginActionSnapshot()
    )
  }

  func pluginConnectorClearDisabledReason(connectorID: String) -> String? {
    PluginActionPlanner.connectorClearCredentialDisabledReason(
      connectorID: connectorID,
      snapshot: pluginActionSnapshot()
    )
  }

  func revealPluginManifest(pluginID: String) {
    guard let plugin = pluginSummary(pluginID: pluginID) else {
      runtimeDetail = "Plugin manifest path is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      plugin.manifestPath,
      successDetail: "Revealed \(plugin.displayName) manifest."
    )
  }

  func refreshPluginState() async {
    let pluginRefresh = await PluginStateLoader.refresh(using: runtimeBridge)
    updatePluginState { state in
      state.apply(pluginRefresh)
    }
    await refreshRuntimeReadiness()
  }

  func pluginActionSnapshot() -> PluginActionSnapshot {
    PluginActionSnapshot(
      runtimeState: runtimeState,
      hasRuntimeThreadSelection: hasRuntimeThreadSelection(),
      selectedThreadID: selectedThreadID,
      hasActiveOrPendingTurn: hasActiveOrPendingTurn(),
      hasLifecycleOperation: hasPluginLifecycleOperation(),
      plugins: plugins,
      connectors: pluginConnectors,
      commands: pluginCommands
    )
  }

  private func beginPluginLifecycleOperation(detail: String) -> UUID? {
    var operationID: UUID?
    updatePluginState { state in
      operationID = state.beginLifecycleOperation()
    }
    if operationID != nil {
      runtimeDetail = detail
    }
    return operationID
  }

  private func focusPluginManagerSection(capabilities: [String], permissions: [String]) {
    pluginManagerSection = PluginSurfaceClassifier.preferredSection(
      capabilities: capabilities,
      permissions: permissions
    )
  }

  private func focusAfterConnectorAuthorization(pluginID: String) {
    if pluginCommands.contains(where: { $0.pluginID == pluginID }) {
      pluginManagerSection = .commands
      return
    }

    pluginManagerSection = .connectors
  }

  private func finishPluginLifecycleOperation(_ operationID: UUID) {
    updatePluginState { state in
      state.finishLifecycleOperation(operationID)
    }
  }

  private func pluginSourcePath(from entry: TimelineEntry) -> String? {
    let explicitPath = [
      "pluginRunnerResolvedEntrypoint",
      "sourcePath",
      "pluginSourcePath",
      "pluginRunnerPluginRoot",
    ]
    .compactMap { key in
      entry.attributes[key]?.trimmingCharacters(in: .whitespacesAndNewlines)
    }
    .first { !$0.isEmpty }
    if let explicitPath {
      return explicitPath
    }

    guard let pluginID = pluginID(from: entry),
          let plugin = pluginSummary(pluginID: pluginID)
    else {
      return nil
    }

    let manifestPath = plugin.manifestPath.trimmingCharacters(in: .whitespacesAndNewlines)
    return manifestPath.isEmpty ? nil : manifestPath
  }

  private func isPluginSourceRevealEntry(_ entry: TimelineEntry) -> Bool {
    isPluginCommandIssueEntry(entry)
      || isPluginInstallIssueEntry(entry)
      || isPluginConnectorIssueEntry(entry)
      || isPluginCommandApprovalEntry(entry)
  }

  private func isPluginRecoveryEntry(_ entry: TimelineEntry) -> Bool {
    isPluginCommandIssueEntry(entry)
      || isPluginInstallIssueEntry(entry)
      || entry.attributes["pluginId"] != nil
  }

  private func pluginID(from entry: TimelineEntry) -> String? {
    if let pluginID = entry.attributes["pluginId"]?.trimmingCharacters(in: .whitespacesAndNewlines),
       !pluginID.isEmpty
    {
      return pluginID
    }

    guard let qualifiedID = entry.attributes["commandId"]
      ?? entry.attributes["connectorId"]
    else {
      return nil
    }

    guard let separatorRange = qualifiedID.range(of: "::") else {
      return nil
    }

    let pluginID = String(qualifiedID[..<separatorRange.lowerBound])
    return pluginID.isEmpty ? nil : pluginID
  }

  private func pluginConnectorID(from entry: TimelineEntry) -> String? {
    if isPluginCommandIssueEntry(entry),
       let commandID = entry.attributes["commandId"],
       let connectorID = PluginActionPlanner.commandAuthorizationConnectorID(
         commandID: commandID,
         snapshot: pluginActionSnapshot()
       ) {
      return connectorID
    }

    if let connectorID = entry.attributes["connectorId"]?.trimmingCharacters(in: .whitespacesAndNewlines),
       !connectorID.isEmpty
    {
      return connectorID
    }

    for key in ["connectorIds", "pluginRunnerConnectorId", "pluginRunnerConnectorIds"] {
      if let connectorID = singleConnectorID(from: entry.attributes[key]) {
        return connectorID
      }
    }

    return nil
  }

  private func singleConnectorID(from value: String?) -> String? {
    let connectorIDs = value?
      .split(separator: ",")
      .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
      .filter { !$0.isEmpty } ?? []
    return connectorIDs.count == 1 ? connectorIDs[0] : nil
  }

  private func isPluginCommandIssueEntry(_ entry: TimelineEntry) -> Bool {
    entry.attributes["pluginCommandStatus"] == "failed"
      || entry.attributes["pluginCommandStatus"] == "blocked"
      || entry.attributes["pluginCommandRouting"] != nil
  }

  private func isPluginInstallIssueEntry(_ entry: TimelineEntry) -> Bool {
    switch entry.attributes["pluginInstallStatus"] {
    case "failed",
         "previewFailed",
         "inspectFailed",
         "installFailed",
         "refreshFailed",
         "blocked",
         "alreadyInstalled":
      return true
    default:
      return false
    }
  }

  private func isPluginConnectorIssueEntry(_ entry: TimelineEntry) -> Bool {
    entry.attributes["connectorStatus"] != nil
      || entry.attributes["connectorRepairHint"] != nil
  }

  private func isPluginCommandApprovalEntry(_ entry: TimelineEntry) -> Bool {
    entry.kind == .approval
      && entry.attributes["action"] == "run_plugin_command"
      && entry.attributes["sourcePath"] != nil
  }

  private func isPluginCommandRetryableEntry(_ entry: TimelineEntry) -> Bool {
    isPluginCommandIssueEntry(entry)
  }
}
