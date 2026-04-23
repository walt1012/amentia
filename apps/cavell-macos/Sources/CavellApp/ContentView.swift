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
            || viewModel.draftMessage.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        )
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

      GroupBox("Milestone 1") {
        VStack(alignment: .leading, spacing: 8) {
          Text("Workspace open flow")
          Text("Read, search, shell, diff preview, and approval-gated write tools")
          Text("Tool, diff, and approval timeline cards")
          Text("Workspace-aware prompt loop")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .font(.subheadline)
      }

      GroupBox("Next Integration") {
        Text("Add streaming events, patch review actions, SQLite persistence, and model-backed planning.")
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
      Text(entry.title)
        .font(.headline)
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
