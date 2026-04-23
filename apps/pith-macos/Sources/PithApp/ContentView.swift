import AppKit
import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel

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
        .disabled(viewModel.runtimeState != .ready)
      }

      ToolbarItem {
        Button("New Thread") {
          viewModel.createThread()
        }
        .disabled(viewModel.runtimeState != .ready)
      }

      ToolbarItem(placement: .primaryAction) {
        Button("Launch Runtime") {
          viewModel.launchRuntime()
        }
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
      HStack {
        VStack(alignment: .leading, spacing: 4) {
          Text("Timeline")
            .font(.title2.weight(.semibold))
          Text(viewModel.workspaceDisplayName())
            .font(.caption)
            .foregroundColor(.secondary)
        }
        Spacer()
        VStack(alignment: .trailing, spacing: 2) {
          Text(viewModel.runtimeState.rawValue.capitalized)
            .font(.caption.weight(.medium))
            .foregroundColor(.secondary)
          Text(viewModel.runtimeDetail)
            .font(.caption2)
            .foregroundColor(.secondary)
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

      HStack(alignment: .bottom, spacing: 12) {
        TextField(viewModel.composerPlaceholder(), text: $viewModel.draftMessage)
          .textFieldStyle(.roundedBorder)

        Button("Send") {
          viewModel.sendDraftMessage()
        }
        .buttonStyle(.borderedProminent)
        .disabled(
          viewModel.runtimeState != .ready
            || viewModel.workspace == nil
            || viewModel.selectedThreadID == nil
            || viewModel.isTurnStreaming()
            || viewModel.draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        )

        Button("Cancel") {
          viewModel.cancelActiveTurn()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.isTurnStreaming())
      }
      .padding(20)
    }
    .frame(minWidth: 520)
  }

  private var inspector: some View {
    VStack(alignment: .leading, spacing: 16) {
      Text("Inspector")
        .font(.title3.weight(.semibold))

      GroupBox("Workspace") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.workspaceDisplayName())
            .font(.headline)
          Text(viewModel.workspacePath())
            .font(.subheadline)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Local Model") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.modelDisplayName())
            .font(.headline)
          Text(viewModel.modelStatusSummary())
            .font(.subheadline)
            .foregroundColor(.secondary)
          Text(viewModel.modelDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          Text(viewModel.modelSourceSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          Text(viewModel.modelMetricsSummary())
            .font(.caption2)
            .foregroundColor(.secondary)
          Text(viewModel.modelReadinessSummary())
            .font(.caption2)
            .foregroundColor(.secondary)
          Text(viewModel.modelInstallHintSummary())
            .font(.caption2)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          Text(viewModel.modelSuggestedPathSummary())
            .font(.caption2)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          HStack(spacing: 8) {
            Button("Install Pack Metadata") {
              viewModel.bootstrapModelPackMetadata()
            }
            .buttonStyle(.borderedProminent)

            Button("Reveal Model Folder") {
              viewModel.revealSuggestedModelDirectory()
            }
            .buttonStyle(.bordered)

            Button("Reveal Binary Folder") {
              viewModel.revealSuggestedBinaryDirectory()
            }
            .buttonStyle(.bordered)
          }
          Text(viewModel.modelArtifactPathSummary())
            .font(.caption2)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Memory") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.memoryCountSummary())
            .font(.headline)
          Text(viewModel.memoryDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          Text(viewModel.memoryLatestSummary())
            .font(.caption2)
            .foregroundColor(.secondary)
            .textSelection(.enabled)

          Divider()

          TextField("Workspace note title", text: $viewModel.memoryNoteTitle)
            .textFieldStyle(.roundedBorder)

          TextEditor(text: $viewModel.memoryNoteBody)
            .font(.caption)
            .frame(minHeight: 72)
            .overlay(
              RoundedRectangle(cornerRadius: 8, style: .continuous)
                .stroke(Color.secondary.opacity(0.18), lineWidth: 1)
            )

          Button("Save Workspace Note") {
            viewModel.saveWorkspaceMemoryNote()
          }
          .buttonStyle(.borderedProminent)
          .disabled(!viewModel.canSaveWorkspaceMemoryNote())
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Plugins") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.pluginCountSummary())
            .font(.headline)
          Text(viewModel.pluginDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Thread") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.selectedThreadTitle())
            .font(.headline)
          Text(viewModel.selectedThreadPreview())
            .font(.subheadline)
            .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Selected Item") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.selectedEntryTitle())
            .font(.headline)
          Text(viewModel.selectedEntryMetadata())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          Text(viewModel.selectedEntryBody())
            .font(.subheadline)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      if let diffBody = viewModel.selectedDiffBody() {
        GroupBox("Diff Detail") {
          Text(diffBody)
            .font(.system(.caption, design: .monospaced))
            .foregroundColor(.secondary)
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
      }

      if let memorySummary = viewModel.selectedEntryMemorySummary() {
        GroupBox("Selected Memory Context") {
          Text(memorySummary)
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
      }

      GroupBox("Milestone 1") {
        VStack(alignment: .leading, spacing: 8) {
          Text("Workspace open flow")
          Text("Read, search, shell, diff preview, and approval-gated write tools")
          Text("SQLite-backed thread persistence and workspace restoration")
          Text("Built-in workspace memory with user notes, thread summaries, and retrieval")
          Text("Workspace-aware prompt loop with cancel control")
          Text("Local model health and summarizer runtime wiring")
          Text("Optional plugin discovery kept separate from core memory")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .font(.subheadline)
      }

      GroupBox("Next Integration") {
        Text("Refine token events, richer planner prompts, and packaged llama.cpp delivery.")
          .font(.subheadline)
          .foregroundColor(.secondary)
      }

      Spacer()
    }
    .padding(20)
    .frame(minWidth: 280)
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

      Text(entry.body)
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

  private var bodyFont: Font {
    switch entry.kind {
    case .diff:
      return .system(.body, design: .monospaced)
    default:
      return .body
    }
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
        Text("Default built-in model: LFM2.5-350M")
      }

      Section("Platform") {
        Text("Target: macOS 12+ on Intel")
      }
    }
    .padding(20)
    .frame(width: 420)
  }
}
