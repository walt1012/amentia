import Foundation

@MainActor
extension AppViewModel {
  func runPluginCommand(commandID: String) {
    runPluginCommand(commandID: commandID, input: nil)
  }

  func runPluginCommandWithInput(commandID: String) {
    guard let command = pluginCommands.first(where: { $0.id == commandID }) else {
      runtimeDetail = "Plugin command is not loaded."
      return
    }
    let snapshot = pluginActionSnapshot()
    if PluginActionPlanner.commandNeedsExecutionContract(
      commandID: commandID,
      snapshot: snapshot
    ) {
      appendBlockedPluginCommand(
        command,
        detail: TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail
      )
      return
    }
    if PluginActionPlanner.commandNeedsConnectorAuth(commandID: commandID, snapshot: snapshot) {
      let detail = PluginActionPlanner.commandRunBlocker(
        commandID: commandID,
        snapshot: snapshot
      )
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
          let commandID = pluginRetryCommandID(from: entry)
    else {
      return false
    }

    let snapshot = pluginActionSnapshot()
    if pluginRetryInput(from: entry) == nil {
      return PluginActionPlanner.directCommandRunDisabledReason(
        commandID: commandID,
        snapshot: snapshot
      ) == nil
    }

    return PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot)
  }

  func canRunPluginCommandWithInput(from entry: TimelineEntry) -> Bool {
    guard (isPluginCommandIssueEntry(entry) || isPluginCommandRetryableEntry(entry)),
          pluginRetryInput(from: entry) == nil,
          let commandID = pluginRetryCommandID(from: entry),
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
          let commandID = pluginRetryCommandID(from: entry)
    else {
      runtimeDetail = "Plugin command input run is unavailable."
      return
    }

    runPluginCommandWithInput(commandID: commandID)
  }

  func retryPluginCommand(from entry: TimelineEntry) {
    guard canRetryPluginCommand(from: entry),
          let commandID = pluginRetryCommandID(from: entry)
    else {
      runtimeDetail = "Plugin command retry is unavailable."
      return
    }

    pluginManagerSection = .commands
    runPluginCommand(commandID: commandID, input: pluginRetryInput(from: entry))
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

  private func runPluginCommand(commandID: String, input: String?) {
    let preparation = PluginCommandCoordinator.prepareRun(
      commandID: commandID,
      input: input,
      selectedThreadID: selectedThreadID,
      snapshot: pluginActionSnapshot(),
      commands: pluginCommands
    )
    guard case .ready(let request) = preparation else {
      applyPluginCommandRunPreparation(preparation)
      return
    }

    pluginManagerSection = .commands
    runtimeDetail = TimelineEventPresenter.runningPluginCommandDetail
    let requestID = localExecutionRequests.beginPluginCommandRequest(threadID: request.threadID)

    let task = Task {
      defer {
        localExecutionRequests.clearLocalWorkRequest(requestID: requestID)
      }
      do {
        let result = try await PluginCommandCoordinator.run(
          request: request,
          runtimeBridge: runtimeBridge
        )
        await applyRuntimeTurnResult(result, refreshMemory: true)
      } catch {
        handlePluginCommandFailure(error, request: request)
      }
    }
    localExecutionRequests.bindLocalWorkRequest(task: task, requestID: requestID)
  }

  private func applyPluginCommandRunPreparation(_ preparation: PluginCommandRunPreparation) {
    switch preparation {
    case .ready:
      return
    case .blocked(let command, let detail, let input):
      appendBlockedPluginCommand(command, detail: detail, input: input)
    case .unavailable(let detail):
      runtimeDetail = detail
    }
  }

  private func handlePluginCommandFailure(
    _ error: Error,
    request: PluginCommandRunRequest
  ) {
    if Task.isCancelled {
      runtimeDetail = TimelineEventPresenter.pendingPluginCommandCancelledDetail
      refreshThreadPreview(
        threadID: request.threadID,
        preview: TimelineEventPresenter.cancelledPluginCommandPreview
      )
      appendEntry(
        to: request.threadID,
        TimelineEventPresenter.pluginCommandCancelled()
      )
      return
    }

    let failureEntry = TimelineEventPresenter.pluginCommandFailed(error: error)
    if failureEntry.attributes["pluginCommandStatus"] == "blocked" {
      refreshThreadPreview(
        threadID: request.threadID,
        preview: TimelineEventPresenter.blockedPluginCommandPreview
      )
    } else {
      refreshThreadPreview(
        threadID: request.threadID,
        preview: TimelineEventPresenter.failedPluginCommandPreview
      )
    }
    runtimeDetail = error.localizedDescription
    appendEntry(
      to: request.threadID,
      failureEntry
    )
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
}
