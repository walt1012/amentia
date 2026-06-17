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
    LocalModelProbePresentation(
      runtimeDetail: "Local model check failed: \(error.localizedDescription)",
      timelineTitle: "Local Model Check Failed",
      timelineBody: error.localizedDescription,
      timelineKind: .warning,
      attributes: [
        "status": "request_failed"
      ]
    )
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
    LocalModelProbePresentation(
      runtimeDetail:
        "Local model check failed. Re-download the model or restart Amentia, then check again.",
      timelineTitle: "Local Model Check Failed",
      timelineBody: probe.detail,
      timelineKind: .warning,
      attributes: baseAttributes(for: probe)
    )
  }

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
