import AppKit
import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel
  @AppStorage("pith.inspector.workspaceExpanded") private var workspaceExpanded = false
  @AppStorage("pith.inspector.localModelExpanded") private var localModelExpanded = false
  @AppStorage("pith.inspector.memoryExpanded") private var memoryExpanded = false
  @AppStorage("pith.inspector.pluginManagerExpanded") private var pluginManagerExpanded = false
  @AppStorage("pith.inspector.threadExpanded") private var threadExpanded = false
  @AppStorage("pith.inspector.selectedItemExpanded") private var selectedItemExpanded = true
  @AppStorage("pith.inspector.selectedMemoryExpanded") private var selectedMemoryExpanded = false
  @AppStorage("pith.inspector.selectedAttributesExpanded") private var selectedAttributesExpanded = false

  var body: some View {
    NavigationView {
      sidebar
      timeline
      inspector
    }
    .toolbar {
      ToolbarItem {
        Button("Open Workspace") {
          viewModel.openWorkspace()
        }
        .disabled(!viewModel.canOpenWorkspace())
      }

      ToolbarItem {
        Button("New Thread") {
          viewModel.createThread()
        }
        .disabled(!viewModel.canCreateThread())
      }

      ToolbarItem(placement: .primaryAction) {
        Button(viewModel.runtimeLaunchButtonTitle()) {
          viewModel.launchRuntime()
        }
        .disabled(!viewModel.canLaunchRuntime())
      }
    }
  }

  private var sidebar: some View {
    List(selection: Binding(get: { viewModel.selectedThreadID }, set: { viewModel.selectThread(id: $0) })) {
      Section("Threads") {
        ForEach(viewModel.threads) { thread in
          VStack(alignment: .leading, spacing: 4) {
            Text(thread.title)
              .font(.headline)
            Text(thread.preview)
              .font(.caption)
              .foregroundColor(.secondary)
          }
          .padding(.vertical, 4)
          .tag(thread.id)
        }
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
  }

  private var timeline: some View {
    VStack(alignment: .leading, spacing: 0) {
      VStack(alignment: .leading, spacing: 10) {
        HStack {
          VStack(alignment: .leading, spacing: 4) {
            Text("Timeline")
              .font(.title2.weight(.semibold))
            Text(viewModel.workspaceDisplayName())
              .font(.caption)
              .foregroundColor(.secondary)
          }
          Spacer()
          VStack(alignment: .trailing, spacing: 6) {
            HStack(spacing: 6) {
              if viewModel.showsRuntimeActivity() {
                ProgressView()
                  .controlSize(.small)
              }
              StatusPill(
                label: viewModel.runtimeState.rawValue.capitalized,
                tone: viewModel.runtimeStatusTone()
              )
            }
            Text(viewModel.runtimeStatusSummary())
              .font(.caption2)
              .foregroundColor(.secondary)
              .multilineTextAlignment(.trailing)
            Text(viewModel.runtimeDetail)
              .font(.caption2)
              .foregroundColor(.secondary)
              .lineLimit(2)
              .multilineTextAlignment(.trailing)
            if let actionTitle = viewModel.runtimePrimaryActionTitle() {
              Button(actionTitle) {
                viewModel.runRuntimePrimaryAction()
              }
              .buttonStyle(.bordered)
              .disabled(!viewModel.canRunRuntimePrimaryAction())
            }
          }
        }

        SetupProgressView(
          summary: viewModel.setupProgressSummary(),
          value: viewModel.setupProgressValue(),
          tone: viewModel.setupProgressTone()
        )

        if viewModel.shouldShowSetupCallout() {
          if viewModel.shouldShowSetupModelChoice() {
            SetupModelChooser(
              models: viewModel.localModels,
              selectedModelID: $viewModel.selectedSetupModelID,
              detail: viewModel.setupModelChoiceDetail(),
              isDisabled: !viewModel.canChangeSetupModelChoice()
            )
          }

          SetupCallout(
            title: viewModel.setupCalloutTitle(),
            summary: viewModel.setupCalloutSummary(),
            detail: viewModel.setupCalloutDetail(),
            tone: viewModel.setupCalloutTone(),
            actionTitle: viewModel.setupCalloutActionTitle(),
            canRunAction: viewModel.canRunSetupCalloutAction(),
            secondaryActionTitle: viewModel.setupCalloutSecondaryActionTitle(),
            canRunSecondaryAction: viewModel.canRunSetupCalloutSecondaryAction(),
            onAction: {
              viewModel.runSetupCalloutAction()
            },
            onSecondaryAction: {
              viewModel.runSetupCalloutSecondaryAction()
            }
          )
        }

        HStack(spacing: 8) {
          ForEach(viewModel.runtimeReadinessSteps()) { step in
            ReadinessChip(step: step)
          }
          Spacer()
        }
      }
      .padding(20)

      Divider()

      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          ForEach(viewModel.timeline) { entry in
            TimelineCard(
              entry: entry,
              isSelected: viewModel.selectedEntryID == entry.id,
              showsApprovalActions: viewModel.isPendingApproval(entry),
              onSelect: {
                viewModel.selectTimelineEntry(id: entry.id)
              },
              onApprove: {
                guard let approvalID = viewModel.approvalID(for: entry) else {
                  return
                }
                viewModel.respondToApproval(approvalID: approvalID, decision: "approved")
              },
              onDeny: {
                guard let approvalID = viewModel.approvalID(for: entry) else {
                  return
                }
                viewModel.respondToApproval(approvalID: approvalID, decision: "denied")
              }
            )
          }
        }
        .padding(20)
      }

      Divider()

      VStack(alignment: .leading, spacing: 8) {
        HStack(alignment: .bottom, spacing: 12) {
          TextField(viewModel.composerPlaceholder(), text: $viewModel.draftMessage)
            .textFieldStyle(.roundedBorder)
            .disabled(!viewModel.canUseComposer())
            .onSubmit {
              if viewModel.canSendDraftMessage() {
                viewModel.sendDraftMessage()
              }
            }

          Button("Send") {
            viewModel.sendDraftMessage()
          }
          .buttonStyle(.borderedProminent)
          .disabled(!viewModel.canSendDraftMessage())

          Button("Cancel") {
            viewModel.cancelActiveTurn()
          }
          .buttonStyle(.bordered)
          .disabled(!viewModel.canCancelActiveTurn())
        }

        if !viewModel.composerSuggestions().isEmpty {
          ComposerSuggestionStrip(
            suggestions: viewModel.composerSuggestions(),
            onSelect: { suggestion in
              viewModel.useComposerSuggestion(suggestion)
            }
          )
        }

        HStack(spacing: 6) {
          if viewModel.showsComposerActivity() {
            ProgressView()
              .controlSize(.small)
          }
          Text(viewModel.composerStatusSummary())
            .font(.caption2)
            .foregroundColor(viewModel.runtimeState == .failed ? .red : .secondary)
        }
      }
      .padding(20)
    }
    .frame(minWidth: 520)
  }

  private var inspector: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 16) {
        Text("Inspector")
          .font(.title3.weight(.semibold))

        InspectorSessionCard(
          title: viewModel.inspectorSessionTitle(),
          detail: viewModel.inspectorSessionDetail(),
          meta: viewModel.inspectorSessionMetaSummary(),
          tone: viewModel.runtimeStatusTone()
        )

        DisclosureGroup("Workspace Search", isExpanded: $workspaceExpanded) {
          WorkspaceSearchPanel(viewModel: viewModel)
        }

        DisclosureGroup("Local Model", isExpanded: $localModelExpanded) {
          LocalModelPanel(viewModel: viewModel)
        }

        DisclosureGroup("Memory", isExpanded: $memoryExpanded) {
          MemoryPanel(viewModel: viewModel)
        }

        DisclosureGroup("Plugin Manager", isExpanded: $pluginManagerExpanded) {
          PluginManagerPanel(viewModel: viewModel)
        }

        DisclosureGroup("Thread", isExpanded: $threadExpanded) {
          VStack(alignment: .leading, spacing: 8) {
            Text(viewModel.selectedThreadTitle())
              .font(.headline)
            Text(viewModel.selectedThreadPreview())
              .font(.subheadline)
              .foregroundColor(.secondary)
          }
          .frame(maxWidth: .infinity, alignment: .leading)
        }

        DisclosureGroup("Selected Item", isExpanded: $selectedItemExpanded) {
          VStack(alignment: .leading, spacing: 8) {
            Text(viewModel.selectedEntryTitle())
              .font(.headline)
            Text(viewModel.selectedEntryBody())
              .font(.subheadline)
              .foregroundColor(.secondary)
              .textSelection(.enabled)
            DisclosureGroup("Attributes", isExpanded: $selectedAttributesExpanded) {
              Text(viewModel.selectedEntryMetadata())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
          }
          .frame(maxWidth: .infinity, alignment: .leading)
        }

        if let diffSummary = viewModel.selectedDiffSummary() {
          GroupBox("Diff Detail") {
            DiffDetailView(summary: diffSummary, lines: viewModel.selectedDiffLines())
          }
        }

        if let memorySummary = viewModel.selectedEntryMemorySummary() {
          DisclosureGroup("Selected Memory Context", isExpanded: $selectedMemoryExpanded) {
            Text(memorySummary)
              .font(.caption)
              .foregroundColor(.secondary)
              .textSelection(.enabled)
              .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
      }
      .padding(20)
    }
    .frame(minWidth: 280)
  }
}

private struct StatusPill: View {
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

private struct InspectorSessionCard: View {
  let title: String
  let detail: String
  let meta: String
  let tone: StatusTone

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(spacing: 8) {
        Circle()
          .fill(tone.color)
          .frame(width: 8, height: 8)
        Text(title)
          .font(.headline)
        Spacer()
      }

      Text(detail)
        .font(.caption)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)

      Text(meta)
        .font(.caption2.weight(.medium))
        .foregroundColor(tone.color)
        .lineLimit(2)
    }
    .padding(10)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(tone.color.opacity(0.08))
    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
  }
}

private struct ReadinessChip: View {
  let step: ReadinessStepSummary

  var body: some View {
    HStack(spacing: 4) {
      Text(step.label)
        .font(.caption2.weight(.medium))
        .foregroundColor(.secondary)
      Text(step.detail)
        .font(.caption2.weight(.semibold))
        .foregroundColor(step.tone.color)
        .lineLimit(1)
        .truncationMode(.tail)
        .frame(maxWidth: 150, alignment: .leading)
    }
    .padding(.horizontal, 8)
    .padding(.vertical, 5)
    .background(step.tone.color.opacity(0.10))
    .clipShape(Capsule())
  }
}

private struct ComposerSuggestionStrip: View {
  let suggestions: [ComposerSuggestionSummary]
  let onSelect: (ComposerSuggestionSummary) -> Void

  var body: some View {
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 6) {
        Text("Try")
          .font(.caption2.weight(.semibold))
          .foregroundColor(.secondary)

        ForEach(suggestions) { suggestion in
          Button(suggestion.title) {
            onSelect(suggestion)
          }
          .buttonStyle(.bordered)
          .controlSize(.small)
          .help(suggestion.message)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
    }
  }
}

private struct SetupProgressView: View {
  let summary: String
  let value: Double
  let tone: StatusTone

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack {
        Text(summary)
          .font(.caption2.weight(.semibold))
          .foregroundColor(tone.color)
        Spacer()
        Text("Runtime -> Model -> Workspace -> Thread")
          .font(.caption2)
          .foregroundColor(.secondary)
      }
      ProgressView(value: value)
        .progressViewStyle(.linear)
        .tint(tone.color)
    }
  }
}

private struct SetupModelChooser: View {
  let models: [LocalModelSummary]
  @Binding var selectedModelID: String
  let detail: String
  let isDisabled: Bool

  var body: some View {
    HStack(alignment: .top, spacing: 12) {
      VStack(alignment: .leading, spacing: 4) {
        Text("Choose Local Model")
          .font(.caption.weight(.semibold))
        Text(detail)
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }

      Spacer()

      Picker("Local Model", selection: $selectedModelID) {
        ForEach(models) { model in
          Text(model.displayName).tag(model.id)
        }
      }
      .labelsHidden()
      .frame(width: 260)
      .disabled(isDisabled)
    }
    .padding(10)
    .background(Color.secondary.opacity(0.08))
    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
  }
}

private struct SetupCallout: View {
  let title: String
  let summary: String
  let detail: String
  let tone: StatusTone
  let actionTitle: String?
  let canRunAction: Bool
  let secondaryActionTitle: String?
  let canRunSecondaryAction: Bool
  let onAction: () -> Void
  let onSecondaryAction: () -> Void

  var body: some View {
    HStack(alignment: .top, spacing: 12) {
      VStack(alignment: .leading, spacing: 4) {
        Text(title)
          .font(.caption.weight(.semibold))
          .foregroundColor(tone.color)
        Text(summary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
        Text(detail)
          .font(.caption2)
          .foregroundColor(.secondary)
      }

      Spacer()

      VStack(alignment: .trailing, spacing: 6) {
        if let actionTitle {
          Button(actionTitle) {
            onAction()
          }
          .buttonStyle(.borderedProminent)
          .disabled(!canRunAction)
        }

        if let secondaryActionTitle {
          Button(secondaryActionTitle) {
            onSecondaryAction()
          }
          .buttonStyle(.bordered)
          .disabled(!canRunSecondaryAction)
        }
      }
    }
    .padding(10)
    .background(tone.color.opacity(0.10))
    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
  }
}

private extension StatusTone {
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

private struct DiffDetailView: View {
  let summary: String
  let lines: [DiffLineSummary]

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(summary)
        .font(.caption.weight(.semibold))
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      ScrollView(.horizontal) {
        VStack(alignment: .leading, spacing: 2) {
          ForEach(lines) { line in
            DiffLineRow(line: line)
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}

private struct DiffLineRow: View {
  let line: DiffLineSummary

  var body: some View {
    HStack(alignment: .top, spacing: 8) {
      Text("\(line.lineNumber)")
        .font(.system(.caption2, design: .monospaced))
        .foregroundColor(.secondary)
        .frame(width: 34, alignment: .trailing)

      Text(line.text.isEmpty ? " " : line.text)
        .font(.system(.caption, design: .monospaced))
        .foregroundColor(foregroundColor)
        .textSelection(.enabled)
    }
    .padding(.vertical, 2)
    .padding(.horizontal, 6)
    .background(backgroundColor)
    .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
  }

  private var foregroundColor: Color {
    switch line.kind {
    case .addition:
      return .green
    case .deletion:
      return .red
    case .hunk:
      return .blue
    case .metadata:
      return .secondary
    case .context:
      return .primary
    }
  }

  private var backgroundColor: Color {
    switch line.kind {
    case .addition:
      return Color.green.opacity(0.10)
    case .deletion:
      return Color.red.opacity(0.10)
    case .hunk:
      return Color.blue.opacity(0.10)
    case .metadata:
      return Color.secondary.opacity(0.08)
    case .context:
      return Color.clear
    }
  }
}

private struct TimelineCard: View {
  let entry: TimelineEntry
  let isSelected: Bool
  let showsApprovalActions: Bool
  let onSelect: () -> Void
  let onApprove: () -> Void
  let onDeny: () -> Void

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

      if showsApprovalActions {
        HStack(spacing: 12) {
          Button("Approve") {
            onApprove()
          }
          .buttonStyle(.borderedProminent)

          Button("Deny") {
            onDeny()
          }
          .buttonStyle(.bordered)
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

struct SettingsView: View {
  var body: some View {
    Form {
      Section("Model") {
        Text("Default first-use suggestion: LFM2.5-350M")
      }

      Section("Platform") {
        Text("Target: macOS 12+ on Intel")
      }
    }
    .padding(20)
    .frame(width: 420)
  }
}
