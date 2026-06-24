enum TimelineConnectorReceiptPresenter {
  static func hasReceipt(attributes: [String: String]) -> Bool {
    receiptKeys.contains { key in attributes[key] != nil }
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
    notionPageAction(attributes: attributes) ?? genericReceiptAction(attributes: attributes)
  }

  static func receiptSummary(attributes: [String: String]) -> TimelineReceiptSummary? {
    notionPageReceiptSummary(attributes: attributes) ?? genericReceiptSummary(attributes: attributes)
  }

  private static let receiptKeys = [
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
    guard TimelineReceiptText.firstAttribute(attributes, keys: [
      "connectorId",
      "connectorIds",
      "pluginRunnerConnectorId",
      "pluginRunnerConnectorIds",
    ]) != nil else {
      return nil
    }

    let authorization = TimelineReceiptText.firstAttribute(attributes, keys: [
      "authorizationSummary",
      "connectorAuthorizationSummary",
    ]) ?? authorizationSummary(attributes: attributes)
    return "Connection: \(serviceName(attributes: attributes)). Authorization: \(authorization)."
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
    let readableStatus = TimelineReceiptText.readableStatus(status)
    let readableStage = TimelineReceiptText.readableStage(stage)
    lines.append("\(name): \(readableStatus) in \(readableStage).")

    if let target = attributes["connectorWorkflowTarget"] {
      lines.append("Target: \(target)")
    }
    if let receiptKind = attributes["connectorWorkflowProof"] {
      lines.append("Confirmation: \(TimelineReceiptText.readableReceiptKind(receiptKind))")
    }
    if let recovery = attributes["connectorWorkflowRecovery"] {
      lines.append("Recovery: \(TimelineReceiptText.readableRecovery(recovery))")
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
    let targetService = serviceName(attributes: attributes)
    let targetTool = TimelineReceiptText.readableToolLabel(attributes["targetTool"])
    lines.append(
      "External action: \(TimelineReceiptText.readableStatus(status)). "
        + "Sent: \(TimelineReceiptText.yesNo(sent)). "
        + "Stage: \(TimelineReceiptText.readableStage(stage)). "
        + "Service: \(targetService) via \(targetTool)."
    )

    appendRemoteWriteContinuation(attributes: attributes, to: &lines)
    appendRemoteReceiptSummary(attributes: attributes, to: &lines)
    appendRetrySummary(attributes: attributes, to: &lines)
  }

  private static func authorizationSummary(attributes: [String: String]) -> String {
    let authStatus = TimelineReceiptText.firstAttribute(attributes, keys: [
      "authStatus",
      "connectorAuthStatus",
    ]) ?? "ready"
    let credentialPresent = TimelineReceiptText.boolAttribute(attributes, keys: [
      "credentialPresent",
      "connectorCredentialPresent",
    ])
    let credentialSecretPresent = TimelineReceiptText.boolAttribute(attributes, keys: [
      "credentialSecretPresent",
      "connectorCredentialSecretPresent",
    ])
    let localCredentialBinding = TimelineReceiptText.firstAttribute(attributes, keys: [
      "credentialStore",
      "connectorCredentialStores",
      "pluginRunnerConnectorStores",
      "credentialBinding",
      "connectorSecretBindings",
      "pluginRunnerSecretBindings",
    ])

    if authStatus == "needsAuth" {
      return "needs sign in"
    }

    if credentialPresent == true {
      return credentialSecretPresent == false ? "needs sign in" : "saved locally"
    }

    if localCredentialBinding != nil {
      return "available locally"
    }

    if let authRequired = TimelineReceiptText.boolAttribute(attributes, keys: [
      "authRequired",
      "connectorAuthRequired",
    ]) {
      return PluginStatusDisplay.authorizationStatus(
        authStatus,
        authRequired: authRequired,
        credentialPresent: false,
        credentialSecretPresent: false
      )
    }

    return "not saved"
  }

  private static func appendRemoteWriteContinuation(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let approvalRequired = attributes["remoteWriteRequiresApproval"] {
      let approval = TimelineReceiptText.yesNo(approvalRequired)
      lines.append("Approval before external write: \(approval)")
    }
    if let sourceArtifact = attributes["sourceArtifact"] {
      lines.append("Source file: \(sourceArtifact)")
    }
    if let nextCommandID = attributes["nextCommandId"] {
      let label = attributes["nextCommandLabel"]
        ?? TimelineReceiptText.readableCommandLabel(nextCommandID)
      lines.append("Next step: \(label)")
    }
    if let nextCommandInput = attributes["nextCommandInput"] {
      lines.append("Next input: \(nextCommandInput)")
    }
    if let nextCommandInputTemplate = attributes["nextCommandInputTemplate"] {
      lines.append("Draft input: \(nextCommandInputTemplate)")
    }
    if let nextCommandInputHint = attributes["nextCommandInputHint"] {
      lines.append("Input hint: \(nextCommandInputHint)")
    }
  }

  private static func appendRemoteReceiptSummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if attributes["remoteProofStatus"] != nil || attributes["remoteProofKind"] != nil {
      let receiptStatus = attributes["remoteProofStatus"] ?? "unknown"
      let receiptKind = attributes["remoteProofKind"] ?? "unknown confirmation"
      let readableStatus = TimelineReceiptText.readableStatus(receiptStatus)
      let readableKind = TimelineReceiptText.readableReceiptKind(receiptKind)
      lines.append(
        "External confirmation: \(readableStatus) (\(readableKind))"
      )
    }
    if let receiptTitle = attributes["remoteProofTitle"] {
      lines.append("Confirmation title: \(receiptTitle)")
    }
    if let receiptID = attributes["remoteProofId"] {
      lines.append("Confirmation: \(receiptID)")
    }
    if let receiptURL = attributes["remoteProofUrl"] {
      lines.append("Confirmation link: \(receiptURL)")
    }

    appendNotionReceiptSummary(attributes: attributes, to: &lines)
    appendTruncationSummary(attributes: attributes, to: &lines)
  }

  private static func appendNotionReceiptSummary(
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
      lines.append(titleTruncated == "true" ? "Title was shortened" : "Title was complete")
    }
    if let bodyTruncated = attributes["bodyTruncated"] {
      lines.append(bodyTruncated == "true" ? "Body was shortened" : "Body was complete")
    }
  }

  private static func appendRetrySummary(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let failureReason = attributes["publishFailureReason"] {
      lines.append("Publish issue: \(TimelineReceiptText.readableFailureReason(failureReason))")
    }
    if let retryCommandID = attributes["retryCommandId"] {
      lines.append("Retry step: \(TimelineReceiptText.readableCommandLabel(retryCommandID))")
    }
    if let retryInputEditable = attributes["retryInputEditable"] {
      lines.append("Retry input editable: \(TimelineReceiptText.yesNo(retryInputEditable))")
    }
    if let retryInputHint = attributes["retryInputHint"] {
      lines.append("Retry hint: \(retryInputHint)")
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
          let url = TimelineReceiptText.safeWebURL(attributes["notionPageUrl"])
    else {
      return nil
    }

    return TimelineExternalActionSummary(
      title: "Open Notion Page",
      copyTitle: "Copy Link",
      url: url
    )
  }

  private static func genericReceiptAction(
    attributes: [String: String]
  ) -> TimelineExternalActionSummary? {
    guard attributes["remoteProofStatus"] == "success",
          let url = TimelineReceiptText.safeWebURL(attributes["remoteProofUrl"])
    else {
      return nil
    }

    let service = serviceName(attributes: attributes)
    return TimelineExternalActionSummary(
      title: attributes["remoteProofActionTitle"] ?? "Open \(service) Confirmation",
      copyTitle: attributes["remoteProofCopyTitle"] ?? "Copy Link",
      url: url
    )
  }

  private static func notionPageReceiptSummary(
    attributes: [String: String]
  ) -> TimelineReceiptSummary? {
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
      parts.append(bodyTruncated == "true" ? "Body shortened" : "Body complete")
    }
    if let blockCount = attributes["notionBlockCount"], !blockCount.isEmpty {
      parts.append("Blocks: \(blockCount)")
    }

    return TimelineReceiptSummary(
      title: "Notion page created",
      detail: parts.joined(separator: " | ")
    )
  }

  private static func genericReceiptSummary(
    attributes: [String: String]
  ) -> TimelineReceiptSummary? {
    guard attributes["remoteProofStatus"] == "success" else {
      return nil
    }

    var parts: [String] = []
    if let receiptID = attributes["remoteProofId"], !receiptID.isEmpty {
      parts.append("Confirmation: \(receiptID)")
    }
    if let receiptKind = attributes["remoteProofKind"], !receiptKind.isEmpty {
      parts.append(TimelineReceiptText.readableReceiptKind(receiptKind))
    }
    if let receiptURL = attributes["remoteProofUrl"], !receiptURL.isEmpty {
      parts.append("Link: \(receiptURL)")
    }

    guard !parts.isEmpty else {
      return nil
    }

    return TimelineReceiptSummary(
      title: attributes["remoteProofTitle"] ?? "\(serviceName(attributes: attributes)) confirmation recorded",
      detail: parts.joined(separator: " | ")
    )
  }

  private static func serviceName(attributes: [String: String]) -> String {
    let raw = TimelineReceiptText.firstAttribute(attributes, keys: [
      "targetService",
      "connectorWorkflowService",
      "connectorService",
    ]) ?? "Remote"
    return TimelineReceiptText.readableTokenLabel(raw)
  }
}
