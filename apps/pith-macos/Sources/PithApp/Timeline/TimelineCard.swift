import AppKit
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
  let onRevealPluginSource: () -> Void
  let onRefreshPlugins: () -> Void
  let onRecoverLocalExecution: () -> Void
  let onOpenExternalAction: () -> Void
  let onCopyExternalAction: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .center, spacing: 8) {
        Text(kindLabel)
          .font(.caption2.weight(.semibold))
          .foregroundColor(kindColor)
          .padding(.horizontal, 8)
          .padding(.vertical, 4)
          .background(kindColor.opacity(0.12))
          .clipShape(Capsule())

        Text(entry.title)
          .font(.headline)

        Spacer()

        if let streamingLabel {
          Text(streamingLabel)
            .font(.caption2.weight(.semibold))
            .foregroundColor(streamingColor)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(streamingColor.opacity(0.12))
            .clipShape(Capsule())
        }

        if let sandboxBadge {
          Text(sandboxBadge.label)
            .font(.caption2.weight(.semibold))
            .foregroundColor(sandboxBadge.tone.color)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(sandboxBadge.tone.color.opacity(0.12))
            .clipShape(Capsule())
        }

        ForEach(evidenceBadges, id: \.self) { badge in
          Text(badge.label)
            .font(.caption2.weight(.semibold))
            .foregroundColor(badge.tone.color)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(badge.tone.color.opacity(0.12))
            .clipShape(Capsule())
        }
      }

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
        HStack(spacing: 12) {
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
            Button("Enable Plugin") {
              onEnablePlugin()
            }
            .buttonStyle(.borderedProminent)
          }

          if showsPluginAuthorizeAction {
            Button("Authorize Connector") {
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

          if showsPluginSourceAction {
            Button("Reveal Source") {
              onRevealPluginSource()
            }
            .buttonStyle(.bordered)
          }

          if showsPluginRefreshAction {
            Button("Refresh Plugins") {
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
        .padding(.top, 4)
      }
    }
    .contentShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
    .onTapGesture {
      onSelect()
    }
    .padding(16)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(backgroundColor)
    .overlay(
      RoundedRectangle(cornerRadius: 12, style: .continuous)
        .strokeBorder(isSelected ? Color.accentColor.opacity(0.45) : Color.clear, lineWidth: 1.5)
    )
    .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
  }

  private var backgroundColor: Color {
    switch entry.kind {
    case .plan:
      return Color.accentColor.opacity(0.12)
    case .tool:
      return Color.green.opacity(0.12)
    case .diff:
      return Color.blue.opacity(0.1)
    case .approval:
      return Color.yellow.opacity(0.16)
    case .warning:
      return Color.orange.opacity(0.16)
    default:
      return Color(NSColor.controlBackgroundColor)
    }
  }

  private var showsActionRow: Bool {
    showsApprovalActions
      || showsPluginEnableAction
      || showsPluginAuthorizeAction
      || showsPluginInputAction
      || showsPluginRetryAction
      || showsPluginFollowUpAction
      || showsPluginSourceAction
      || showsPluginRefreshAction
      || localExecutionRecoveryTitle != nil
      || externalActionTitle != nil
      || externalCopyActionTitle != nil
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
      return "Tool"
    case .diff:
      return "Diff"
    case .approval:
      return "Approval"
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

    return "\(preview)\n... \(lines.count - previewLimit) more line(s). Select this diff to inspect the full highlighted change."
  }

  private var streamingLabel: String? {
    guard entry.kind == .assistantMessage,
          let streamingStatus = entry.attributes["streamingStatus"]
    else {
      return nil
    }

    switch streamingStatus {
    case "in_progress":
      return progressLabel().map { "Streaming \($0)" } ?? "Streaming"
    case "completed":
      return "Completed"
    case "cancelled":
      return "Cancelled"
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
