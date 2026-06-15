import Foundation

@MainActor
extension AppViewModel {
  func refreshPlugins() {
    guard canRefreshPlugins() else {
      runtimeDetail = pluginRefreshDisabledReason() ?? "Connector refresh is unavailable."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Refreshing connectors..."
    ) else {
      runtimeDetail = "Finish the current connector operation before refreshing connectors."
      return
    }
    let timelineThreadID = selectedThreadID

    let task = Task { @MainActor in
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      guard await refreshPluginStateIfCurrent(operationID: operationID) else {
        return
      }

      let snapshot = pluginDashboardSnapshot
      let hasDiagnostics = !snapshot.diagnostics.isEmpty
      appendPluginStatusEntry(
        to: timelineThreadID,
        TimelineEventPresenter.pluginCatalogRefreshed(
          pluginSummary: pluginCountSummary(),
          surfaceSummary: pluginSurfaceSummary(),
          diagnostics: snapshot.diagnostics,
          recoveryAttributes: snapshot.refreshRecoveryAttributes
        ),
        detail: hasDiagnostics
          ? "Connector setup refreshed and needs attention."
          : "Connector setup refreshed.",
        preview: hasDiagnostics ? "Connector setup needs attention" : "Connectors refreshed"
      )
    }
    bindPluginLifecycleTask(task, operationID: operationID)
  }

  func installPlugin() {
    guard canInstallPlugin() else {
      return
    }

    guard let url = AppFilePicker.choosePluginSource() else {
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Inspecting plugin..."
    ) else {
      runtimeDetail = "Finish the current connector operation before starting another."
      return
    }
    let timelineThreadID = selectedThreadID

    let task = Task { @MainActor in
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      var confirmedPreview: PluginInstallPreview?
      do {
        let preview = try await PluginLifecycleCoordinator.inspectInstallSource(
          url,
          runtimeBridge: runtimeBridge,
          installRootPath: runtimeBridge.localPluginInstallRootPath()
        )
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        guard preview.canInstall else {
          let detail = preview.installBlocker ?? "Connector cannot be installed yet."
          appendPluginStatusEntry(
            to: timelineThreadID,
            TimelineEventPresenter.pluginInstallBlocked(preview: preview),
            detail: detail,
            preview: "Connector install needs attention"
          )
          return
        }
        guard PluginInstallDialogPresenter.confirmInstall(preview: preview) else {
          runtimeDetail = "Connector install was cancelled."
          return
        }
        confirmedPreview = preview
        runtimeDetail = "Installing plugin..."
        let installedPlugin = try await PluginLifecycleCoordinator.installConfirmedPlugin(
          preview: preview,
          runtimeBridge: runtimeBridge
        )
        guard await refreshPluginStateIfCurrent(operationID: operationID) else {
          return
        }
        focusPluginManagerSection(
          capabilities: installedPlugin.capabilities,
          permissions: installedPlugin.permissions
        )
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginInstalled(installedPlugin, preview: preview),
          detail: installedPlugin.enabled
            ? "Connector installed and enabled."
            : "Connector installed. Enable it before running actions.",
          preview: "Connector installed"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginInstallFailure(
          to: timelineThreadID,
          error: error,
          sourcePath: url.path,
          confirmedPreview: confirmedPreview
        )
      }
    }
    bindPluginLifecycleTask(task, operationID: operationID)
  }

  func setPluginEnabled(pluginID: String, enabled: Bool) {
    guard canSetPluginEnabled(pluginID: pluginID) else {
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: enabled ? "Enabling connector..." : "Disabling connector..."
    ) else {
      runtimeDetail = "Finish the current connector operation before changing another connector."
      return
    }
    let timelineThreadID = selectedThreadID

    let task = Task { @MainActor in
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let updatedPlugin = try await PluginLifecycleCoordinator.setPluginEnabled(
          pluginID: pluginID,
          enabled: enabled,
          runtimeBridge: runtimeBridge
        )
        guard await refreshPluginStateIfCurrent(operationID: operationID) else {
          return
        }
        focusPluginManagerSection(
          capabilities: updatedPlugin.capabilities,
          permissions: updatedPlugin.permissions
        )
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginUpdated(updatedPlugin, enabled: enabled),
          detail: "\(updatedPlugin.displayName) is now \(enabled ? "enabled" : "disabled").",
          preview: enabled ? "Connector enabled" : "Connector disabled"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginUpdateFailed(
            pluginID: pluginID,
            enabled: enabled,
            error: error
          ),
          detail: error.localizedDescription,
          preview: enabled ? "Connector enable failed" : "Connector disable failed"
        )
      }
    }
    bindPluginLifecycleTask(task, operationID: operationID)
  }

  func removePlugin(pluginID: String) {
    guard canRemovePlugin(pluginID: pluginID),
          let plugin = pluginSummary(pluginID: pluginID)
    else {
      return
    }

    guard PluginInstallDialogPresenter.confirmRemoval(plugin: plugin) else {
      runtimeDetail = "Connector removal was cancelled."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Removing plugin..."
    ) else {
      runtimeDetail = "Finish the current connector operation before removing another connector."
      return
    }
    let timelineThreadID = selectedThreadID

    let task = Task { @MainActor in
      defer {
        finishPluginLifecycleOperation(operationID)
      }
      do {
        let removedPlugin = try await PluginLifecycleCoordinator.removePlugin(
          plugin: plugin,
          runtimeBridge: runtimeBridge
        )
        guard await refreshPluginStateIfCurrent(operationID: operationID) else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginRemoved(removedPlugin),
          detail: "\(removedPlugin.displayName) was removed from Connectors.",
          preview: "Connector removed"
        )
      } catch {
        guard !Task.isCancelled,
              isCurrentPluginLifecycleOperation(operationID)
        else {
          return
        }
        appendPluginStatusEntry(
          to: timelineThreadID,
          TimelineEventPresenter.pluginRemovalFailed(pluginID: pluginID, error: error),
          detail: error.localizedDescription,
          preview: "Connector removal failed"
        )
      }
    }
    bindPluginLifecycleTask(task, operationID: operationID)
  }

  private func appendPluginInstallFailure(
    to threadID: String?,
    error: Error,
    sourcePath: String,
    confirmedPreview: PluginInstallPreview?
  ) {
    let repairHint = PluginInstallDialogPresenter.repairHint(for: error)
    if confirmedPreview == nil {
      appendPluginStatusEntry(
        to: threadID,
        TimelineEventPresenter.pluginInstallPreviewFailed(
          error: error,
          repairHint: repairHint,
          sourcePath: sourcePath
        ),
        detail: error.localizedDescription,
        preview: "Connector preview failed"
      )
      return
    }

    appendPluginStatusEntry(
      to: threadID,
      TimelineEventPresenter.pluginInstallFailed(
        error: error,
        repairHint: repairHint,
        sourcePath: confirmedPreview?.sourcePath
      ),
      detail: error.localizedDescription,
      preview: "Connector install failed"
    )
  }
}
