import SwiftUI

struct PluginManagerPanel: View {
  private enum Section: String, CaseIterable, Identifiable {
    case catalog = "Catalog"
    case access = "Access"
    case connectors = "Connectors"
    case commands = "Commands"
    case hooks = "Hooks"

    var id: String { rawValue }
  }

  @ObservedObject var viewModel: AppViewModel
  @State private var section: Section = .catalog

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginCountSummary())
        .font(.headline)

      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(pluginSurfaceSummary)
          .font(.caption)
          .foregroundColor(.secondary)

        Spacer()

        Button("Install Local Plugin") {
          viewModel.installPlugin()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canInstallPlugin())
      }

      Picker("Surface", selection: $section) {
        ForEach(Section.allCases) { section in
          Text(section.rawValue)
            .tag(section)
        }
      }
      .pickerStyle(.menu)

      sectionBody
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  private var pluginSurfaceSummary: String {
    [
      viewModel.pluginRegistryCountSummary(),
      viewModel.pluginConnectorCountSummary(),
      viewModel.pluginCommandCountSummary(),
      viewModel.pluginHookCountSummary(),
    ].joined(separator: " | ")
  }

  @ViewBuilder
  private var sectionBody: some View {
    switch section {
    case .catalog:
      catalogSection
    case .access:
      accessSection
    case .commands:
      commandsSection
    case .connectors:
      connectorsSection
    case .hooks:
      hooksSection
    }
  }

  private var catalogSection: some View {
    VStack(alignment: .leading, spacing: 10) {
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
    }
  }

  private var accessSection: some View {
    VStack(alignment: .leading, spacing: 10) {
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
    }
  }

  private var commandsSection: some View {
    VStack(alignment: .leading, spacing: 10) {
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
    }
  }

  private var connectorsSection: some View {
    VStack(alignment: .leading, spacing: 10) {
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
    }
  }

  private var hooksSection: some View {
    VStack(alignment: .leading, spacing: 10) {
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
