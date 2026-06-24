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
