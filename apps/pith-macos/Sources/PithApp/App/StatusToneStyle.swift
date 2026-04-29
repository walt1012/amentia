import SwiftUI

extension StatusTone {
  var color: Color {
    switch self {
    case .neutral:
      return .secondary
    case .ready:
      return .green
    case .active:
      return .blue
    case .warning:
      return .orange
    case .danger:
      return .red
    }
  }
}
