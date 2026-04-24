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

      ToolbarItem {
        Button("Install Plugin") {
          viewModel.installPlugin()
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
          Text(viewModel.localPluginCountSummary())
            .font(.caption)
            .foregroundColor(.secondary)

          if viewModel.plugins.isEmpty {
            Text(viewModel.pluginDetailSummary())
              .font(.caption)
              .foregroundColor(.secondary)
              .textSelection(.enabled)
          } else {
            ForEach(viewModel.plugins) { plugin in
              PluginRow(
                plugin: plugin,
                canEdit: viewModel.runtimeState == .ready && plugin.status == "ready",
                canRemove: viewModel.runtimeState == .ready && viewModel.isRemovablePlugin(plugin),
                onSetEnabled: { enabled in
                  viewModel.setPluginEnabled(pluginID: plugin.id, enabled: enabled)
                },
                onRemove: {
                  viewModel.removePlugin(pluginID: plugin.id)
                }
              )
            }
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Plugin Permissions") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.pluginPermissionCountSummary())
            .font(.headline)

          Text(viewModel.pluginPermissionDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)

          if !viewModel.pluginPermissionPreview().isEmpty {
            ForEach(viewModel.pluginPermissionPreview()) { plugin in
              PluginPermissionRow(
                plugin: plugin,
                onRevealManifest: {
                  viewModel.revealPluginManifest(pluginID: plugin.id)
                }
              )
            }
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Plugin Validation") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.invalidPluginCountSummary())
            .font(.headline)

          Text(viewModel.invalidPluginDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)

          if !viewModel.invalidPlugins().isEmpty {
            ForEach(viewModel.invalidPlugins()) { plugin in
              InvalidPluginRow(
                plugin: plugin,
                onRevealManifest: {
                  viewModel.revealPluginManifest(pluginID: plugin.id)
                },
                onRemove: {
                  viewModel.removePlugin(pluginID: plugin.id)
                }
              )
            }
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Plugin Registry") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.pluginRegistryCountSummary())
            .font(.headline)

          Text(viewModel.pluginRegistryDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)

          if !viewModel.pluginCapabilityPreview().isEmpty {
            ForEach(viewModel.pluginCapabilityPreview()) { capability in
              PluginCapabilityRow(capability: capability)
            }
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Plugin Commands") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.pluginCommandCountSummary())
            .font(.headline)

          Text(viewModel.pluginCommandDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)

          if !viewModel.pluginCommands.isEmpty {
            ForEach(viewModel.pluginCommands) { command in
              PluginCommandRow(
                command: command,
                canRun: viewModel.runtimeState == .ready
                  && viewModel.selectedThreadID != nil
                  && viewModel.activeTurnID == nil,
                onRun: {
                  viewModel.runPluginCommand(commandID: command.id)
                }
              )
            }
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      GroupBox("Plugin Hooks") {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.pluginHookCountSummary())
            .font(.headline)

          Text(viewModel.pluginHookDetailSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .textSelection(.enabled)

          if !viewModel.pluginHooks.isEmpty {
            ForEach(viewModel.pluginHooks) { hook in
              PluginHookRow(hook: hook)
            }
          }
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

      GroupBox("Milestone 2 Focus") {
        Text("Expand plugin management, manifest workflows, plugin commands and hooks, and richer local model delivery.")
          .font(.subheadline)
          .foregroundColor(.secondary)
      }

      Spacer()
    }
    .padding(20)
    .frame(minWidth: 280)
  }
}

private struct PluginRow: View {
  let plugin: PluginSummary
  let canEdit: Bool
  let canRemove: Bool
  let onSetEnabled: (Bool) -> Void
  let onRemove: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.subheadline.weight(.semibold))
          Text("\(plugin.version) | \(plugin.provenance) | \(plugin.status)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Toggle(
          "",
          isOn: Binding(
            get: { plugin.enabled },
            set: onSetEnabled
          )
        )
        .labelsHidden()
        .disabled(!canEdit)
      }

      if canRemove {
        Button("Remove Local Plugin") {
          onRemove()
        }
        .buttonStyle(.bordered)
      }

      Text(plugin.description)
        .font(.caption)
        .foregroundColor(.secondary)

      if !plugin.capabilities.isEmpty {
        Text("Capabilities: \(plugin.capabilities.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if !plugin.permissions.isEmpty {
        Text("Permissions: \(plugin.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let validationError = plugin.validationError {
        Text(validationError)
          .font(.caption2)
          .foregroundColor(.orange)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

private struct PluginCapabilityRow: View {
  let capability: PluginCapabilitySummary

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text("\(capability.kind):\(capability.identifier)")
            .font(.caption.weight(.semibold))
          Text("\(capability.pluginDisplayName) | \(capability.pluginID)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()
      }

      if !capability.permissions.isEmpty {
        Text("Permissions: \(capability.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

private struct PluginPermissionRow: View {
  let plugin: PluginSummary
  let onRevealManifest: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.caption.weight(.semibold))
          Text(plugin.enabled ? "Enabled" : "Disabled")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Button("Reveal Manifest") {
          onRevealManifest()
        }
        .buttonStyle(.bordered)
      }

      if plugin.permissions.isEmpty {
        Text("No extra runtime permissions declared.")
          .font(.caption2)
          .foregroundColor(.secondary)
      } else {
        Text(plugin.permissions.joined(separator: ", "))
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

private struct PluginCommandRow: View {
  let command: PluginCommandSummary
  let canRun: Bool
  let onRun: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(command.title)
            .font(.caption.weight(.semibold))
          Text("\(command.pluginDisplayName) | \(command.pluginID)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()

        Button("Run") {
          onRun()
        }
        .buttonStyle(.bordered)
        .disabled(!canRun)
      }

      Text(command.description)
        .font(.caption2)
        .foregroundColor(.secondary)

      if !command.permissions.isEmpty {
        Text("Permissions: \(command.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

private struct InvalidPluginRow: View {
  let plugin: PluginSummary
  let onRevealManifest: () -> Void
  let onRemove: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(plugin.displayName)
            .font(.caption.weight(.semibold))
          Text(plugin.manifestPath)
            .font(.caption2)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
        }

        Spacer()

        Button("Reveal Manifest") {
          onRevealManifest()
        }
        .buttonStyle(.bordered)

        if plugin.provenance == "local" {
          Button("Remove Local Plugin") {
            onRemove()
          }
          .buttonStyle(.bordered)
        }
      }

      Text(plugin.validationError ?? "Plugin manifest did not pass runtime validation.")
        .font(.caption2)
        .foregroundColor(.orange)
        .textSelection(.enabled)
    }
    .padding(.vertical, 4)
  }
}

private struct PluginHookRow: View {
  let hook: PluginHookSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(hook.title)
            .font(.caption.weight(.semibold))
          Text("\(hook.pluginDisplayName) | \(hook.event)")
            .font(.caption2)
            .foregroundColor(.secondary)
        }

        Spacer()
      }

      Text(hook.description)
        .font(.caption2)
        .foregroundColor(.secondary)

      if !hook.permissions.isEmpty {
        Text("Permissions: \(hook.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
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
