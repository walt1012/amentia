import Foundation

@MainActor
extension AppViewModel {
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
        ?? "Finish the current connector operation before authorizing a connector."
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
          credentialSecret: credentialInput.secret
        )
        guard await refreshPluginStateIfCurrent(operationID: operationID) else {
          return
        }
        focusAfterConnectorAuthorization(pluginID: connector.pluginID)
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorAuthorized(connector),
          detail: "Connector authorized. Actions were refreshed.",
          preview: "Connector authorized"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorAuthFailed(connectorID: connectorID, error: error),
          detail: error.localizedDescription,
          preview: "Connector authorization failed"
        )
      }
    }
    bindPluginLifecycleTask(task, operationID: operationID)
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
        ?? "Finish the current connector operation before clearing a connector credential."
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
          TimelineEventPresenter.pluginConnectorCredentialCleared(connector),
          detail: "Connector credential cleared. Actions were refreshed.",
          preview: "Connector credential cleared"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginConnectorCredentialClearFailed(
            connectorID: connectorID,
            error: error
          ),
          detail: error.localizedDescription,
          preview: "Connector credential clear failed"
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
