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

        Button("Refresh") {
          Task {
            await viewModel.refreshPlugins()
          }
        }
        .font(.caption2)
        .disabled(!viewModel.canRefreshPlugins())

        Button("Install Local Plugin") {
          viewModel.installPlugin()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canInstallPlugin())
      }

      Picker("Surface", selection: $viewModel.pluginManagerSection) {
        ForEach(PluginManagerSection.allCases) { section in
          Text(section.rawValue)
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
