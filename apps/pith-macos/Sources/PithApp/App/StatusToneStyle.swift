import AppKit
import SwiftUI

enum PithVisualStyle {
  static let windowBackground = Color(nsColor: .windowBackgroundColor)
  static let paneBackground = Color(nsColor: .textBackgroundColor)
  static let inspectorBackground = Color(nsColor: .controlBackgroundColor)
  static let panelBackground = Color(nsColor: .textBackgroundColor)
  static let selectedPanelBackground = Color.accentColor.opacity(0.095)
  static let panelBorder = Color(nsColor: .separatorColor).opacity(0.45)
  static let panelShadow = Color(nsColor: .shadowColor).opacity(0.04)
}

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
      .background(pillBackground)
      .overlay(
        Capsule()
          .stroke(pillBorder, lineWidth: 1)
      )
      .clipShape(Capsule())
  }

  private var pillBackground: Color {
    if tone == .neutral {
      return PithVisualStyle.panelBackground
    }

    return tone.color.opacity(0.12)
  }

  private var pillBorder: Color {
    if tone == .neutral {
      return PithVisualStyle.panelBorder
    }

    return tone.color.opacity(0.18)
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
      .shadow(color: PithVisualStyle.panelShadow, radius: 8, x: 0, y: 3)
  }

  private var backgroundColor: Color {
    if isSelected {
      return PithVisualStyle.selectedPanelBackground
    }
    if tone == .neutral {
      return PithVisualStyle.panelBackground
    }
    return tone.color.opacity(0.07)
  }

  private var borderColor: Color {
    if isSelected {
      return Color.accentColor.opacity(0.45)
    }
    if tone == .neutral {
      return PithVisualStyle.panelBorder
    }
    return tone.color.opacity(0.18)
  }
}

extension View {
  func softPanel(tone: StatusTone = .neutral, isSelected: Bool = false) -> some View {
    modifier(SoftPanel(tone: tone, isSelected: isSelected))
  }
}
