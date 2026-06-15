import SwiftUI

struct PluginManagerPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      header
      actionBar
      sectionSwitcher

      PluginManagerSectionView(
        viewModel: viewModel,
        section: viewModel.pluginManagerSection
      )
      .transition(.opacity.combined(with: .move(edge: .top)))
    }
    .frame(maxWidth: .infinity, alignment: .leading)
    .animation(PithMotionStyle.sectionReveal, value: viewModel.pluginManagerSection)
    .animation(PithMotionStyle.quick, value: viewModel.hasPluginLifecycleOperation())
  }

  private var header: some View {
    HStack(alignment: .top, spacing: 10) {
      ZStack {
        Circle()
          .fill(pluginTone.color.opacity(0.11))
          .frame(width: 32, height: 32)
        Image(systemName: "puzzlepiece")
          .font(.body.weight(.semibold))
          .foregroundColor(pluginTone.color)
      }

      VStack(alignment: .leading, spacing: 4) {
        Text(viewModel.pluginCountSummary())
          .font(.headline.weight(.semibold))
          .lineLimit(2)
        Text(viewModel.pluginSurfaceSummary())
          .font(.caption)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }

      Spacer()

      StatusPill(label: pluginStatusLabel, tone: pluginTone)
    }
  }

  private var actionBar: some View {
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 8) {
        Button("Add Plugin") {
          viewModel.installPlugin()
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
        .disabled(!viewModel.canInstallPlugin())

        Button("Refresh") {
          viewModel.refreshPlugins()
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .disabled(!viewModel.canRefreshPlugins())

        if viewModel.hasPluginLifecycleOperation() {
          Button("Cancel") {
            viewModel.cancelPluginLifecycleOperation()
          }
          .buttonStyle(.bordered)
          .controlSize(.small)
        }
      }
    }
  }

  private var sectionSwitcher: some View {
    ScrollView(.horizontal, showsIndicators: false) {
      HStack(spacing: 6) {
        ForEach(PluginManagerSection.allCases) { section in
          Button {
            viewModel.pluginManagerSection = section
          } label: {
            PluginSectionChip(
              title: section.title,
              isSelected: viewModel.pluginManagerSection == section
            )
          }
          .buttonStyle(.plain)
        }
      }
    }
  }

  private var pluginTone: StatusTone {
    viewModel.hasPluginLifecycleOperation() ? .active : .neutral
  }

  private var pluginStatusLabel: String {
    if viewModel.hasPluginLifecycleOperation() {
      return "Updating"
    }
    if viewModel.pluginCountSummary() == "No plugins yet." {
      return "Empty"
    }
    return "Ready"
  }
}

private struct PluginSectionChip: View {
  let title: String
  let isSelected: Bool

  var body: some View {
    Text(title)
      .font(.caption2.weight(isSelected ? .semibold : .medium))
      .foregroundColor(isSelected ? .accentColor : .secondary)
      .padding(.horizontal, 9)
      .padding(.vertical, 5)
      .background(background)
      .overlay(
        Capsule()
          .stroke(border, lineWidth: 1)
      )
      .clipShape(Capsule())
  }

  private var background: Color {
    isSelected ? Color.accentColor.opacity(0.10) : PithVisualStyle.panelBackground
  }

  private var border: Color {
    isSelected ? Color.accentColor.opacity(0.22) : PithVisualStyle.panelBorder
  }
}
