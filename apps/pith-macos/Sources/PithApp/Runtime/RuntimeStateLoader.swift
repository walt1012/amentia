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
    let runtimeModel = try? await runtimeBridge.modelHealth()

    guard let runtimeModel else {
      return ModelHealthRefresh(modelHealth: nil, runtimeDetail: serverLabel)
    }

    return ModelHealthRefresh(
      modelHealth: RuntimeSummaryMapper.modelHealthSummary(from: runtimeModel),
      runtimeDetail: serverLabel.map { "\($0) | \(runtimeModel.displayName)" }
    )
  }

  static func refreshRuntimeReadiness(
    using runtimeBridge: RuntimeBridge
  ) async -> RuntimeReadinessSummary? {
    let readiness = try? await runtimeBridge.runtimeReadiness()

    return readiness.map { RuntimeSummaryMapper.readinessSummary(from: $0) }
  }
}
