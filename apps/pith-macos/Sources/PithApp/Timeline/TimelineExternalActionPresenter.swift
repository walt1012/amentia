import Foundation

struct TimelineExternalActionSummary: Hashable {
  let title: String
  let copyTitle: String
  let url: URL
}

struct TimelineProofSummary: Hashable {
  let title: String
  let detail: String
}

enum TimelineExternalActionPresenter {
  static func primaryAction(attributes: [String: String]) -> TimelineExternalActionSummary? {
    notionPageAction(attributes: attributes)
  }

  static func proofSummary(attributes: [String: String]) -> TimelineProofSummary? {
    notionPageProofSummary(attributes: attributes)
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
