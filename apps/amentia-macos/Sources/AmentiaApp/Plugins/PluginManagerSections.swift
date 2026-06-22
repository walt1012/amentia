import SwiftUI

enum PluginManagerSection: String, CaseIterable, Identifiable {
  case catalog = "Catalog"
  case capabilities = "Capabilities"
  case access = "Access"
  case connectors = "Connectors"
  case commands = "Commands"
  case hooks = "Hooks"

  var id: String { rawValue }

  var title: String {
    switch self {
    case .catalog:
      return "Installed"
    case .capabilities:
      return "Capabilities"
    case .access:
      return "Permissions"
    case .connectors:
      return "Connections"
    case .commands:
      return "Actions"
    case .hooks:
      return "Checks"
    }
  }
}

struct PluginManagerSectionView: View {
  @ObservedObject var viewModel: AppViewModel
  let section: PluginManagerSection

  var body: some View {
    switch section {
    case .catalog:
      PluginCatalogSection(viewModel: viewModel)
    case .capabilities:
      PluginCapabilitiesSection(viewModel: viewModel)
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
    }
  }
}

private struct PluginCapabilitiesSection: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginRegistryDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      if !viewModel.pluginCapabilityPreview().isEmpty {
        Divider()
        ForEach(viewModel.pluginCapabilityPreview()) { capability in
          PluginCapabilityRow(capability: capability)
        }
      }

      if !viewModel.pluginSkillPreview().isEmpty {
        Divider()
        Text("Reviewable guidance")
          .font(.caption.weight(.semibold))
        Text(viewModel.pluginSkillCountSummary())
          .font(.caption2)
          .foregroundColor(.secondary)
          .textSelection(.enabled)

        ForEach(viewModel.pluginSkillPreview()) { skill in
          PluginSkillRow(
            skill: skill,
            canDisablePlugin: viewModel.canDisablePluginGuidance(skill: skill),
            onRevealSource: {
              viewModel.revealPluginSourcePath(skill.sourcePath)
            },
            onDisablePlugin: {
              viewModel.setPluginEnabled(pluginID: skill.pluginID, enabled: false)
            }
          )
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
            canRefresh: viewModel.canRefreshPlugins(),
            refreshDisabledReason: viewModel.pluginRefreshDisabledReason(),
            onRevealManifest: {
              viewModel.revealPluginManifest(pluginID: plugin.id)
            },
            onRefresh: {
              viewModel.refreshPlugins()
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
            connectors: viewModel.pluginCommandConnectors(commandID: command.id),
            canRun: viewModel.canRunPluginCommand(commandID: command.id),
            canRefresh: viewModel.canRefreshPlugins(),
            runDisabledReason: viewModel.pluginCommandRunDisabledReason(commandID: command.id),
            canEnablePlugin: { pluginID in
              viewModel.canSetPluginEnabled(pluginID: pluginID)
            },
            canAuthorizeConnector: { connectorID in
              viewModel.canAuthorizePluginConnector(connectorID: connectorID)
            },
            onRun: {
              viewModel.runPluginCommand(commandID: command.id)
            },
            onRunWithInput: {
              viewModel.runPluginCommandWithInput(commandID: command.id)
            },
            onAuthorizeConnector: { connectorID in
              viewModel.authorizePluginConnector(connectorID: connectorID)
            },
            onEnablePlugin: { pluginID in
              viewModel.setPluginEnabled(pluginID: pluginID, enabled: true)
            },
            onRevealSource: {
              viewModel.revealPluginSourcePath(command.sourcePath)
            },
            onRefresh: {
              viewModel.refreshPlugins()
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
          PluginConnectorRow(
            connector: connector,
            canEnablePlugin: viewModel.canSetPluginEnabled(pluginID: connector.pluginID),
            canAuthorize: viewModel.canAuthorizePluginConnector(connectorID: connector.id),
            canClearCredential: viewModel.canClearPluginConnectorCredential(
              connectorID: connector.id
            ),
            authorizeDisabledReason: viewModel.pluginConnectorAuthorizeDisabledReason(
              connectorID: connector.id
            ),
            clearCredentialDisabledReason: viewModel.pluginConnectorClearDisabledReason(
              connectorID: connector.id
            ),
            onAuthorize: {
              viewModel.authorizePluginConnector(connectorID: connector.id)
            },
            onClearCredential: {
              viewModel.clearPluginConnectorCredential(connectorID: connector.id)
            },
            onEnablePlugin: {
              viewModel.setPluginEnabled(pluginID: connector.pluginID, enabled: true)
            },
            onRevealManifest: {
              viewModel.revealPluginManifest(pluginID: connector.pluginID)
            }
          )
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
          PluginHookRow(
            hook: hook,
            canRefresh: viewModel.canRefreshPlugins(),
            onRevealSource: {
              viewModel.revealPluginSourcePath(hook.sourcePath)
            },
            onRefresh: {
              viewModel.refreshPlugins()
            }
          )
        }
      }
    }
  }
}
