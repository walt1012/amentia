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
    TimelineConnectorEvidencePresenter.primaryAction(attributes: attributes)
  }

  static func proofSummary(attributes: [String: String]) -> TimelineProofSummary? {
    TimelineConnectorEvidencePresenter.proofSummary(attributes: attributes)
  }
}
