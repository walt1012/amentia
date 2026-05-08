import Foundation

enum LocalModelDisplayPresenter {
  static func sourceName(_ sourceURL: URL) -> String {
    sourceName(host: sourceURL.host)
  }

  static func sourceName(_ sourceURLString: String) -> String {
    sourceName(host: URL(string: sourceURLString)?.host)
  }

  static func firstUseMetadata(_ model: LocalModelSummary) -> String {
    if model.downloaded {
      return "Ready locally. Use this model without another download."
    }

    return "Source: \(sourceName(model.homepage))"
  }

  static func firstUseFit(_ model: LocalModelSummary, defaultModelID: String) -> String {
    if model.id == defaultModelID {
      return "Default path: fastest first setup and the lightest local loop."
    }

    if model.tags.contains("recommended") {
      return "Recommended alternative: still tiny, stronger for tools, code, and memory-assisted context."
    }

    return "Catalog candidate: lightweight, local, and verified before activation."
  }

  private static func sourceName(host: String?) -> String {
    guard let host else {
      return "open-source catalog"
    }

    if host == "huggingface.co" {
      return "Hugging Face"
    }

    return host
  }
}
