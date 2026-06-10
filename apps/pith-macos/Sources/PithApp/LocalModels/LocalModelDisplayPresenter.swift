import Foundation

enum LocalModelDisplayPresenter {
  static func cleanDisplayName(_ displayName: String) -> String {
    displayName
      .replacingOccurrences(of: " Q4_K_M", with: "")
      .replacingOccurrences(of: " Q4_K_S", with: "")
      .replacingOccurrences(of: " Q8_0", with: "")
  }

  static func actionName(_ model: LocalModelSummary) -> String {
    cleanDisplayName(model.displayName)
  }

  static func setupTitle(_ model: LocalModelSummary) -> String {
    actionName(model)
  }

  static func setupMetadata(_ model: LocalModelSummary) -> String {
    "Size \(LocalModelByteFormatter.string(model.sizeBytes)). License \(model.license). Active context \(model.contextSize) tokens."
  }

  static func statusMetadata(status: String, sizeBytes: Int64, license: String) -> String {
    "\(status). Size \(LocalModelByteFormatter.string(sizeBytes)). License \(license)."
  }

  static func firstUseFit(_ model: LocalModelSummary, defaultModelID: String) -> String {
    if model.id == defaultModelID {
      return "Default path: fastest first setup and the lightest local loop."
    }

    if model.tags.contains("recommended") {
      return "Recommended alternative: still tiny, stronger for tools and code."
    }

    return "Optional local model for longer or heavier cowork tasks."
  }
}

enum LocalModelByteFormatter {
  static func string(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}
