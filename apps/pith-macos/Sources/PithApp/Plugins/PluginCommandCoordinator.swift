import Foundation

struct PluginCommandRunRequest {
  let threadID: String
  let commandID: String
  let input: String?
}

enum PluginCommandRunPreparation {
  case ready(PluginCommandRunRequest)
  case blocked(command: PluginCommandSummary, detail: String, input: String?)
  case unavailable(detail: String)
}

enum PluginCommandCoordinator {
  static func prepareRun(
    commandID: String,
    input: String?,
    selectedThreadID: String?,
    snapshot: PluginActionSnapshot,
    commands: [PluginCommandSummary]
  ) -> PluginCommandRunPreparation {
    if PluginActionPlanner.commandNeedsExecutionContract(commandID: commandID, snapshot: snapshot) {
      return blockedOrUnavailable(
        commandID: commandID,
        commands: commands,
        detail: TimelineEventPresenter.pluginCommandNeedsExecutionContractDetail,
        input: input
      )
    }

    if PluginActionPlanner.commandNeedsConnectorAuth(commandID: commandID, snapshot: snapshot) {
      let detail = PluginActionPlanner.commandRunBlocker(
        commandID: commandID,
        snapshot: snapshot
      ) ?? TimelineEventPresenter.pluginCommandNeedsConnectorAuthDetail
      return blockedOrUnavailable(
        commandID: commandID,
        commands: commands,
        detail: detail,
        input: input
      )
    }

    if input == nil,
       let reason = PluginActionPlanner.directCommandRunDisabledReason(
         commandID: commandID,
         snapshot: snapshot
       )
    {
      return .unavailable(detail: reason)
    }

    guard PluginActionPlanner.canRunCommand(commandID: commandID, snapshot: snapshot),
          let selectedThreadID
    else {
      return .unavailable(
        detail: PluginActionPlanner.commandRunDisabledReason(
          commandID: commandID,
          snapshot: snapshot
        ) ?? "Action cannot run yet."
      )
    }

    return .ready(
      PluginCommandRunRequest(
        threadID: selectedThreadID,
        commandID: commandID,
        input: normalizedInput(input)
      )
    )
  }

  static func run(
    request: PluginCommandRunRequest,
    runtimeBridge: RuntimeBridge
  ) async throws -> RuntimeBridge.RuntimeTurnResult {
    try await runtimeBridge.runPluginCommand(
      threadID: request.threadID,
      commandID: request.commandID,
      input: request.input
    )
  }

  private static func blockedOrUnavailable(
    commandID: String,
    commands: [PluginCommandSummary],
    detail: String,
    input: String?
  ) -> PluginCommandRunPreparation {
    guard let command = commands.first(where: { $0.id == commandID }) else {
      return .unavailable(detail: detail)
    }

    return .blocked(command: command, detail: detail, input: input)
  }

  private static func normalizedInput(_ input: String?) -> String? {
    let trimmedInput = input?.trimmingCharacters(in: .whitespacesAndNewlines)
    return trimmedInput?.isEmpty == true ? nil : trimmedInput
  }
}
