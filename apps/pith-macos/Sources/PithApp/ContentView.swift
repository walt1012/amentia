import AppKit
import SwiftUI

struct ContentView: View {
  private enum PluginInspectorSection: String, CaseIterable, Identifiable {
    case catalog = "Catalog"
    case access = "Access"
    case connectors = "Connectors"
    case commands = "Commands"
    case hooks = "Hooks"

    var id: String { rawValue }
  }

  @ObservedObject var viewModel: AppViewModel
  @State private var pluginInspectorSection: PluginInspectorSection = .catalog
  @State private var localModelExpanded = false
  @State private var modelManagerExpanded = false
  @State private var memoryExpanded = false
  @State private var pluginManagerExpanded = false

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
            || !viewModel.isLocalModelReady()
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
    ScrollView {
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

        DisclosureGroup("Local Model", isExpanded: $localModelExpanded) {
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
            if viewModel.shouldShowModelDownloadProgress() {
              VStack(alignment: .leading, spacing: 4) {
                if let progressValue = viewModel.modelDownloadProgressValue() {
                  ProgressView(value: progressValue)
                    .progressViewStyle(.linear)
                } else {
                  ProgressView()
                    .progressViewStyle(.linear)
                }
                Text(viewModel.modelDownloadProgressSummary())
                  .font(.caption2)
                  .foregroundColor(.secondary)
                  .textSelection(.enabled)
              }
            }
            HStack(spacing: 8) {
              Button(viewModel.defaultModelDownloadButtonTitle()) {
                viewModel.downloadLocalModel()
              }
              .buttonStyle(.borderedProminent)
              .disabled(!viewModel.canDownloadLocalModel())

              Button("Pause Download") {
                viewModel.pauseModelDownload()
              }
              .buttonStyle(.bordered)
              .disabled(!viewModel.canPauseModelDownload())

              Button("Cancel Download") {
                viewModel.cancelModelDownload()
              }
              .buttonStyle(.bordered)
              .disabled(!viewModel.canCancelModelDownload())

              Button("Install Pack Metadata") {
                viewModel.bootstrapModelPackMetadata()
              }
              .buttonStyle(.bordered)

              Button("Reveal Model Folder") {
                viewModel.revealSuggestedModelDirectory()
              }
              .buttonStyle(.bordered)

              Button("Reveal Binary Folder") {
                viewModel.revealSuggestedBinaryDirectory()
              }
              .buttonStyle(.bordered)
            }
            DisclosureGroup("Model Manager", isExpanded: $modelManagerExpanded) {
              VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                  Text(viewModel.modelManagerSummary())
                    .font(.caption2)
                    .foregroundColor(.secondary)
                  Spacer()
                  Button("Use Default") {
                    viewModel.resetActiveLocalModel()
                  }
                  .buttonStyle(.bordered)
                  .disabled(!viewModel.canResetActiveLocalModel())
                }
                ForEach(viewModel.localModels) { model in
                  VStack(alignment: .leading, spacing: 5) {
                    Text(model.displayName)
                      .font(.caption)
                      .fontWeight(.semibold)
                    Text(model.description)
                      .font(.caption2)
                      .foregroundColor(.secondary)
                    Text(viewModel.localModelStatusSummary(model))
                      .font(.caption2)
                      .foregroundColor(.secondary)
                    Text(viewModel.localModelTagSummary(model))
                      .font(.caption2)
                      .foregroundColor(.secondary)
                    HStack(spacing: 8) {
                      Button(model.active ? "Active" : "Use") {
                        viewModel.activateRecommendedModel(modelID: model.id)
                      }
                      .buttonStyle(.borderedProminent)
                      .disabled(!viewModel.canActivateRecommendedModel(modelID: model.id))

                      Button(viewModel.localModelDownloadButtonTitle(model)) {
                        viewModel.downloadRecommendedModel(modelID: model.id)
                      }
                      .buttonStyle(.bordered)
                      .disabled(!viewModel.canDownloadRecommendedModel(modelID: model.id))

                      Button("Reveal") {
                        viewModel.revealRecommendedModel(modelID: model.id)
                      }
                      .buttonStyle(.bordered)
                      .disabled(!model.downloaded)
                    }
                    Text(viewModel.localModelPathSummary(model))
                      .font(.caption2)
                      .foregroundColor(.secondary)
                      .textSelection(.enabled)
                  }
                  .padding(.vertical, 4)
                }
              }
              .frame(maxWidth: .infinity, alignment: .leading)
            }
            Text(viewModel.modelArtifactPathSummary())
              .font(.caption2)
              .foregroundColor(.secondary)
              .textSelection(.enabled)
          }
          .frame(maxWidth: .infinity, alignment: .leading)
        }

        DisclosureGroup("Memory", isExpanded: $memoryExpanded) {
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

        DisclosureGroup("Plugin Manager", isExpanded: $pluginManagerExpanded) {
          VStack(alignment: .leading, spacing: 10) {
            Text(viewModel.pluginCountSummary())
              .font(.headline)
            Text(
              "\(viewModel.pluginRegistryCountSummary()) | \(viewModel.pluginConnectorCountSummary()) | \(viewModel.pluginCommandCountSummary()) | \(viewModel.pluginHookCountSummary())"
            )
            .font(.caption)
            .foregroundColor(.secondary)
            Picker("Surface", selection: $pluginInspectorSection) {
              ForEach(PluginInspectorSection.allCases) { section in
                Text(section.rawValue)
                  .tag(section)
              }
            }
            .pickerStyle(.menu)

            switch pluginInspectorSection {
            case .catalog:
              Text(viewModel.pluginDetailSummary())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)

              if !viewModel.plugins.isEmpty {
                Divider()
                ForEach(viewModel.plugins) { plugin in
                  PluginRow(
                    plugin: plugin,
                    canEdit: viewModel.runtimeState == .ready && plugin.status == "ready",
                    canRemove: viewModel.runtimeState == .ready
                      && viewModel.isRemovablePlugin(plugin),
                    onSetEnabled: { enabled in
                      viewModel.setPluginEnabled(pluginID: plugin.id, enabled: enabled)
                    },
                    onRemove: {
                      viewModel.removePlugin(pluginID: plugin.id)
                    }
                  )
                }
              }

              if !viewModel.pluginCapabilityPreview().isEmpty {
                Divider()
                Text(viewModel.pluginRegistryDetailSummary())
                  .font(.caption2)
                  .foregroundColor(.secondary)
                  .textSelection(.enabled)
                ForEach(viewModel.pluginCapabilityPreview()) { capability in
                  PluginCapabilityRow(capability: capability)
                }
              }

            case .access:
              Text(viewModel.pluginPermissionDetailSummary())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)

              if !viewModel.pluginPermissionPreview().isEmpty {
                Divider()
                ForEach(viewModel.pluginPermissionPreview()) { plugin in
                  PluginPermissionRow(
                    plugin: plugin,
                    onRevealManifest: {
                      viewModel.revealPluginManifest(pluginID: plugin.id)
                    }
                  )
                }
              }

              if !viewModel.invalidPlugins().isEmpty {
                Divider()
                Text(viewModel.invalidPluginDetailSummary())
                  .font(.caption2)
                  .foregroundColor(.secondary)
                  .textSelection(.enabled)
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

            case .commands:
              Text(viewModel.pluginCommandDetailSummary())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)

              if !viewModel.pluginCommands.isEmpty {
                Divider()
                ForEach(viewModel.pluginCommands) { command in
                  PluginCommandRow(
                    command: command,
                    canRun: viewModel.runtimeState == .ready
                      && viewModel.isLocalModelReady()
                      && viewModel.selectedThreadID != nil
                      && viewModel.activeTurnID == nil
                      && command.executionKind != nil,
                    onRun: {
                      viewModel.runPluginCommand(commandID: command.id)
                    }
                  )
                }
              }

            case .connectors:
              Text(viewModel.pluginConnectorDetailSummary())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)

              if !viewModel.pluginConnectorPreview().isEmpty {
                Divider()
                ForEach(viewModel.pluginConnectorPreview()) { connector in
                  PluginConnectorRow(connector: connector)
                }
              }

            case .hooks:
              Text(viewModel.pluginHookDetailSummary())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)

              if !viewModel.pluginHooks.isEmpty {
                Divider()
                ForEach(viewModel.pluginHooks) { hook in
                  PluginHookRow(hook: hook)
                }
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
      }
      .padding(20)
    }
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

      if let validationHint = plugin.validationHint {
        Text("Repair: \(validationHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
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

      if !capability.metadata.isEmpty {
        Text("Metadata: \(capability.metadataSummary)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }
}

private extension PluginCapabilitySummary {
  var metadataSummary: String {
    metadata
      .sorted(by: { $0.key < $1.key })
      .map { "\($0.key)=\($0.value)" }
      .joined(separator: " | ")
  }
}

private struct PluginConnectorRow: View {
  let connector: PluginConnectorSummary

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(alignment: .top, spacing: 12) {
        VStack(alignment: .leading, spacing: 2) {
          Text(connector.displayName)
            .font(.caption.weight(.semibold))
          Text("\(connector.service) | \(connector.status)")
            .font(.caption2)
            .foregroundColor(statusColor)
        }

        Spacer()
      }

      Text("\(connector.pluginDisplayName) | \(connector.pluginID)")
        .font(.caption2)
        .foregroundColor(.secondary)

      Text(connector.authSummary)
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !connector.permissions.isEmpty {
        Text("Permissions: \(connector.permissions.joined(separator: ", "))")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

      if let homepage = connector.homepage {
        Text("Homepage: \(homepage)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
    }
    .padding(.vertical, 4)
  }

  private var statusColor: Color {
    switch connector.status {
    case "ready":
      return .green
    case "needsAuth":
      return .orange
    default:
      return .secondary
    }
  }
}

private extension PluginConnectorSummary {
  var authSummary: String {
    let type = authType ?? "none"
    let required = authRequired ? "required" : "optional"
    let scopes = authScopes.isEmpty ? "no scopes" : authScopes.joined(separator: ", ")
    let store = credentialStore ?? "none"
    return "Auth: \(type) | \(required) | \(scopes) | store: \(store)"
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

      Text("Execution: \(command.executionKind ?? "missing contract")")
        .font(.caption2)
        .foregroundColor(command.executionKind == nil ? .orange : .secondary)
        .textSelection(.enabled)

      if let memorySummary = command.memorySummary {
        Text(memorySummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

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

      if let validationHint = plugin.validationHint {
        Text("Repair: \(validationHint)")
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }
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

      if let memorySummary = hook.memorySummary {
        Text(memorySummary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
      }

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
