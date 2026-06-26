import Foundation

struct LocalModelProbePresentation {
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let timelineKind: TimelineEntry.Kind
  let attributes: [String: String]
}

enum LocalModelProbePresenter {
  static let blockedDetail =
    "Finish startup, model download, model selection, or active work before checking the model."

  static func startedDetail() -> String {
    "Checking the active local model..."
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
      timelineTitle: "Local Model Check Failed",
      timelineBody:
        "Amentia could not complete the local model check. Restart Amentia or re-download the selected model, then check again.",
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
      runtimeDetail: "Local model check passed.",
      timelineTitle: "Local Model Checked",
      timelineBody: "The active local model answered a short local prompt.",
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
      timelineTitle: "Local Model Check Failed",
      timelineBody:
        "The selected local model did not answer the check prompt. Restart Amentia or re-download the model, then check again.",
      timelineKind: .warning,
      attributes: attributes
    )
  }

  private static let recoveryDetail =
    "Cowork is paused until the local model check passes. "
    + "Restart Amentia or re-download the selected model, then check again."

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
