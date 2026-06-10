import SwiftUI

struct MemoryPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      Text(viewModel.memoryCountSummary())
        .font(.headline)
      Text(viewModel.memoryDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
      Text(viewModel.memoryLatestSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      TextField("Short title", text: $viewModel.memoryNoteTitle)
        .textFieldStyle(.roundedBorder)

      TextEditor(text: $viewModel.memoryNoteBody)
        .font(.caption)
        .frame(minHeight: 72)
        .overlay(
          RoundedRectangle(cornerRadius: 8, style: .continuous)
            .stroke(Color.secondary.opacity(0.18), lineWidth: 1)
        )

      Button("Save Note") {
        viewModel.saveWorkspaceMemoryNote()
      }
      .buttonStyle(.borderedProminent)
      .disabled(!viewModel.canSaveWorkspaceMemoryNote())
    }
    .softPanel()
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}
