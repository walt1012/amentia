import SwiftUI

struct PluginManagerPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.pluginCountSummary())
        .font(.headline)

      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(viewModel.pluginSurfaceSummary())
          .font(.caption)
          .foregroundColor(.secondary)

        Spacer()

        if viewModel.hasPluginLifecycleOperation() {
          Button("Cancel") {
            viewModel.cancelPluginLifecycleOperation()
          }
          .font(.caption2)
        }

        Button("Refresh") {
          viewModel.refreshPlugins()
        }
        .font(.caption2)
        .disabled(!viewModel.canRefreshPlugins())

        Button("Add Local Connector") {
          viewModel.installPlugin()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canInstallPlugin())
      }

      Picker("View", selection: $viewModel.pluginManagerSection) {
        ForEach(PluginManagerSection.allCases) { section in
          Text(section.title)
            .tag(section)
        }
      }
      .pickerStyle(.menu)

      PluginManagerSectionView(
        viewModel: viewModel,
        section: viewModel.pluginManagerSection
      )
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}
