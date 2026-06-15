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
        Text("Project")
          .font(.title3.weight(.semibold))

        InspectorSessionCard(
          title: viewModel.inspectorSessionTitle(),
          detail: viewModel.inspectorSessionDetail(),
          meta: viewModel.inspectorSessionMetaSummary(),
          tone: viewModel.runtimeStatusTone()
        )

        DisclosureGroup("Search Project", isExpanded: $workspaceExpanded) {
          WorkspaceSearchPanel(viewModel: viewModel)
        }
        .inspectorSectionCard()

        DisclosureGroup("Local Models", isExpanded: $localModelExpanded) {
          LocalModelPanel(viewModel: viewModel)
        }
        .inspectorSectionCard()

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

            DisclosureGroup("Plugins", isExpanded: $pluginManagerExpanded) {
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
        .inspectorSectionCard()
      }
      .padding(20)
      .animation(PithMotionStyle.sectionReveal, value: workspaceExpanded)
      .animation(PithMotionStyle.sectionReveal, value: localModelExpanded)
      .animation(PithMotionStyle.sectionReveal, value: selectedItemExpanded)
      .animation(PithMotionStyle.sectionReveal, value: selectedContextExpanded)
      .animation(PithMotionStyle.sectionReveal, value: selectedPluginExpanded)
      .animation(PithMotionStyle.sectionReveal, value: selectedSandboxExpanded)
      .animation(PithMotionStyle.sectionReveal, value: advancedExpanded)
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
          DisclosureGroup("Support Details", isExpanded: $selectedAttributesExpanded) {
            VStack(alignment: .leading, spacing: 6) {
              Text("Event details for troubleshooting. Most daily work does not need this.")
                .font(.caption2)
                .foregroundColor(.secondary)
              Text(viewModel.selectedEntryMetadata())
                .font(.caption)
                .foregroundColor(.secondary)
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      .inspectorSectionCard()
      .transition(.opacity.combined(with: .move(edge: .top)))
    }
  }

  @ViewBuilder
  private var diffDetailSection: some View {
    if let diffSummary = viewModel.selectedDiffSummary() {
      GroupBox("Change Detail") {
        DiffDetailView(summary: diffSummary, lines: viewModel.selectedDiffLines())
      }
      .inspectorSectionCard()
      .transition(.opacity.combined(with: .move(edge: .top)))
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
      .inspectorSectionCard()
      .transition(.opacity.combined(with: .move(edge: .top)))
    }
  }

  @ViewBuilder
  private var selectedPluginSection: some View {
    if let pluginSummary = viewModel.selectedEntryPluginSummary() {
      DisclosureGroup("Connection Proof", isExpanded: $selectedPluginExpanded) {
        Text(pluginSummary)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
      }
      .inspectorSectionCard()
      .transition(.opacity.combined(with: .move(edge: .top)))
    }
  }

  @ViewBuilder
  private var selectedSandboxSection: some View {
    if let sandboxSummary = viewModel.selectedEntrySandboxSummary() {
      DisclosureGroup("Safety Proof", isExpanded: $selectedSandboxExpanded) {
        Text(sandboxSummary)
          .font(.caption)
          .foregroundColor(.secondary)
          .textSelection(.enabled)
          .frame(maxWidth: .infinity, alignment: .leading)
      }
      .inspectorSectionCard()
      .transition(.opacity.combined(with: .move(edge: .top)))
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

private struct InspectorSectionCard: ViewModifier {
  func body(content: Content) -> some View {
    content
      .padding(12)
      .frame(maxWidth: .infinity, alignment: .leading)
      .background(PithVisualStyle.panelBackground)
      .overlay(
        RoundedRectangle(cornerRadius: 12, style: .continuous)
          .stroke(PithVisualStyle.panelBorder, lineWidth: 1)
      )
      .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
      .shadow(color: PithVisualStyle.panelShadow, radius: 6, x: 0, y: 2)
  }
}

private extension View {
  func inspectorSectionCard() -> some View {
    modifier(InspectorSectionCard())
  }
}
