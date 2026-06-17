import SwiftUI

struct WorkspaceSearchPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        VStack(alignment: .leading, spacing: 3) {
          Text(viewModel.workspaceDisplayName())
            .font(.headline.weight(.semibold))
            .lineLimit(1)
          Text("Search the open project before asking Amentia to act.")
            .font(.caption2)
            .foregroundColor(.secondary)
        }
        Spacer()
        StatusPill(label: "Local", tone: .neutral)
      }

      Text(viewModel.workspaceSearchScopeSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
        .lineLimit(2)

      HStack(alignment: .center, spacing: 8) {
        Image(systemName: "magnifyingglass")
          .font(.caption.weight(.semibold))
          .foregroundColor(.secondary)
          .frame(width: 14)

        TextField("Search files, symbols, or notes", text: $viewModel.workspaceSearchQuery)
          .textFieldStyle(.plain)
          .disabled(viewModel.runtimeState != .ready || viewModel.workspace == nil)
          .onSubmit {
            if viewModel.canSearchWorkspace() {
              viewModel.searchWorkspace()
            }
          }

        Button("Search") {
          viewModel.searchWorkspace()
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.small)
        .disabled(!viewModel.canSearchWorkspace())

        Button("Clear") {
          viewModel.clearWorkspaceSearch()
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .disabled(viewModel.workspaceSearchQuery.isEmpty && viewModel.workspaceSearchResults.isEmpty)
      }
      .padding(.horizontal, 10)
      .padding(.vertical, 8)
      .background(AmentiaVisualStyle.panelBackground)
      .overlay(
        RoundedRectangle(cornerRadius: 12, style: .continuous)
          .stroke(AmentiaVisualStyle.panelBorder, lineWidth: 1)
      )
      .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))

      HStack(spacing: 6) {
        if viewModel.isWorkspaceSearching {
          ProgressView()
            .controlSize(.small)
        } else {
          Circle()
            .fill(workspaceSearchTone.color.opacity(0.75))
            .frame(width: 6, height: 6)
        }
        Text(viewModel.workspaceSearchStatus)
          .font(.caption2)
          .foregroundColor(workspaceSearchTone == .warning ? .orange : .secondary)
          .lineLimit(2)
      }

      if let emptyState = viewModel.workspaceSearchEmptyStateSummary() {
        EmptyStateHint(text: emptyState)
      }

      ForEach(viewModel.workspaceSearchResults.prefix(8)) { match in
        WorkspaceSearchResultRow(match: match)
      }

      if let overflow = viewModel.workspaceSearchOverflowSummary() {
        Text(overflow)
          .font(.caption2)
          .foregroundColor(.secondary)
      }
    }
    .softPanel()
    .frame(maxWidth: .infinity, alignment: .leading)
    .animation(AmentiaMotionStyle.sectionReveal, value: viewModel.isWorkspaceSearching)
    .animation(AmentiaMotionStyle.sectionReveal, value: viewModel.workspaceSearchResults.count)
  }

  private var workspaceSearchTone: StatusTone {
    if viewModel.isWorkspaceSearching {
      return .active
    }
    if viewModel.workspace == nil || viewModel.runtimeState == .failed {
      return .warning
    }
    return .neutral
  }
}

private struct EmptyStateHint: View {
  let text: String

  var body: some View {
    Text(text)
      .font(.caption2)
      .foregroundColor(.secondary)
      .frame(maxWidth: .infinity, alignment: .leading)
      .softPanel()
  }
}

private struct WorkspaceSearchResultRow: View {
  let match: WorkspaceSearchMatchSummary

  var body: some View {
    HStack(alignment: .top, spacing: 8) {
      Image(systemName: "doc.text")
        .font(.caption)
        .foregroundColor(.secondary)
        .frame(width: 16)

      VStack(alignment: .leading, spacing: 3) {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
          Text(match.relativePath)
            .font(.caption.weight(.semibold))
            .lineLimit(1)
            .textSelection(.enabled)
          Spacer()
          Text("line \(match.lineNumber)")
            .font(.caption2.weight(.medium))
            .foregroundColor(.secondary)
        }
        Text(match.line)
          .font(.caption2)
          .foregroundColor(.secondary)
          .lineLimit(2)
          .textSelection(.enabled)
      }
    }
    .softPanel()
  }
}
