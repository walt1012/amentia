import Foundation

@MainActor
extension AppViewModel {
  func refreshPlugins() {
    guard canRefreshPlugins() else {
      runtimeDetail = pluginRefreshDisabledReason() ?? "Plugin refresh is unavailable."
      return
    }

    guard let operationID = beginPluginLifecycleOperation(
      detail: "Refreshing plugins..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before refreshing plugins."
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
          ? "Plugin catalog refreshed with diagnostics."
          : "Plugin catalog refreshed.",
        preview: hasDiagnostics ? "Plugin refresh diagnostics" : "Plugins refreshed"
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
      detail: "Inspecting local plugin..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before starting another."
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
          let detail = preview.installBlocker ?? "Plugin cannot be installed yet."
          appendPluginStatusEntry(
            to: timelineThreadID,
            TimelineEventPresenter.pluginInstallBlocked(preview: preview),
            detail: detail,
            preview: "Plugin install blocked"
          )
          return
        }
        guard PluginInstallDialogPresenter.confirmInstall(preview: preview) else {
          runtimeDetail = "Plugin install was cancelled."
          return
        }
        confirmedPreview = preview
        runtimeDetail = "Installing local plugin..."
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
            ? "Plugin installed and enabled."
            : "Plugin installed. Enable it before running commands.",
          preview: "Plugin installed"
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
      detail: enabled ? "Enabling plugin..." : "Disabling plugin..."
    ) else {
      runtimeDetail = "Finish the current plugin operation before changing another plugin."
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
          preview: enabled ? "Plugin enabled" : "Plugin disabled"
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
          preview: enabled ? "Plugin enable failed" : "Plugin disable failed"
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
          detail: "\(removedPlugin.displayName) was removed from the local plugin catalog.",
          preview: "Plugin removed"
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
          preview: "Plugin removal failed"
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
        preview: "Plugin install preview failed"
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
      preview: "Plugin install failed"
    )
  }
}
