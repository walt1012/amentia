import Foundation

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
    guard firstAttribute(attributes, keys: [
      "connectorId",
      "connectorIds",
      "pluginRunnerConnectorId",
      "pluginRunnerConnectorIds",
    ]) != nil else {
      return nil
    }

    let authorization = firstAttribute(attributes, keys: [
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
    lines.append("\(name): \(readableStatus(status)) in \(readableStage(stage)).")

    if let target = attributes["connectorWorkflowTarget"] {
      lines.append("Target: \(target)")
    }
    if let receiptKind = attributes["connectorWorkflowProof"] {
      lines.append("Confirmation: \(readableReceiptKind(receiptKind))")
    }
    if let recovery = attributes["connectorWorkflowRecovery"] {
      lines.append("Recovery: \(readableRecovery(recovery))")
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
    let targetTool = readableToolLabel(attributes["targetTool"])
    lines.append(
      "External action: \(readableStatus(status)). Sent: \(yesNo(sent)). "
        + "Stage: \(readableStage(stage)). Service: \(targetService) via \(targetTool)."
    )

    appendRemoteWriteContinuation(attributes: attributes, to: &lines)
    appendRemoteReceiptSummary(attributes: attributes, to: &lines)
    appendRetrySummary(attributes: attributes, to: &lines)
  }

  private static func authorizationSummary(attributes: [String: String]) -> String {
    let authStatus = firstAttribute(attributes, keys: [
      "authStatus",
      "connectorAuthStatus",
    ]) ?? "ready"
    let credentialPresent = boolAttribute(attributes, keys: [
      "credentialPresent",
      "connectorCredentialPresent",
    ])
    let credentialSecretPresent = boolAttribute(attributes, keys: [
      "credentialSecretPresent",
      "connectorCredentialSecretPresent",
    ])
    let localCredentialBinding = firstAttribute(attributes, keys: [
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

    if let authRequired = boolAttribute(attributes, keys: [
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

  private static func boolAttribute(
    _ attributes: [String: String],
    keys: [String]
  ) -> Bool? {
    guard let value = firstAttribute(attributes, keys: keys)?.lowercased() else {
      return nil
    }
    switch value {
    case "true", "yes", "1":
      return true
    case "false", "no", "0":
      return false
    default:
      return nil
    }
  }

  private static func appendRemoteWriteContinuation(
    attributes: [String: String],
    to lines: inout [String]
  ) {
    if let approvalRequired = attributes["remoteWriteRequiresApproval"] {
      lines.append("Approval before external write: \(yesNo(approvalRequired))")
    }
    if let sourceArtifact = attributes["sourceArtifact"] {
      lines.append("Source file: \(sourceArtifact)")
    }
    if let nextCommandID = attributes["nextCommandId"] {
      let label = attributes["nextCommandLabel"] ?? readableCommandLabel(nextCommandID)
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
      lines.append(
        "External confirmation: \(readableStatus(receiptStatus)) (\(readableReceiptKind(receiptKind)))"
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
      lines.append("Publish issue: \(readableFailureReason(failureReason))")
    }
    if let retryCommandID = attributes["retryCommandId"] {
      lines.append("Retry step: \(readableCommandLabel(retryCommandID))")
    }
    if let retryInputEditable = attributes["retryInputEditable"] {
      lines.append("Retry input editable: \(yesNo(retryInputEditable))")
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

  private static func genericReceiptAction(
    attributes: [String: String]
  ) -> TimelineExternalActionSummary? {
    guard attributes["remoteProofStatus"] == "success",
          let url = safeWebURL(attributes["remoteProofUrl"])
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
      parts.append(readableReceiptKind(receiptKind))
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

  private static func readableCommandLabel(_ value: String) -> String {
    let tail = value.components(separatedBy: "::").last ?? value
    let words = tail
      .split { character in
        character == "." || character == "_" || character == "-" || character == ":"
      }
      .map { word in
        let lowercased = word.lowercased()
        return lowercased.prefix(1).uppercased() + String(lowercased.dropFirst())
      }

    return words.isEmpty ? "Continue" : words.joined(separator: " ")
  }

  private static func readableStatus(_ value: String) -> String {
    switch value {
    case "success", "completed":
      return "completed"
    case "notRequested", "notSent":
      return "not sent yet"
    case "prepared":
      return "prepared"
    case "inspected":
      return "ready for review"
    case "retryNeeded":
      return "needs retry"
    default:
      return readableTokenLabel(value)
    }
  }

  private static func readableStage(_ value: String) -> String {
    switch value {
    case "draftPrepared":
      return "draft prepared"
    case "inspectBeforeWrite":
      return "review before write"
    case "blockedBeforeWrite":
      return "blocked before external write"
    case "failedBeforeProof":
      return "finished without trusted confirmation"
    case "completed":
      return "completed"
    default:
      return readableTokenLabel(value)
    }
  }

  private static func readableReceiptKind(_ value: String) -> String {
    if let serviceLabel = PluginConnectorServiceGuide.receiptKindLabel(value) {
      return serviceLabel
    }

    switch value {
    case "localDraft":
      return "local draft"
    case "inspection":
      return "review completed"
    case "notRequested":
      return "not requested"
    case "missing":
      return "missing"
    case "messageApiResponse":
      return "message confirmation"
    default:
      return readableTokenLabel(value)
    }
  }

  private static func readableFailureReason(_ value: String) -> String {
    switch value {
    case "invalidParentPageId":
      return "the parent page ID needs review"
    case "missingParentPageId":
      return "a parent page is required"
    case "missingRemoteProof":
      return "Amentia could not verify the external result"
    default:
      return readableTokenLabel(value)
    }
  }

  private static func readableRecovery(_ value: String) -> String {
    switch value {
    case "retry":
      return "retry available"
    default:
      return readableTokenLabel(value)
    }
  }

  private static func readableToolLabel(_ value: String?) -> String {
    guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !value.isEmpty
    else {
      return "local connector"
    }

    return readableTokenLabel(value)
  }

  private static func readableTokenLabel(_ value: String) -> String {
    let normalized = value
      .replacingOccurrences(of: "::", with: " ")
      .replacingOccurrences(of: ".", with: " ")
      .replacingOccurrences(of: "_", with: " ")
      .replacingOccurrences(of: "-", with: " ")
    let spaced = normalized.reduce(into: "") { result, character in
      if isUppercaseLetter(character),
         let last = result.last,
         isLowercaseLetter(last) || isNumber(last) {
        result.append(" ")
      }
      result.append(character)
    }
    let words = spaced
      .split(separator: " ")
      .map { word in
        let lowercased = word.lowercased()
        return lowercased.prefix(1).uppercased() + String(lowercased.dropFirst())
      }

    return words.isEmpty ? value : words.joined(separator: " ")
  }

  private static func isUppercaseLetter(_ character: Character) -> Bool {
    character.unicodeScalars.allSatisfy { CharacterSet.uppercaseLetters.contains($0) }
  }

  private static func isLowercaseLetter(_ character: Character) -> Bool {
    character.unicodeScalars.allSatisfy { CharacterSet.lowercaseLetters.contains($0) }
  }

  private static func isNumber(_ character: Character) -> Bool {
    character.unicodeScalars.allSatisfy { CharacterSet.decimalDigits.contains($0) }
  }

  private static func yesNo(_ value: String) -> String {
    switch value {
    case "true":
      return "yes"
    case "false":
      return "no"
    default:
      return value
    }
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
