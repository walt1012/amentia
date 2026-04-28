import Foundation

struct TimelineSandboxBadgeSummary: Hashable {
  let label: String
  let tone: StatusTone
}

enum TimelineSandboxBadgePresenter {
  static func badge(attributes: [String: String]) -> TimelineSandboxBadgeSummary? {
    guard attributes["sandboxMode"] != nil else {
      return nil
    }

    if attributes["sandboxActive"] == "true" {
      let label = attributes["sandboxBackend"] == "macosSeatbelt"
        ? "Native Sandbox"
        : "Sandbox Active"
      return TimelineSandboxBadgeSummary(label: label, tone: .ready)
    }

    return TimelineSandboxBadgeSummary(label: "Sandbox Limited", tone: .warning)
  }
}
