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

struct StatusPill: View {
  let label: String
  let tone: StatusTone

  var body: some View {
    Text(label)
      .font(.caption.weight(.medium))
      .foregroundColor(tone.color)
      .padding(.horizontal, 8)
      .padding(.vertical, 4)
      .background(tone.color.opacity(0.12))
      .clipShape(Capsule())
  }
}
