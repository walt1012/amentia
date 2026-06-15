import SwiftUI

struct WorkspaceSearchPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(viewModel.workspaceDisplayName())
          .font(.headline)
        Spacer()
        StatusPill(label: "Local", tone: .neutral)
      }

      Text("Search the open project before asking Pith to act.")
        .font(.caption)
        .foregroundColor(.secondary)

      Text(viewModel.workspacePath())
        .font(.caption2)
        .foregroundColor(.secondary)
        .lineLimit(2)
        .textSelection(.enabled)

      TextField("Search files, symbols, or notes", text: $viewModel.workspaceSearchQuery)
        .textFieldStyle(.roundedBorder)
        .disabled(viewModel.runtimeState != .ready || viewModel.workspace == nil)
        .onSubmit {
          if viewModel.canSearchWorkspace() {
            viewModel.searchWorkspace()
          }
        }

      HStack(spacing: 8) {
        Button("Search") {
          viewModel.searchWorkspace()
        }
        .buttonStyle(.borderedProminent)
        .disabled(!viewModel.canSearchWorkspace())

        Button("Clear") {
          viewModel.clearWorkspaceSearch()
        }
        .buttonStyle(.bordered)
        .disabled(viewModel.workspaceSearchQuery.isEmpty && viewModel.workspaceSearchResults.isEmpty)
      }

      Text(viewModel.workspaceSearchStatus)
        .font(.caption2)
        .foregroundColor(.secondary)

      if viewModel.isWorkspaceSearching {
        ProgressView()
          .progressViewStyle(.linear)
      }

      if let emptyState = viewModel.workspaceSearchEmptyStateSummary() {
        EmptyStateHint(text: emptyState)
      }

      ForEach(viewModel.workspaceSearchResults.prefix(8)) { match in
        VStack(alignment: .leading, spacing: 2) {
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
        .softPanel()
      }

      if let overflow = viewModel.workspaceSearchOverflowSummary() {
        Text(overflow)
          .font(.caption2)
          .foregroundColor(.secondary)
      }
    }
    .softPanel()
    .frame(maxWidth: .infinity, alignment: .leading)
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
