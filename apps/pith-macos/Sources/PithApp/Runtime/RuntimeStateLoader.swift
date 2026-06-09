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
      runtimeDetail: serverLabel.map { "\($0) | \(runtimeModel.displayName)" }
    )
  }

  static func refreshRuntimeReadiness(
    using runtimeBridge: RuntimeBridge
  ) async -> RuntimeReadinessSummary? {
    do {
      let readiness = try await runtimeBridge.runtimeReadiness()
      return RuntimeSummaryMapper.readinessSummary(from: readiness)
    } catch {
      return RuntimeReadinessSummary(
        status: "unavailable",
        summary: "Local engine readiness unavailable: \(error.localizedDescription)",
        checks: [
          RuntimeReadinessCheckSummary(
            id: "runtime-readiness",
            title: "Local Engine Readiness",
            status: "unavailable",
            detail: error.localizedDescription
          )
        ],
        metrics: ["error": error.localizedDescription]
      )
    }
  }

  private static func modelHealthFailureDetail(serverLabel: String?, error: Error) -> String {
    let detail = "Model setup unavailable: \(error.localizedDescription)"
    guard let serverLabel, !serverLabel.isEmpty else {
      return detail
    }

    return "\(serverLabel) | \(detail)"
  }
}
