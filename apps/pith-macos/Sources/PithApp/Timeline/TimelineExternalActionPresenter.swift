import Foundation

struct TimelineExternalActionSummary: Hashable {
  let title: String
  let copyTitle: String
  let url: URL
  let copyValue: String?

  init(title: String, copyTitle: String, url: URL, copyValue: String? = nil) {
    self.title = title
    self.copyTitle = copyTitle
    self.url = url
    self.copyValue = copyValue
  }
}

struct TimelineProofSummary: Hashable {
  let title: String
  let detail: String
}

enum TimelineExternalActionPresenter {
  static func primaryAction(
    attributes: [String: String],
    workspaceRoot: String? = nil
  ) -> TimelineExternalActionSummary? {
    TimelineConnectorEvidencePresenter.primaryAction(attributes: attributes)
      ?? webSourceAction(attributes: attributes)
      ?? workspaceFileAction(attributes: attributes, workspaceRoot: workspaceRoot)
  }

  static func proofSummary(attributes: [String: String]) -> TimelineProofSummary? {
    TimelineConnectorEvidencePresenter.proofSummary(attributes: attributes)
  }

  private static func webSourceAction(
    attributes: [String: String]
  ) -> TimelineExternalActionSummary? {
    guard attributes["webSearchSourceMode"] != nil
      || attributes["sourceAttribution"] == "web_search"
    else {
      return nil
    }

    guard let url = firstSafeWebSourceURL(attributes["sourceUrls"]) else {
      return nil
    }

    return TimelineExternalActionSummary(
      title: "Open Web Source",
      copyTitle: "Copy Source Link",
      url: url
    )
  }

  private static func workspaceFileAction(
    attributes: [String: String],
    workspaceRoot: String?
  ) -> TimelineExternalActionSummary? {
    guard let workspaceRoot, !workspaceRoot.isEmpty else {
      return nil
    }

    let relativePath = firstNonEmpty([
      attributes["relativePath"],
      attributes["nextRelativePath"],
    ])
    guard let relativePath,
          let fileURL = safeWorkspaceFileURL(rootPath: workspaceRoot, relativePath: relativePath)
    else {
      return nil
    }

    let isDiff = attributes["tool"] == "generate_diff" || attributes["toolName"] == "generate_diff"
    let title = isDiff ? "Show Changed File" : "Show Source File"
    return TimelineExternalActionSummary(
      title: title,
      copyTitle: "Copy File Path",
      url: fileURL,
      copyValue: fileURL.path
    )
  }

  private static func firstSafeWebSourceURL(_ value: String?) -> URL? {
    guard let value else {
      return nil
    }

    let separators = CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: ",;"))
    return value
      .components(separatedBy: separators)
      .compactMap { component in
        safeWebURL(component.trimmingCharacters(in: .whitespacesAndNewlines))
      }
      .first
  }

  private static func safeWebURL(_ value: String?) -> URL? {
    guard let value,
          let url = URL(string: value),
          url.scheme == "https",
          url.host?.isEmpty == false
    else {
      return nil
    }

    return url
  }

  private static func safeWorkspaceFileURL(rootPath: String, relativePath: String) -> URL? {
    let trimmed = relativePath.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty,
          !trimmed.hasPrefix("/"),
          !trimmed.hasPrefix("~")
    else {
      return nil
    }

    let parts = trimmed
      .split(separator: "/", omittingEmptySubsequences: false)
      .map(String.init)
    guard !parts.contains(".."), !parts.contains(where: { $0 == "." || $0.isEmpty }) else {
      return nil
    }

    return parts.reduce(URL(fileURLWithPath: rootPath)) { url, part in
      url.appendingPathComponent(part)
    }
  }

  private static func firstNonEmpty(_ values: [String?]) -> String? {
    values.compactMap { value in
      let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines)
      return trimmed?.isEmpty == false ? trimmed : nil
    }.first
  }
}
