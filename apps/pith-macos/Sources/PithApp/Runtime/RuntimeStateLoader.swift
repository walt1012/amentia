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
      runtimeDetail: serverLabel.map {
        "\($0). \(LocalModelDisplayPresenter.cleanDisplayName(runtimeModel.displayName))"
      }
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

  private static func modelHealthFailureDetail(serverLabel: String?, error: Error) -> String {
    let detail = userFacingModelSetupFailure(error)
    guard let serverLabel, !serverLabel.isEmpty else {
      return detail
    }

    return "\(serverLabel). \(detail)"
  }

  private static func userFacingModelSetupFailure(_ error: Error) -> String {
    let rawDetail = error.localizedDescription.lowercased()
    if rawDetail.contains("setup verification")
      || rawDetail.contains("backend")
      || rawDetail.contains("llama")
    {
      return "Pith could not start its local model runner. Reinstall Pith from the latest release, then reopen the app."
    }

    if rawDetail.contains("checksum")
      || rawDetail.contains("integrity")
      || rawDetail.contains("sha")
    {
      return "The selected model did not pass verification. Re-download the model from Pith before using it."
    }

    return "Pith could not verify local model setup. Re-download the selected model or restart Pith."
  }
}
