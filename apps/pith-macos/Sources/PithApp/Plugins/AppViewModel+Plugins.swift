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

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Installing local plugin..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before starting another."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let installedPlugin = try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
        await refreshPluginState()
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginInstalled(installedPlugin, preview: preview)
        )
      } catch {
        appendEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginInstallFailed(error: error)
        )
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
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Authorizing connector..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before authorizing a connector."
      return
    }
    let timelineThreadID = selectedThreadID

    Task {
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let connector = try await runtimeBridge.authorizePluginConnector(connectorID: connectorID)
        await refreshPluginState()
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
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Clearing connector credential..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before clearing a connector credential."
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
    let snapshot = pluginActionSnapshot()
    if PluginActionPlanner.commandNeedsExecutionContract(commandID: commandID, snapshot: snapshot) {
      runtimeDetail = TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail
      return
    }
    if PluginActionPlanner.commandNeedsConnectorAuth(commandID: commandID, snapshot: snapshot) {
      runtimeDetail = PluginActionPlanner.commandRunBlocker(
        commandID: commandID,
        snapshot: snapshot
      ) ?? TimelineEventPresenter.pluginCommandNeedsConnectorAuthDetail
      return
    }

    guard PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot),
          let threadID = selectedThreadID
    else {
      return
    }

    runtimeDetail = TimelineEventPresenter.runningPluginCommandDetail
    let requestID = localExecutionRequests.beginAgentRequest(threadID: threadID)

    let task = Task {
      defer {
        localExecutionRequests.clearAgentRequest(requestID: requestID)
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
    localExecutionRequests.bindAgentRequest(task: task, requestID: requestID)
  }

  func canRunPluginCommand(commandID: String) -> Bool {
    PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: pluginActionSnapshot())
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

  func canClearPluginConnectorCredential(connectorID: String) -> Bool {
    PluginActionPlanner.canClearConnectorCredential(
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
      isLocalModelReady: isLocalModelReady(),
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

  private func finishPluginLifecycleOperation(_ operationID: UUID) {
    updatePluginState { state in
      state.finishLifecycleOperation(operationID)
    }
  }
}
