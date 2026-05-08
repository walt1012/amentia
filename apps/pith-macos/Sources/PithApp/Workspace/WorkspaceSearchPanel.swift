import SwiftUI

struct WorkspaceSearchPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(viewModel.workspaceDisplayName())
        .font(.headline)
      Text(viewModel.workspacePath())
        .font(.subheadline)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
      TextField("Search workspace", text: $viewModel.workspaceSearchQuery)
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
          Text("\(match.relativePath):\(match.lineNumber)")
            .font(.caption.weight(.semibold))
            .textSelection(.enabled)
          Text(match.line)
            .font(.caption2)
            .foregroundColor(.secondary)
            .lineLimit(2)
            .textSelection(.enabled)
        }
        .padding(.vertical, 2)
      }

      if let overflow = viewModel.workspaceSearchOverflowSummary() {
        Text(overflow)
          .font(.caption2)
          .foregroundColor(.secondary)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}

private struct EmptyStateHint: View {
  let text: String

  var body: some View {
    Text(text)
      .font(.caption2)
      .foregroundColor(.secondary)
      .padding(8)
      .frame(maxWidth: .infinity, alignment: .leading)
      .background(Color.secondary.opacity(0.08))
      .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
  }
}
