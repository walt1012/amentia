import SwiftUI

struct TimelineCard: View {
  let entry: TimelineEntry
  let isSelected: Bool
  let proofSummary: TimelineProofSummary?
  let approvalOutcomeSummary: TimelineApprovalOutcomeSummary?
  let externalActionTitle: String?
  let externalCopyActionTitle: String?
  let showsApprovalActions: Bool
  let showsPluginEnableAction: Bool
  let showsPluginAuthorizeAction: Bool
  let showsPluginInputAction: Bool
  let showsPluginRetryAction: Bool
  let showsPluginFollowUpAction: Bool
  let showsPluginGuidanceDisableAction: Bool
  let showsPluginSourceAction: Bool
  let showsPluginRefreshAction: Bool
  let localExecutionRecoveryTitle: String?
  let onSelect: () -> Void
  let onApprove: () -> Void
  let onDeny: () -> Void
  let onEnablePlugin: () -> Void
  let onAuthorizePluginConnector: () -> Void
  let onRunPluginCommandWithInput: () -> Void
  let onRetry: () -> Void
  let onRunPluginFollowUp: () -> Void
  let onDisablePluginGuidance: () -> Void
  let onRevealPluginSource: () -> Void
  let onRefreshPlugins: () -> Void
  let onRecoverLocalExecution: () -> Void
  let onOpenExternalAction: () -> Void
  let onCopyExternalAction: () -> Void
  @State private var isHovered = false

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        TimelineChip(label: kindLabel, color: kindColor, isProminent: true)

        Text(entry.title)
          .font(.headline.weight(.semibold))
          .lineLimit(2)

        Spacer(minLength: 12)

        if let streamingLabel {
          TimelineChip(label: streamingLabel, color: streamingColor, isProminent: true)
        }
      }

      badgeRow

      if let streamingProgressValue {
        ProgressView(value: streamingProgressValue)
          .progressViewStyle(.linear)
          .tint(streamingColor)
      }

      Text(displayBody)
        .font(bodyFont)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if let contextReceiptSummary {
        HStack(alignment: .top, spacing: 6) {
          Image(systemName: "checkmark.circle")
            .foregroundColor(.secondary)
          Text(contextReceiptSummary)
            .font(.caption)
            .foregroundColor(.secondary)
            .lineLimit(2)
        }
        .padding(.top, 2)
      }

      if let proofSummary {
        proofSummaryView(proofSummary)
      }

      if let approvalOutcomeSummary {
        approvalOutcomeView(approvalOutcomeSummary)
      }

      if showsActionRow {
        ScrollView(.horizontal, showsIndicators: false) {
          HStack(spacing: 10) {
            if showsApprovalActions {
              Button("Approve") {
                onApprove()
              }
              .buttonStyle(.borderedProminent)

              Button("Deny") {
                onDeny()
              }
              .buttonStyle(.bordered)
            }

            if showsPluginEnableAction {
              Button("Enable") {
                onEnablePlugin()
              }
              .buttonStyle(.borderedProminent)
            }

            if showsPluginAuthorizeAction {
              Button("Authorize Connection") {
                onAuthorizePluginConnector()
              }
              .buttonStyle(.borderedProminent)
            }

            if showsPluginInputAction {
              Button(pluginInputTitle) {
                onRunPluginCommandWithInput()
              }
              .buttonStyle(.bordered)
            }

            if showsPluginRetryAction {
              Button(pluginRetryTitle) {
                onRetry()
              }
              .buttonStyle(.bordered)
            }

            if showsPluginFollowUpAction {
              Button(pluginFollowUpTitle) {
                onRunPluginFollowUp()
              }
              .buttonStyle(.borderedProminent)
            }

            if showsPluginGuidanceDisableAction {
              Button("Disable Guidance") {
                onDisablePluginGuidance()
              }
              .buttonStyle(.bordered)
            }

            if showsPluginSourceAction {
              Button("Show Source") {
                onRevealPluginSource()
              }
              .buttonStyle(.bordered)
            }

            if showsPluginRefreshAction {
              Button("Refresh") {
                onRefreshPlugins()
              }
              .buttonStyle(.bordered)
            }

            if let localExecutionRecoveryTitle {
              Button(localExecutionRecoveryTitle) {
                onRecoverLocalExecution()
              }
              .buttonStyle(.borderedProminent)
            }

            if let externalActionTitle {
              Button(externalActionTitle) {
                onOpenExternalAction()
              }
              .buttonStyle(.borderedProminent)
            }

            if let externalCopyActionTitle {
              Button(externalCopyActionTitle) {
                onCopyExternalAction()
              }
              .buttonStyle(.bordered)
            }
          }
        }
        .padding(.top, 2)
      }
    }
    .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
    .onTapGesture {
      onSelect()
    }
    .onHover { hovering in
      isHovered = hovering
    }
    .padding(4)
    .frame(maxWidth: .infinity, alignment: .leading)
    .scaleEffect(isSelected || isHovered ? 1.003 : 1)
    .softPanel(tone: entryTone, isSelected: isSelected)
    .animation(AmentiaMotionStyle.quick, value: isSelected)
    .animation(AmentiaMotionStyle.quick, value: isHovered)
  }

  private var entryTone: StatusTone {
    switch entry.kind {
    case .plan:
      return .active
    case .tool:
      return .ready
    case .diff:
      return .active
    case .approval:
      return .warning
    case .warning:
      return .warning
    default:
      return .neutral
    }
  }

  private var showsActionRow: Bool {
    showsApprovalActions
      || showsPluginEnableAction
      || showsPluginAuthorizeAction
      || showsPluginInputAction
      || showsPluginRetryAction
      || showsPluginFollowUpAction
      || showsPluginGuidanceDisableAction
      || showsPluginSourceAction
      || showsPluginRefreshAction
      || localExecutionRecoveryTitle != nil
      || externalActionTitle != nil
      || externalCopyActionTitle != nil
  }

  @ViewBuilder
  private var badgeRow: some View {
    if sandboxBadge != nil || !evidenceBadges.isEmpty {
      ScrollView(.horizontal, showsIndicators: false) {
        HStack(spacing: 6) {
          if let sandboxBadge {
            TimelineChip(label: sandboxBadge.label, color: sandboxBadge.tone.color)
          }

          ForEach(evidenceBadges, id: \.self) { badge in
            TimelineChip(label: badge.label, color: badge.tone.color)
          }
        }
      }
      .transition(.opacity)
    }
  }

  private func proofSummaryView(_ summary: TimelineProofSummary) -> some View {
    HStack(alignment: .top, spacing: 8) {
      Image(systemName: "checkmark.seal")
        .foregroundColor(.green)
      VStack(alignment: .leading, spacing: 2) {
        Text(summary.title)
          .font(.caption.weight(.semibold))
        Text(summary.detail)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(10)
    .background(Color.green.opacity(0.08))
    .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
  }

  private func approvalOutcomeView(_ summary: TimelineApprovalOutcomeSummary) -> some View {
    HStack(alignment: .top, spacing: 8) {
      Image(systemName: summary.tone == .warning ? "hand.raised" : "checkmark.seal")
        .foregroundColor(summary.tone.color)
      VStack(alignment: .leading, spacing: 2) {
        Text(summary.title)
          .font(.caption.weight(.semibold))
        Text(summary.detail)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(10)
    .background(summary.tone.color.opacity(0.08))
    .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
  }

  private var pluginRetryTitle: String {
    if entry.attributes["retryInputEditable"] == "true" {
      return "Retry After Editing"
    }
    if entry.attributes["retryInput"] != nil || entry.attributes["commandInput"] != nil {
      return "Retry with Input"
    }

    return "Retry"
  }

  private var pluginInputTitle: String {
    entry.attributes["retryInputEditable"] == "true" ? "Edit Retry Input" : "Run with Input"
  }

  private var pluginFollowUpTitle: String {
    let title = entry.attributes["nextCommandLabel"]?
      .trimmingCharacters(in: .whitespacesAndNewlines)
    if let title, !title.isEmpty {
      return title
    }
    return "Continue"
  }

  private var kindLabel: String {
    switch entry.kind {
    case .userMessage:
      return "User"
    case .assistantMessage:
      return "Assistant"
    case .system:
      return "System"
    case .plan:
      return "Plan"
    case .tool:
      return "Action"
    case .diff:
      return "Change"
    case .approval:
      return "Review"
    case .warning:
      return "Warning"
    }
  }

  private var kindColor: Color {
    switch entry.kind {
    case .userMessage:
      return .accentColor
    case .assistantMessage:
      return .blue
    case .system:
      return .secondary
    case .plan:
      return .accentColor
    case .tool:
      return .green
    case .diff:
      return .blue
    case .approval:
      return .orange
    case .warning:
      return .orange
    }
  }

  private var bodyFont: Font {
    switch entry.kind {
    case .diff:
      return .system(.caption, design: .monospaced)
    default:
      return .body
    }
  }

  private var displayBody: String {
    guard entry.kind == .diff else {
      return entry.body
    }

    let lines = entry.body.components(separatedBy: .newlines)
    let previewLimit = 10
    let preview = lines.prefix(previewLimit).joined(separator: "\n")
    if lines.count <= previewLimit {
      return preview
    }

    let remainingLines = lines.count - previewLimit
    let lineLabel = remainingLines == 1 ? "line" : "lines"
    return "\(preview)\n... \(remainingLines) more \(lineLabel). Select this change to inspect the full detail."
  }

  private var streamingLabel: String? {
    guard entry.kind == .assistantMessage,
          let streamingStatus = entry.attributes["streamingStatus"]
    else {
      return nil
    }

    switch streamingStatus {
    case "in_progress":
      return progressLabel().map { "Working \($0)" } ?? "Working"
    case "completed":
      return "Done"
    case "cancelled":
      return "Stopped"
    default:
      return nil
    }
  }

  private var streamingProgressValue: Double? {
    guard entry.kind == .assistantMessage,
          entry.attributes["streamingStatus"] == "in_progress",
          let streamedCharacters = entry.attributes["streamedCharacters"],
          let totalCharacters = entry.attributes["totalCharacters"],
          let streamedValue = Double(streamedCharacters),
          let totalValue = Double(totalCharacters),
          totalValue > 0
    else {
      return nil
    }

    return streamedValue / totalValue
  }

  private var streamingColor: Color {
    switch entry.attributes["streamingStatus"] {
    case "completed":
      return .green
    case "cancelled":
      return .orange
    default:
      return .accentColor
    }
  }

  private var sandboxBadge: TimelineSandboxBadgeSummary? {
    TimelineSandboxBadgePresenter.badge(attributes: entry.attributes)
  }

  private var evidenceBadges: [TimelineEvidenceBadgeSummary] {
    TimelineEvidenceBadgePresenter.badges(attributes: entry.attributes)
  }

  private var contextReceiptSummary: String? {
    TimelineContextReceiptPresenter.cardSummary(entry)
  }

  private func progressLabel() -> String? {
    guard let streamedCharacters = entry.attributes["streamedCharacters"],
          let totalCharacters = entry.attributes["totalCharacters"],
          let streamedValue = Double(streamedCharacters),
          let totalValue = Double(totalCharacters),
          totalValue > 0
    else {
      return nil
    }

    let percentage = Int(((streamedValue / totalValue) * 100).rounded())
    return "\(min(percentage, 100))%"
  }
}

private struct TimelineChip: View {
  let label: String
  let color: Color
  var isProminent = false

  var body: some View {
    Text(label)
      .font(.caption2.weight(isProminent ? .semibold : .medium))
      .foregroundColor(color)
      .lineLimit(1)
      .padding(.horizontal, 8)
      .padding(.vertical, 4)
      .background(color.opacity(isProminent ? 0.12 : 0.08))
      .overlay(
        Capsule()
          .stroke(color.opacity(isProminent ? 0.18 : 0.12), lineWidth: 1)
      )
      .clipShape(Capsule())
  }
}
