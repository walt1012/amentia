import Foundation

struct LocalModelProbeRequest: Equatable {
  let modelID: String
}

final class LocalModelProbeCoordinator {
  private var pendingPostActivationRequest: LocalModelProbeRequest?

  func schedulePostActivationCheck(modelID: String) {
    pendingPostActivationRequest = LocalModelProbeRequest(modelID: modelID)
  }

  func cancelPendingPostActivationCheck() {
    pendingPostActivationRequest = nil
  }

  func consumePostActivationCheck(
    activeModelID: String?,
    canProbe: Bool
  ) -> LocalModelProbeRequest? {
    guard let request = pendingPostActivationRequest else {
      return nil
    }

    guard request.modelID == activeModelID else {
      pendingPostActivationRequest = nil
      return nil
    }

    guard canProbe else {
      return nil
    }

    pendingPostActivationRequest = nil
    return request
  }
}
