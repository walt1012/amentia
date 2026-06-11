import Foundation

@MainActor
extension AppViewModel {
  func appendPluginStatusEntry(
    to threadID: String?,
    _ entry: TimelineEntry,
    detail: String,
    preview: String
  ) {
    runtimeDetail = detail
    if let threadID {
      refreshThreadPreview(threadID: threadID, preview: preview)
    }
    appendEntry(to: threadID, entry)
  }

  func hasPluginLifecycleOperation() -> Bool {
    pluginDashboardSnapshot.hasLifecycleOperation || pluginLifecycleOperations.isActive
  }

  func canCancelPluginLifecycleOperation() -> Bool {
    hasPluginLifecycleOperation()
  }

  func cancelPluginLifecycleOperation() {
    guard canCancelPluginLifecycleOperation() else {
      return
    }

    let timelineThreadID = selectedThreadID
    pluginLifecycleOperations.cancel()
    updatePluginState { state in
      state.resetLifecycleOperation()
    }
    appendPluginStatusEntry(
      to: timelineThreadID,
      TimelineEventPresenter.pluginLifecycleCancelled(),
      detail: TimelineEventPresenter.pluginLifecycleCancelledDetail,
      preview: TimelineEventPresenter.cancelledPluginLifecyclePreview
    )
  }

  func canRefreshPlugins() -> Bool {
    pluginRefreshDisabledReason() == nil
  }

  func pluginRefreshDisabledReason() -> String? {
    if runtimeState != .ready {
      return "Local service is not ready."
    }
    if hasActiveOrPendingTurn() {
      return "Finish or cancel the active task first."
    }
    if hasPluginLifecycleOperation() {
      return "Finish the current connector operation first."
    }

    return nil
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

  func refreshPluginState() async {
    let pluginRefresh = await PluginLifecycleCoordinator.refreshState(using: runtimeBridge)
    updatePluginState { state in
      state.apply(pluginRefresh)
    }
    await refreshRuntimeReadiness()
  }

  @discardableResult
  func refreshPluginStateIfCurrent(operationID: UUID) async -> Bool {
    let pluginRefresh = await PluginLifecycleCoordinator.refreshState(using: runtimeBridge)
    guard !Task.isCancelled,
          isCurrentPluginLifecycleOperation(operationID)
    else {
      return false
    }

    updatePluginState { state in
      state.apply(pluginRefresh)
    }
    await refreshRuntimeReadiness()
    return !Task.isCancelled && isCurrentPluginLifecycleOperation(operationID)
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

  func beginPluginLifecycleOperation(detail: String) -> UUID? {
    guard let operationID = pluginLifecycleOperations.begin() else {
      return nil
    }
    var accepted = false
    updatePluginState { state in
      accepted = state.beginLifecycleOperation(operationID: operationID)
    }
    guard accepted else {
      pluginLifecycleOperations.finish(operationID)
      return nil
    }

    runtimeDetail = detail
    return operationID
  }

  func bindPluginLifecycleTask(_ task: Task<Void, Never>, operationID: UUID) {
    pluginLifecycleOperations.bind(task: task, operationID: operationID)
  }

  func isCurrentPluginLifecycleOperation(_ operationID: UUID) -> Bool {
    pluginLifecycleOperations.isCurrent(operationID)
  }

  func focusPluginManagerSection(capabilities: [String], permissions: [String]) {
    pluginManagerSection = PluginSurfaceClassifier.preferredSection(
      capabilities: capabilities,
      permissions: permissions
    )
  }

  func finishPluginLifecycleOperation(_ operationID: UUID) {
    pluginLifecycleOperations.finish(operationID)
    updatePluginState { state in
      state.finishLifecycleOperation(operationID)
    }
  }
}
