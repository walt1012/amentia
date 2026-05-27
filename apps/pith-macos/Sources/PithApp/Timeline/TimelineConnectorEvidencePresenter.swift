import Foundation

enum TimelineConnectorEvidencePresenter {
  static func hasEvidence(attributes: [String: String]) -> Bool {
    evidenceKeys.contains { key in attributes[key] != nil }
  }

  static func summaryLines(attributes: [String: String]) -> [String] {
    var lines: [String] = []

    if let connectorSummary = connectorSummary(attributes: attributes) {
      lines.append(connectorSummary)
    }
    appendConnectorWorkflowSummary(attributes: attributes, to: &lines)
    appendRemoteWriteSummary(attributes: attributes, to: &lines)

    return lines
  }

  static func primaryAction(attributes: [String: String]) -> TimelineExternalActionSummary? {
    notionPageAction(attributes: attributes) ?? genericProofAction(attributes: attributes)
  }

  static func proofSummary(attributes: [String: String]) -> TimelineProofSummary? {
    notionPageProofSummary(attributes: attributes) ?? genericProofSummary(attributes: attributes)
  }

  private static let evidenceKeys = [
    "connectorId",
    "connectorIds",
    "connectorStatus",
    "connectorRepairHint",
    "connectorWorkflowId",
    "connectorWorkflowName",
    "connectorWorkflowService",
    "connectorWorkflowAction",
    "connectorWorkflowStage",
    "connectorWorkflowStatus",
    "connectorWorkflowTarget",
    "connectorWorkflowProof",
    "connectorWorkflowRecovery",
    "remoteWrite",
    "remoteWriteStage",
    "remoteWriteStatus",
    "remoteProofKind",
    "remoteProofStatus",
    "remoteProofUrl",
    "remoteProofTitle",
    "remoteProofId",
    "notionPageId",
    "notionBlockCount",
    "nextCommandId",
    "nextCommandInput",
    "nextCommandInputHint",
    "nextCommandInputTemplate",
    "nextCommandLabel",
    "retryCommandId",
    "retryInput",
    "retryInputEditable",
    "retryInputHint",
  ]

  private static func connectorSummary(attributes: [String: String]) -> String? {
    guard let connectorIDs = firstAttribute(attributes, keys: [
      "connectorId",
      "connectorIds",
      "pluginRunnerConnectorId",
      "pluginRunnerConnectorIds",
    ]) else {
      return nil
    }

    let services = firstAttribute(attributes, keys: [
      "connectorService",
      "connectorServices",
      "pluginRunnerConnectorServices",
    ]) ?? "unknown service"
    let stores = firstAttribute(attributes, keys: [
      "credentialStore",
      "connectorCredentialStores",
      "pluginRunnerConnectorStores",
    ]) ?? "unknown store"
    let providers = firstAttribute(attributes, keys: [
      "credentialProvider",
      "connectorCredentialProviders",
    ]) ?? "unknown provider"
    let bindings = firstAttribute(attributes, keys: [
      "credentialBinding",
      "connectorSecretBindings",
      "pluginRunnerSecretBindings",
    ]) ?? "unknown binding"
    return "Connectors: \(connectorIDs) | \(services) | \(stores) | \(providers) "
      + "| \(bindings)"
  }

  private static func appendConnectorWorkflowSummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    guard attributes["connectorWorkflowId"] != nil
      || attributes["connectorWorkflowStatus"] != nil
    else {
      return
    }

    let name = attributes["connectorWorkflowName"] ?? "Connector workflow"
    let status = attributes["connectorWorkflowStatus"] ?? "unknown"
    let stage = attributes["connectorWorkflowStage"] ?? "unknown stage"
    let service = attributes["connectorWorkflowService"] ?? "unknown service"
    let action = attributes["connectorWorkflowAction"] ?? "unknown action"
    lines.append("\(name): \(status) | stage \(stage) | \(service) \(action)")

    if let target = attributes["connectorWorkflowTarget"] {
      lines.append("Workflow target: \(target)")
    }
    if let proof = attributes["connectorWorkflowProof"] {
      lines.append("Workflow proof: \(proof)")
    }
    if let recovery = attributes["connectorWorkflowRecovery"] {
      lines.append("Workflow recovery: \(recovery)")
    }
  }

  private static func appendRemoteWriteSummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    guard attributes["remoteWrite"] != nil
      || attributes["remoteWriteStage"] != nil
      || attributes["remoteWriteStatus"] != nil
      || attributes["remoteProofStatus"] != nil
    else {
      return
    }

    let status = attributes["remoteWriteStatus"] ?? "unknown"
    let stage = attributes["remoteWriteStage"] ?? "unknown stage"
    let sent = attributes["remoteWrite"] ?? "unknown"
    let targetService = attributes["targetService"] ?? "unknown service"
    let targetTool = attributes["targetTool"] ?? "unknown tool"
    lines.append(
      "Remote write: \(status) | sent \(sent) | stage \(stage) | "
        + "\(targetService) via \(targetTool)"
    )

    appendRemoteWriteContinuation(attributes: attributes, to: &lines)
    appendRemoteProofSummary(attributes: attributes, to: &lines)
    appendRetrySummary(attributes: attributes, to: &lines)
  }

  private static func appendRemoteWriteContinuation(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let approvalRequired = attributes["remoteWriteRequiresApproval"] {
      lines.append("Remote approval required: \(approvalRequired)")
    }
    if let sourceArtifact = attributes["sourceArtifact"] {
      lines.append("Remote write source: \(sourceArtifact)")
    }
    if let nextCommandID = attributes["nextCommandId"] {
      let label = attributes["nextCommandLabel"] ?? "Continue"
      lines.append("Next command: \(label) | \(nextCommandID)")
    }
    if let nextCommandInput = attributes["nextCommandInput"] {
      lines.append("Next input: \(nextCommandInput)")
    }
    if let nextCommandInputTemplate = attributes["nextCommandInputTemplate"] {
      lines.append("Next input template: \(nextCommandInputTemplate)")
    }
    if let nextCommandInputHint = attributes["nextCommandInputHint"] {
      lines.append("Next input hint: \(nextCommandInputHint)")
    }
  }

  private static func appendRemoteProofSummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if attributes["remoteProofStatus"] != nil || attributes["remoteProofKind"] != nil {
      let proofStatus = attributes["remoteProofStatus"] ?? "unknown"
      let proofKind = attributes["remoteProofKind"] ?? "unknown proof"
      lines.append("Remote proof: \(proofStatus) | \(proofKind)")
    }
    if let proofTitle = attributes["remoteProofTitle"] {
      lines.append("Remote proof title: \(proofTitle)")
    }
    if let proofID = attributes["remoteProofId"] {
      lines.append("Remote proof ID: \(proofID)")
    }
    if let proofURL = attributes["remoteProofUrl"] {
      lines.append("Remote proof URL: \(proofURL)")
    }

    appendNotionProofSummary(attributes: attributes, to: &lines)
    appendTruncationSummary(attributes: attributes, to: &lines)
  }

  private static func appendNotionProofSummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let pageID = attributes["notionPageId"] {
      let pageURL = attributes["notionPageUrl"] ?? "no URL"
      lines.append("Notion page: \(pageID) | \(pageURL)")
    }
    if let parentPageID = attributes["notionParentPageId"] {
      lines.append("Notion parent: \(parentPageID)")
    }
    if let blockCount = attributes["notionBlockCount"] {
      lines.append("Notion blocks: \(blockCount)")
    }
  }

  private static func appendTruncationSummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let titleTruncated = attributes["titleTruncated"] {
      lines.append("Title truncated: \(titleTruncated)")
    }
    if let bodyTruncated = attributes["bodyTruncated"] {
      lines.append("Body truncated: \(bodyTruncated)")
    }
  }

  private static func appendRetrySummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let failureReason = attributes["publishFailureReason"] {
      lines.append("Publish failure: \(failureReason)")
    }
    if let retryCommandID = attributes["retryCommandId"] {
      lines.append("Retry command: \(retryCommandID)")
    }
    if let retryInputEditable = attributes["retryInputEditable"] {
      lines.append("Retry input editable: \(retryInputEditable)")
    }
    if let retryInputHint = attributes["retryInputHint"] {
      lines.append("Retry input hint: \(retryInputHint)")
    }
    if let retryInput = attributes["retryInput"] {
      lines.append("Retry input: \(retryInput)")
    }
  }

  private static func notionPageAction(
    attributes: [String: String]
  ) -> TimelineExternalActionSummary? {
    guard attributes["remoteProofStatus"] == "success",
          attributes["remoteProofKind"] == "notionApiResponse",
          let pageID = attributes["notionPageId"],
          !pageID.isEmpty,
          let url = safeWebURL(attributes["notionPageUrl"])
    else {
      return nil
    }

    return TimelineExternalActionSummary(
      title: "Open Notion Page",
      copyTitle: "Copy Link",
      url: url
    )
  }

  private static func genericProofAction(
    attributes: [String: String]
  ) -> TimelineExternalActionSummary? {
    guard attributes["remoteProofStatus"] == "success",
          let url = safeWebURL(attributes["remoteProofUrl"])
    else {
      return nil
    }

    let service = serviceName(attributes: attributes)
    return TimelineExternalActionSummary(
      title: attributes["remoteProofActionTitle"] ?? "Open \(service) Proof",
      copyTitle: attributes["remoteProofCopyTitle"] ?? "Copy Link",
      url: url
    )
  }

  private static func notionPageProofSummary(
    attributes: [String: String]
  ) -> TimelineProofSummary? {
    guard attributes["remoteProofStatus"] == "success",
          attributes["remoteProofKind"] == "notionApiResponse",
          let pageID = attributes["notionPageId"],
          !pageID.isEmpty
    else {
      return nil
    }

    var parts = ["Page: \(pageID)"]
    if let parentPageID = attributes["notionParentPageId"], !parentPageID.isEmpty {
      parts.append("Parent: \(parentPageID)")
    }
    if let bodyTruncated = attributes["bodyTruncated"] {
      parts.append(bodyTruncated == "true" ? "Body truncated" : "Body complete")
    }
    if let blockCount = attributes["notionBlockCount"], !blockCount.isEmpty {
      parts.append("Blocks: \(blockCount)")
    }

    return TimelineProofSummary(
      title: "Notion page created",
      detail: parts.joined(separator: " | ")
    )
  }

  private static func genericProofSummary(
    attributes: [String: String]
  ) -> TimelineProofSummary? {
    guard attributes["remoteProofStatus"] == "success" else {
      return nil
    }

    var parts: [String] = []
    if let proofID = attributes["remoteProofId"], !proofID.isEmpty {
      parts.append("ID: \(proofID)")
    }
    if let proofKind = attributes["remoteProofKind"], !proofKind.isEmpty {
      parts.append("Proof: \(proofKind)")
    }
    if let proofURL = attributes["remoteProofUrl"], !proofURL.isEmpty {
      parts.append("URL: \(proofURL)")
    }

    guard !parts.isEmpty else {
      return nil
    }

    return TimelineProofSummary(
      title: attributes["remoteProofTitle"] ?? "\(serviceName(attributes: attributes)) proof confirmed",
      detail: parts.joined(separator: " | ")
    )
  }

  private static func serviceName(attributes: [String: String]) -> String {
    let raw = firstAttribute(attributes, keys: [
      "targetService",
      "connectorWorkflowService",
      "connectorService",
    ]) ?? "Remote"
    return raw
      .split { character in character == "-" || character == "_" || character == " " }
      .map { word in
        let lowercased = word.lowercased()
        return lowercased.prefix(1).uppercased() + String(lowercased.dropFirst())
      }
      .joined(separator: " ")
  }

  private static func firstAttribute(
    _ attributes: [String: String],
    keys: [String]
  ) -> String? {
    keys
      .compactMap { key in attributes[key]?.trimmingCharacters(in: .whitespacesAndNewlines) }
      .first { !$0.isEmpty }
  }

  private static func safeWebURL(_ value: String?) -> URL? {
    guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !value.isEmpty,
          let url = URL(string: value),
          let scheme = url.scheme?.lowercased(),
          scheme == "https",
          let host = url.host,
          !host.isEmpty
    else {
      return nil
    }

    return url
  }
}
