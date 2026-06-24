import Foundation

enum TimelineReceiptText {
  static func readableCommandLabel(_ value: String) -> String {
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

  static func readableStatus(_ value: String) -> String {
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

  static func readableStage(_ value: String) -> String {
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

  static func readableReceiptKind(_ value: String) -> String {
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

  static func readableFailureReason(_ value: String) -> String {
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

  static func readableRecovery(_ value: String) -> String {
    switch value {
    case "retry":
      return "retry available"
    default:
      return readableTokenLabel(value)
    }
  }

  static func readableToolLabel(_ value: String?) -> String {
    guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines),
          !value.isEmpty
    else {
      return "local connector"
    }

    return readableTokenLabel(value)
  }

  static func readableTokenLabel(_ value: String) -> String {
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

  static func yesNo(_ value: String) -> String {
    switch value {
    case "true":
      return "yes"
    case "false":
      return "no"
    default:
      return value
    }
  }

  static func firstAttribute(
    _ attributes: [String: String],
    keys: [String]
  ) -> String? {
    keys
      .compactMap { key in attributes[key]?.trimmingCharacters(in: .whitespacesAndNewlines) }
      .first { !$0.isEmpty }
  }

  static func boolAttribute(
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

  static func safeWebURL(_ value: String?) -> URL? {
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

  private static func isUppercaseLetter(_ character: Character) -> Bool {
    character.unicodeScalars.allSatisfy { CharacterSet.uppercaseLetters.contains($0) }
  }

  private static func isLowercaseLetter(_ character: Character) -> Bool {
    character.unicodeScalars.allSatisfy { CharacterSet.lowercaseLetters.contains($0) }
  }

  private static func isNumber(_ character: Character) -> Bool {
    character.unicodeScalars.allSatisfy { CharacterSet.decimalDigits.contains($0) }
  }
}
