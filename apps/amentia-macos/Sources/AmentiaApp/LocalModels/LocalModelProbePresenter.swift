import Foundation

struct LocalModelProbePresentation {
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let timelineKind: TimelineEntry.Kind
  let attributes: [String: String]
}

enum LocalModelProbePresenter {
  static func startedDetail() -> String {
    "Starting the active local model..."
  }

  static func presentation(
    for probe: RuntimeBridge.RuntimeModelProbe
  ) -> LocalModelProbePresentation {
    if probe.status == "ready" {
      return successPresentation(for: probe)
    }

    return failurePresentation(for: probe)
  }

  static func requestFailurePresentation(error: Error) -> LocalModelProbePresentation {
    let detail = error.localizedDescription.trimmingCharacters(in: .whitespacesAndNewlines)
    var attributes = [
      "status": "request_failed"
    ]
    if !detail.isEmpty {
      attributes["detail"] = detail
    }

    return LocalModelProbePresentation(
      runtimeDetail: recoveryDetail,
      timelineTitle: "Local Model Startup Failed",
      timelineBody:
        "Amentia could not start the selected local model. Restart Amentia to try starting it again.",
      timelineKind: .warning,
      attributes: attributes
    )
  }

  static func readinessFailureDetail() -> String {
    recoveryDetail
  }

  static func recoveryDetail(for failureDetail: String) -> String {
    let detail = failureDetail.trimmingCharacters(in: .whitespacesAndNewlines)

    if detail.isEmpty {
      return recoveryDetail
    }
    if detail.contains("Cowork is paused") {
      return detail
    }
    return "\(detail) \(recoveryDetail)"
  }

  private static func successPresentation(
    for probe: RuntimeBridge.RuntimeModelProbe
  ) -> LocalModelProbePresentation {
    var attributes = baseAttributes(for: probe)
    if let sample = probe.sample?.trimmingCharacters(in: .whitespacesAndNewlines),
       !sample.isEmpty
    {
      attributes["sample"] = sample
    }

    return LocalModelProbePresentation(
      runtimeDetail: "Local model started.",
      timelineTitle: "Local Model Started",
      timelineBody: "The active local model is ready for local cowork.",
      timelineKind: .system,
      attributes: attributes
    )
  }

  private static func failurePresentation(
    for probe: RuntimeBridge.RuntimeModelProbe
  ) -> LocalModelProbePresentation {
    var attributes = baseAttributes(for: probe)
    let detail = probe.detail.trimmingCharacters(in: .whitespacesAndNewlines)
    if !detail.isEmpty {
      attributes["detail"] = detail
    }

    return LocalModelProbePresentation(
      runtimeDetail: recoveryDetail,
      timelineTitle: "Local Model Startup Failed",
      timelineBody:
        "The selected local model did not answer during startup. Restart Amentia to try starting it again.",
      timelineKind: .warning,
      attributes: attributes
    )
  }

  private static let recoveryDetail =
    "Cowork is paused until the local model starts successfully. "
    + "Restart Amentia to try starting it again."

  private static func baseAttributes(
    for probe: RuntimeBridge.RuntimeModelProbe
  ) -> [String: String] {
    [
      "modelId": probe.modelID,
      "backend": probe.backend,
      "status": probe.status,
    ]
  }
}
