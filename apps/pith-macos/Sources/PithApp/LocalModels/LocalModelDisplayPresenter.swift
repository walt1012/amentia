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

  static func setupFitSummary(_ model: LocalModelSummary, defaultModelID: String) -> String {
    if model.id == defaultModelID {
      return "Fastest first setup for getting Amentia running quickly."
    }

    if model.tags.contains("recommended") {
      return "Balanced tiny model for tools, code, and everyday cowork tasks."
    }

    if model.tags.contains("long-context") {
      return "Stronger small model for longer context and heavier project work."
    }

    return "Optional local model for specialized cowork tasks."
  }

  static func setupCapabilitySummary(_ model: LocalModelSummary) -> String {
    if model.tags.contains("long-context") {
      return "Better for larger files, longer sessions, and heavier cowork tasks."
    }

    if model.tags.contains("recommended") {
      return "Good everyday balance for project help, tools, and code review."
    }

    return "Best for fast first setup, simple edits, and lightweight cowork."
  }

  static func setupFootprintSummary(_ model: LocalModelSummary) -> String {
    "Download: \(LocalModelByteFormatter.string(model.sizeBytes)). License: \(model.license)."
  }

  static func statusMetadata(status: String, sizeBytes: Int64, license: String) -> String {
    "\(statusSummary(status)). About \(LocalModelByteFormatter.string(sizeBytes)). License: \(license)."
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

  private static func statusSummary(_ status: String) -> String {
    switch status {
    case "downloading":
      return "Downloading now"
    case "paused":
      return "Download paused"
    case "active":
      return "Ready and active"
    case "downloaded":
      return "Ready to use"
    case "verify before use":
      return "Found on this Mac, needs verification"
    default:
      return "Available to download"
    }
  }
}

enum LocalModelByteFormatter {
  static func string(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}
