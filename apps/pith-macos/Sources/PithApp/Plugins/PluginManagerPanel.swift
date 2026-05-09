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
            canEdit: viewModel.canSetPluginEnabled(pluginID: plugin.id),
            canRemove: viewModel.canRemovePlugin(pluginID: plugin.id),
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
            canRemove: viewModel.canRemovePlugin(pluginID: plugin.id),
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
            canRun: viewModel.canRunPluginCommand(commandID: command.id),
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
