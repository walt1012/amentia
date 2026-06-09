import SwiftUI

struct InspectorPane: View {
  @ObservedObject var viewModel: AppViewModel
  @AppStorage("pith.inspector.workspaceExpanded") private var workspaceExpanded = false
  @AppStorage("pith.inspector.localModelExpanded") private var localModelExpanded = false
  @AppStorage("pith.inspector.advancedExpanded") private var advancedExpanded = false
  @AppStorage("pith.inspector.memoryExpanded") private var memoryExpanded = false
  @AppStorage("pith.inspector.pluginManagerExpanded") private var pluginManagerExpanded = false
  @AppStorage("pith.inspector.threadExpanded") private var threadExpanded = false
  @AppStorage("pith.inspector.selectedItemExpanded") private var selectedItemExpanded = false
  @AppStorage("pith.inspector.selectedPluginExpanded") private var selectedPluginExpanded = false
  @AppStorage("pith.inspector.selectedContextExpanded") private var selectedContextExpanded = false
  @AppStorage("pith.inspector.selectedSandboxExpanded") private var selectedSandboxExpanded = false
  @AppStorage("pith.inspector.selectedAttributesExpanded") private var selectedAttributesExpanded = false

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 16) {
        Text("Details")
          .font(.title3.weight(.semibold))

        InspectorSessionCard(
          title: viewModel.inspectorSessionTitle(),
          detail: viewModel.inspectorSessionDetail(),
          meta: viewModel.inspectorSessionMetaSummary(),
          tone: viewModel.runtimeStatusTone()
        )

        DisclosureGroup("Search Workspace", isExpanded: $workspaceExpanded) {
          WorkspaceSearchPanel(viewModel: viewModel)
        }

        DisclosureGroup("Local Engine", isExpanded: $localModelExpanded) {
          LocalModelPanel(viewModel: viewModel)
        }

        selectedItemSection
        diffDetailSection
        selectedContextSection
        selectedPluginSection
        selectedSandboxSection

        DisclosureGroup("Advanced", isExpanded: $advancedExpanded) {
          VStack(alignment: .leading, spacing: 14) {
            DisclosureGroup("Memory", isExpanded: $memoryExpanded) {
              MemoryPanel(viewModel: viewModel)
            }

            DisclosureGroup("Connectors", isExpanded: $pluginManagerExpanded) {
              PluginManagerPanel(viewModel: viewModel)
            }

            DisclosureGroup("Conversation", isExpanded: $threadExpanded) {
              VStack(alignment: .leading, spacing: 8) {
                Text(viewModel.selectedThreadTitle())
                  .font(.headline)
                Text(viewModel.selectedThreadPreview())
                  .font(.subheadline)
                  .foregroundColor(.secondary)
              }
              .frame(maxWidth: .infinity, alignment: .leading)
            }
          }
          .frame(maxWidth: .infinity, alignment: .leading)
        }
      }
      .padding(20)
    }
    .frame(minWidth: 280)
    .background(PithVisualStyle.inspectorBackground)
  }

  @ViewBuilder
  private var selectedItemSection: some View {
    if viewModel.shouldShowSelectedEntryInspector() {
      DisclosureGroup("Selection", isExpanded: $selectedItemExpanded) {
        VStack(alignment: .leading, spacing: 8) {
          Text(viewModel.selectedEntryTitle())
            .font(.headline)
          Text(viewModel.selectedEntryBody())
            .font(.subheadline)
            .foregroundColor(.secondary)
            .textSelection(.enabled)
          DisclosureGroup("Details", isExpanded: $selectedAttributesExpanded) {
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
  }

  @ViewBuilder
  private var diffDetailSection: some View {
    if let diffSummary = viewModel.selectedDiffSummary() {
      GroupBox("Change Detail") {
        DiffDetailView(summary: diffSummary, lines: viewModel.selectedDiffLines())
      }
    }
  }

  @ViewBuilder
  private var selectedContextSection: some View {
    let sections = viewModel.selectedEntryContextReceiptSections()
    if !sections.isEmpty {
      DisclosureGroup("Context", isExpanded: $selectedContextExpanded) {
        VStack(alignment: .leading, spacing: 10) {
          ForEach(sections) { section in
            VStack(alignment: .leading, spacing: 4) {
              Text(section.title)
                .font(.caption.weight(.semibold))
              Text(section.body)
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
  }

  @ViewBuilder
  private var selectedPluginSection: some View {
    if let pluginSummary = viewModel.selectedEntryPluginSummary() {
      DisclosureGroup("Connector Details", isExpanded: $selectedPluginExpanded) {
        Text(pluginSummary)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
  }

  @ViewBuilder
  private var selectedSandboxSection: some View {
    if let sandboxSummary = viewModel.selectedEntrySandboxSummary() {
      DisclosureGroup("Safety Details", isExpanded: $selectedSandboxExpanded) {
        Text(sandboxSummary)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
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
    .background(PithVisualStyle.panelBackground)
    .overlay(
      RoundedRectangle(cornerRadius: 10, style: .continuous)
        .stroke(tone.color.opacity(0.18), lineWidth: 1)
    )
    .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
  }
}
