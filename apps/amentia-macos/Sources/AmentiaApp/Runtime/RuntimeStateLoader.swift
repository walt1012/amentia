import Foundation

struct ModelHealthRefresh {
  let modelHealth: ModelHealthSummary?
  let runtimeDetail: String?
}

enum RuntimeStateLoader {
  static func refreshModelHealth(
    using runtimeBridge: RuntimeBridge,
    serverLabel: String?
  ) async -> ModelHealthRefresh {
    let runtimeModel: RuntimeBridge.RuntimeModelHealth
    do {
      runtimeModel = try await runtimeBridge.modelHealth()
    } catch {
      return ModelHealthRefresh(
        modelHealth: nil,
        runtimeDetail: modelHealthFailureDetail(serverLabel: serverLabel, error: error)
      )
    }

    return ModelHealthRefresh(
      modelHealth: RuntimeSummaryMapper.modelHealthSummary(from: runtimeModel),
      runtimeDetail: connectedModelDetail(runtimeModel, serverLabel: serverLabel)
    )
  }

  static func refreshRuntimeReadiness(
    using runtimeBridge: RuntimeBridge
  ) async -> RuntimeReadinessSummary? {
    do {
      let readiness = try await runtimeBridge.runtimeReadiness()
      return RuntimeSummaryMapper.readinessSummary(from: readiness)
    } catch {
      let detail = userFacingModelSetupFailure(error)
      return RuntimeReadinessSummary(
        status: "unavailable",
        summary: "Local model setup needs attention.",
        checks: [
          RuntimeReadinessCheckSummary(
            id: "model-setup",
            title: "Local Model Setup",
            status: "unavailable",
            detail: detail
          )
        ],
        metrics: ["technicalError": error.localizedDescription]
      )
    }
  }

  private static func modelHealthFailureDetail(serverLabel _: String?, error: Error) -> String {
    RuntimeModelSetupFailurePresenter.detail(error: error)
  }

  private static func connectedModelDetail(
    _ runtimeModel: RuntimeBridge.RuntimeModelHealth,
    serverLabel: String?
  ) -> String? {
    guard serverLabel != nil else {
      return nil
    }

    let modelName = LocalModelDisplayPresenter.cleanDisplayName(runtimeModel.displayName)
    return "Amentia is connected. \(modelName)"
  }

  private static func userFacingModelSetupFailure(_ error: Error) -> String {
    RuntimeModelSetupFailurePresenter.detail(error: error)
  }
}

enum RuntimeModelSetupFailurePresenter {
  static func detail(error: Error) -> String {
    let rawDetail = error.localizedDescription.lowercased()
    if rawDetail.contains("checksum")
      || rawDetail.contains("integrity")
      || rawDetail.contains("sha")
    {
      return "The selected model did not pass verification. Re-download the model from Amentia before using it."
    }

    if rawDetail.contains("backend")
      || rawDetail.contains("llama")
    {
      return "Amentia could not start its local model engine. Reinstall Amentia from the latest release, then reopen the app."
    }

    return "Amentia could not verify local model setup. Restart Amentia, then refresh model setup if this returns."
  }
}
