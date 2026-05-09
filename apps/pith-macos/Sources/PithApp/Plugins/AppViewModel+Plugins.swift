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

    Task {
      do {
        let installedPlugin = try await runtimeBridge.installPlugin(sourcePath: preview.sourcePath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginInstalled(installedPlugin, preview: preview)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginInstallFailed(error: error)
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
          TimelineEventPresenter.pluginUpdated(updatedPlugin, enabled: enabled)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
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

    Task {
      do {
        let removedPlugin = try await runtimeBridge.removePlugin(manifestPath: plugin.manifestPath)
        await refreshPluginState()
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginRemoved(removedPlugin)
        )
      } catch {
        appendEntry(
          to: selectedThreadID,
          TimelineEventPresenter.pluginRemovalFailed(pluginID: pluginID, error: error)
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
      plugins: plugins,
      commands: pluginCommands
    )
  }
}
