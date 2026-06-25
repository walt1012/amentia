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
        TimelinePluginEventPresenter.pluginCatalogRefreshed(
          pluginSummary: pluginCountSummary(),
          surfaceSummary: pluginSurfaceSummary(),
          diagnostics: snapshot.diagnostics,
          recoveryAttributes: snapshot.refreshRecoveryAttributes
        ),
        detail: hasDiagnostics
          ? "Plugin setup refreshed and needs attention."
          : "Plugin setup refreshed.",
        preview: hasDiagnostics ? "Plugin setup needs attention" : "Plugins refreshed"
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
            TimelinePluginEventPresenter.pluginInstallBlocked(preview: preview),
            detail: detail,
            preview: "Plugin install needs attention"
          )
          return
        }
        guard PluginInstallDialogPresenter.confirmInstall(preview: preview) else {
          runtimeDetail = "Plugin install was cancelled."
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
          TimelinePluginEventPresenter.pluginInstalled(installedPlugin, preview: preview),
          detail: installedPlugin.enabled
            ? "Plugin installed and enabled."
            : "Plugin installed. Enable it before running actions.",
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
          TimelinePluginEventPresenter.pluginUpdated(updatedPlugin, enabled: enabled),
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
          TimelinePluginEventPresenter.pluginUpdateFailed(
            pluginID: pluginID,
            enabled: enabled,
            error: error
          ),
          detail: UserFacingFailurePresenter.pluginLifecycleFailureBody(
            action: enabled ? "enable" : "disable"
          ),
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
      detail: "Removing plugin..."
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
          TimelinePluginEventPresenter.pluginRemoved(removedPlugin),
          detail: "\(removedPlugin.displayName) was removed.",
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
          TimelinePluginEventPresenter.pluginRemovalFailed(pluginID: pluginID, error: error),
          detail: UserFacingFailurePresenter.pluginLifecycleFailureBody(action: "removal"),
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
        TimelinePluginEventPresenter.pluginInstallPreviewFailed(
          error: error,
          repairHint: repairHint,
          sourcePath: sourcePath
        ),
        detail: UserFacingFailurePresenter.pluginPreviewFailureBody(repairHint: repairHint),
        preview: "Plugin preview failed"
      )
      return
    }

    appendPluginStatusEntry(
      to: threadID,
      TimelinePluginEventPresenter.pluginInstallFailed(
        error: error,
        repairHint: repairHint,
        sourcePath: confirmedPreview?.sourcePath
      ),
      detail: UserFacingFailurePresenter.pluginInstallFailureBody(repairHint: repairHint),
      preview: "Plugin install failed"
    )
  }
}
