import Foundation

enum LocalModelDisplayPresenter {
  static func firstUseFit(_ model: LocalModelSummary, defaultModelID: String) -> String {
    if model.id == defaultModelID {
      return "Default path: fastest first setup and the lightest local loop."
    }

    if model.tags.contains("recommended") {
      return "Recommended alternative: still tiny, stronger for tools, code, and memory-assisted context."
    }

    return "Catalog candidate: lightweight, local, and verified before activation."
  }
}

enum LocalModelByteFormatter {
  static func string(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}
