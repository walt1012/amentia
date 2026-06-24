import SwiftUI

struct PluginCatalogSection: View {
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

struct PluginCapabilitiesSection: View {
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

struct PluginAccessSection: View {
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
