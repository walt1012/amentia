import SwiftUI

struct ContentView: View {
  @ObservedObject var viewModel: AppViewModel
  @AppStorage("pith.inspector.workspaceExpanded") private var workspaceExpanded = false
  @AppStorage("pith.inspector.localModelExpanded") private var localModelExpanded = false
  @AppStorage("pith.inspector.memoryExpanded") private var memoryExpanded = false
  @AppStorage("pith.inspector.pluginManagerExpanded") private var pluginManagerExpanded = false
  @AppStorage("pith.inspector.threadExpanded") private var threadExpanded = false
  @AppStorage("pith.inspector.selectedItemExpanded") private var selectedItemExpanded = false
  @AppStorage("pith.inspector.selectedMemoryExpanded") private var selectedMemoryExpanded = false
  @AppStorage("pith.inspector.selectedAttributesExpanded") private var selectedAttributesExpanded = false

  var body: some View {
    NavigationView {
      sidebar
      TimelinePane(viewModel: viewModel)
      inspector
    }
    .toolbar {
      ToolbarItem {
        Button("Open Workspace") {
          viewModel.openWorkspace()
        }
        .disabled(!viewModel.canOpenWorkspace())
      }

      ToolbarItem {
        Button("New Thread") {
          viewModel.createThread()
        }
        .disabled(!viewModel.canCreateThread())
      }

      ToolbarItem(placement: .primaryAction) {
        if viewModel.shouldShowRuntimeToolbarAction() {
          Button(viewModel.runtimeLaunchButtonTitle()) {
            viewModel.launchRuntime()
          }
          .disabled(!viewModel.canLaunchRuntime())
        }
      }
    }
  }

  private var sidebar: some View {
    List(selection: Binding(get: { viewModel.selectedThreadID }, set: { viewModel.selectThread(id: $0) })) {
      Section("Threads") {
        ForEach(viewModel.threads) { thread in
          VStack(alignment: .leading, spacing: 4) {
            Text(thread.title)
              .font(.headline)
            Text(thread.preview)
              .font(.caption)
              .foregroundColor(.secondary)
          }
          .padding(.vertical, 4)
          .tag(thread.id)
        }
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
  }

  private var inspector: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 16) {
        Text("Inspector")
          .font(.title3.weight(.semibold))

        InspectorSessionCard(
          title: viewModel.inspectorSessionTitle(),
          detail: viewModel.inspectorSessionDetail(),
          meta: viewModel.inspectorSessionMetaSummary(),
          tone: viewModel.runtimeStatusTone()
        )

        DisclosureGroup("Workspace Search", isExpanded: $workspaceExpanded) {
          WorkspaceSearchPanel(viewModel: viewModel)
        }

        DisclosureGroup("Local Model", isExpanded: $localModelExpanded) {
          LocalModelPanel(viewModel: viewModel)
        }

        DisclosureGroup("Memory", isExpanded: $memoryExpanded) {
          MemoryPanel(viewModel: viewModel)
        }

        DisclosureGroup("Plugin Manager", isExpanded: $pluginManagerExpanded) {
          PluginManagerPanel(viewModel: viewModel)
        }

        DisclosureGroup("Thread", isExpanded: $threadExpanded) {
          VStack(alignment: .leading, spacing: 8) {
            Text(viewModel.selectedThreadTitle())
              .font(.headline)
            Text(viewModel.selectedThreadPreview())
              .font(.subheadline)
              .foregroundColor(.secondary)
          }
          .frame(maxWidth: .infinity, alignment: .leading)
        }

        if viewModel.shouldShowSelectedEntryInspector() {
          DisclosureGroup("Selected Item", isExpanded: $selectedItemExpanded) {
            VStack(alignment: .leading, spacing: 8) {
              Text(viewModel.selectedEntryTitle())
                .font(.headline)
              Text(viewModel.selectedEntryBody())
                .font(.subheadline)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
              DisclosureGroup("Attributes", isExpanded: $selectedAttributesExpanded) {
                Text(viewModel.selectedEntryMetadata())
                  .font(.caption)
                  .foregroundColor(.secondary)
                  .textSelection(.enabled)
                  .frame(maxWidth: .infinity, alignment: .leading)
              }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
        }

        if let diffSummary = viewModel.selectedDiffSummary() {
          GroupBox("Diff Detail") {
            DiffDetailView(summary: diffSummary, lines: viewModel.selectedDiffLines())
          }
        }

        if let memorySummary = viewModel.selectedEntryMemorySummary() {
          DisclosureGroup("Selected Memory Context", isExpanded: $selectedMemoryExpanded) {
            Text(memorySummary)
              .font(.caption)
              .foregroundColor(.secondary)
              .textSelection(.enabled)
              .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
      }
      .padding(20)
    }
    .frame(minWidth: 280)
  }
}

private struct InspectorSessionCard: View {
  let title: String
  let detail: String
  let meta: String
  let tone: StatusTone

  var body: some View {
    VStack(alignment: .leading, spacing: 6) {
      HStack(spacing: 8) {
        Circle()
          .fill(tone.color)
          .frame(width: 8, height: 8)
        Text(title)
          .font(.headline)
        Spacer()
      }

      Text(detail)
        .font(.caption)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)

      Text(meta)
        .font(.caption2.weight(.medium))
        .foregroundColor(tone.color)
        .lineLimit(2)
    }
    .padding(10)
    .frame(maxWidth: .infinity, alignment: .leading)
    .background(tone.color.opacity(0.08))
    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
  }
}

struct SettingsView: View {
  var body: some View {
    Form {
      Section("Model") {
        Text("First-use catalog: LFM2.5-350M default, Granite 4.0-H-350M recommended alternative")
      }

      Section("Platform") {
        Text("Target: macOS 12+ on Intel")
      }
    }
    .padding(20)
    .frame(width: 420)
  }
}
