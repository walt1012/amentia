import Foundation

@MainActor
extension AppViewModel {
  func authorizePluginConnector(connectorID: String) {
    guard canAuthorizePluginConnector(connectorID: connectorID) else {
      runtimeDetail = pluginConnectorAuthorizeDisabledReason(connectorID: connectorID)
        ?? "Connection cannot be authorized yet."
      return
    }
    guard let connector = pluginConnectors.first(where: { $0.id == connectorID }) else {
      runtimeDetail = "Connection is not loaded."
      return
    }
    guard let credentialInput = PluginConnectorCredentialDialogPresenter.credentialInput(
      connector: connector
    ) else {
      runtimeDetail = "Connection authorization was cancelled."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Authorizing connection..."
    ) else {
      runtimeDetail = pluginConnectorAuthorizeDisabledReason(connectorID: connectorID)
        ?? "Finish the current connection operation before authorizing another connection."
      return
    }
    let timelineThreadID = selectedThreadID

    let task = Task { @MainActor in
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let connector = try await runtimeBridge.authorizePluginConnector(
          connectorID: connectorID,
          credentialLabel: credentialInput.label,
          credentialSecret: credentialInput.tokenOrKey
        )
        guard await refreshPluginStateIfCurrent(operationID: operationID) else {
          return
        }
        focusAfterConnectorAuthorization(pluginID: connector.pluginID)
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelinePluginEventPresenter.pluginConnectorAuthorized(connector),
          detail: "Connection authorized. Actions were refreshed.",
          preview: "Connection authorized"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelinePluginEventPresenter.pluginConnectorAuthFailed(connectorID: connectorID, error: error),
          detail: error.localizedDescription,
          preview: "Connection authorization failed"
        )
      }
    }
    bindPluginLifecycleTask(task, operationID: operationID)
  }

  func clearPluginConnectorCredential(connectorID: String) {
    guard canClearPluginConnectorCredential(connectorID: connectorID) else {
      runtimeDetail = pluginConnectorClearDisabledReason(connectorID: connectorID)
        ?? "Connection authorization cannot be cleared yet."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Clearing connection authorization..."
    ) else {
      runtimeDetail = pluginConnectorClearDisabledReason(connectorID: connectorID)
        ?? "Finish the current connection operation before clearing saved authorization."
      return
    }
    let timelineThreadID = selectedThreadID

    let task = Task { @MainActor in
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let connector = try await runtimeBridge.clearPluginConnectorCredential(
          connectorID: connectorID
        )
        guard await refreshPluginStateIfCurrent(operationID: operationID) else {
          return
        }
        pluginManagerSection = .connectors
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelinePluginEventPresenter.pluginConnectorCredentialCleared(connector),
          detail: "Connection authorization cleared. Actions were refreshed.",
          preview: "Connection authorization cleared"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelinePluginEventPresenter.pluginConnectorCredentialClearFailed(
            connectorID: connectorID,
            error: error
          ),
          detail: error.localizedDescription,
          preview: "Connection authorization clear failed"
        )
      }
    }
    bindPluginLifecycleTask(task, operationID: operationID)
  }

  private func focusAfterConnectorAuthorization(pluginID: String) {
    if pluginCommands.contains(where: { $0.pluginID == pluginID }) {
      pluginManagerSection = .commands
      return
    }

    pluginManagerSection = .connectors
  }
}
