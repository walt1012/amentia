import Foundation

struct TimelineEvidenceBadgeSummary: Hashable {
  let label: String
  let tone: StatusTone
}

enum TimelineEvidenceBadgePresenter {
  static func badges(attributes: [String: String]) -> [TimelineEvidenceBadgeSummary] {
    [webSearchBadge(attributes: attributes), remoteWriteBadge(attributes: attributes)]
      .compactMap { $0 }
  }

  private static func webSearchBadge(
    attributes: [String: String]
  ) -> TimelineEvidenceBadgeSummary? {
    guard attributes["webSearchSourceMode"] == "searchResultAttribution" else {
      return nil
    }

    if attributes["pageFetchPerformed"] == "true" {
      return TimelineEvidenceBadgeSummary(label: "Verified Sources", tone: .ready)
    }
    if attributes["sourceSnapshotAvailable"] == "true" {
      return TimelineEvidenceBadgeSummary(label: "Search Snapshot", tone: .active)
    }

    return TimelineEvidenceBadgeSummary(label: "Search Result Sources", tone: .active)
  }

  private static func remoteWriteBadge(
    attributes: [String: String]
  ) -> TimelineEvidenceBadgeSummary? {
    guard let status = attributes["remoteWriteStatus"] else {
      return nil
    }

    switch status {
    case "completed":
      return TimelineEvidenceBadgeSummary(label: "Remote Write Done", tone: .ready)
    case "notSent":
      return TimelineEvidenceBadgeSummary(label: "Remote Write Not Sent", tone: .warning)
    case "unconfirmed":
      return TimelineEvidenceBadgeSummary(label: "Remote Write Unconfirmed", tone: .warning)
    case "pending":
      return TimelineEvidenceBadgeSummary(label: "Remote Write Pending", tone: .active)
    default:
      return TimelineEvidenceBadgeSummary(label: "Remote Write Unknown", tone: .warning)
    }
  }
}
