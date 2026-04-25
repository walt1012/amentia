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
      return "Best for: first setup and the fastest local loop."
    }

    if model.tags.contains("code") {
      return "Best for: small code generation and repair experiments."
    }

    if model.tags.contains("multilingual") {
      return "Best for: compact multilingual chat."
    }

    if model.tags.contains("english") {
      return "Best for: fast English assistant tests."
    }

    return "Best for: lightweight local experiments."
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
