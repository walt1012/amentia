import SwiftUI

enum PluginManagerSection: String, CaseIterable, Identifiable {
  case catalog = "Catalog"
  case access = "Access"
  case connectors = "Connectors"
  case commands = "Commands"
  case hooks = "Hooks"

  var id: String { rawValue }
}

struct PluginManagerSectionView: View {
  @ObservedObject var viewModel: AppViewModel
  let section: PluginManagerSection

  var body: some View {
    switch section {
    case .catalog:
      PluginCatalogSection(viewModel: viewModel)
    case .access:
      PluginAccessSection(viewModel: viewModel)
    case .commands:
      PluginCommandsSection(viewModel: viewModel)
    case .connectors:
      PluginConnectorsSection(viewModel: viewModel)
    case .hooks:
      PluginHooksSection(viewModel: viewModel)
    }
  }
}

private struct PluginCatalogSection: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !viewModel.pluginCatalogPreview().isEmpty {
        Divider()
        ForEach(viewModel.pluginCatalogPreview()) { plugin in
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
}

private struct PluginAccessSection: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
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
}

private struct PluginCommandsSection: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginCommandDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !viewModel.pluginCommandPreview().isEmpty {
        Divider()
        ForEach(viewModel.pluginCommandPreview()) { command in
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
}

private struct PluginConnectorsSection: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
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
}

private struct PluginHooksSection: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginHookDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !viewModel.pluginHookPreview().isEmpty {
        Divider()
        ForEach(viewModel.pluginHookPreview()) { hook in
          PluginHookRow(hook: hook)
        }
      }
    }
  }
}
