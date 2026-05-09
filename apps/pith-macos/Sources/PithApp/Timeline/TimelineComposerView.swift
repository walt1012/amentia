import SwiftUI

struct TimelineComposerView: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      HStack(alignment: .bottom, spacing: 12) {
        TextField(viewModel.composerPlaceholder(), text: $viewModel.draftMessage)
          .textFieldStyle(.roundedBorder)
          .disabled(!viewModel.canUseComposer())
          .onSubmit {
            if viewModel.canSendDraftMessage() {
              viewModel.sendDraftMessage()
            }
          }

        Button("Send") {
          viewModel.sendDraftMessage()
        }
        .buttonStyle(.borderedProminent)
        .disabled(!viewModel.canSendDraftMessage())

        Button("Cancel Execution") {
          viewModel.cancelActiveTurn()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canCancelActiveTurn())
      }

      HStack(spacing: 6) {
        if viewModel.showsComposerActivity() {
          ProgressView()
            .controlSize(.small)
        }
        Text(viewModel.composerStatusSummary())
          .font(.caption2)
          .foregroundColor(viewModel.runtimeState == .failed ? .red : .secondary)
      }
    }
    .padding(20)
  }
}
