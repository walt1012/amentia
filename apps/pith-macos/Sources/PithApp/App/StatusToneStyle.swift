import SwiftUI

enum StatusTone: String, Hashable {
  case neutral
  case ready
  case active
  case warning
  case danger
}

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
      .lineLimit(1)
      .padding(.horizontal, 8)
      .padding(.vertical, 4)
      .background(tone.color.opacity(0.12))
      .clipShape(Capsule())
  }
}

struct SoftPanel: ViewModifier {
  var tone: StatusTone = .neutral
  var isSelected = false

  func body(content: Content) -> some View {
    content
      .padding(12)
      .background(
        RoundedRectangle(cornerRadius: 14, style: .continuous)
          .fill(backgroundColor)
      )
      .overlay(
        RoundedRectangle(cornerRadius: 14, style: .continuous)
          .stroke(borderColor, lineWidth: 1)
      )
  }

  private var backgroundColor: Color {
    if isSelected {
      return Color.accentColor.opacity(0.10)
    }
    if tone == .neutral {
      return Color.secondary.opacity(0.06)
    }
    return tone.color.opacity(0.08)
  }

  private var borderColor: Color {
    if isSelected {
      return Color.accentColor.opacity(0.45)
    }
    if tone == .neutral {
      return Color.secondary.opacity(0.12)
    }
    return tone.color.opacity(0.18)
  }
}

extension View {
  func softPanel(tone: StatusTone = .neutral, isSelected: Bool = false) -> some View {
    modifier(SoftPanel(tone: tone, isSelected: isSelected))
  }
}
