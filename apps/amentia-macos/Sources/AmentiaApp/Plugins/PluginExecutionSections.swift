import SwiftUI

struct PluginCommandsSection: View {
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

struct PluginConnectorsSection: View {
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

struct PluginHooksSection: View {
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
            canDisablePlugin: viewModel.canDisablePluginCheck(hook: hook),
            onRevealSource: {
              viewModel.revealPluginSourcePath(hook.sourcePath)
            },
            onRefresh: {
              viewModel.refreshPlugins()
            },
            onDisablePlugin: {
              viewModel.setPluginEnabled(pluginID: hook.pluginID, enabled: false)
            }
          )
        }
      }
    }
  }
}
