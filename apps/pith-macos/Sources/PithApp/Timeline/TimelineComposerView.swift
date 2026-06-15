import SwiftUI

struct TimelineComposerView: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .center, spacing: 10) {
        Image(systemName: composerIconName)
          .font(.body.weight(.medium))
          .foregroundColor(composerTone.color)
          .frame(width: 18)

        TextField(viewModel.composerPlaceholder(), text: $viewModel.draftMessage)
          .textFieldStyle(.plain)
          .font(.body)
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
        .controlSize(.small)
        .disabled(!viewModel.canSendDraftMessage())

        Button("Stop") {
          viewModel.cancelActiveTurn()
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .disabled(!viewModel.canCancelActiveTurn())
      }
      .padding(.horizontal, 12)
      .padding(.vertical, 10)
      .background(PithVisualStyle.panelBackground)
      .overlay(
        RoundedRectangle(cornerRadius: 14, style: .continuous)
          .stroke(composerTone.color.opacity(0.20), lineWidth: 1)
      )
      .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))

      HStack(spacing: 6) {
        if viewModel.showsComposerActivity() {
          ProgressView()
            .controlSize(.small)
        } else {
          Circle()
            .fill(composerTone.color.opacity(0.75))
            .frame(width: 6, height: 6)
        }
        Text(viewModel.composerStatusSummary())
          .font(.caption2)
          .foregroundColor(composerTone == .danger ? .red : .secondary)
          .lineLimit(1)

        Spacer()

        Text("Return to send")
          .font(.caption2)
          .foregroundColor(.secondary.opacity(viewModel.canUseComposer() ? 0.85 : 0.45))
      }
    }
    .frame(maxWidth: 900, alignment: .leading)
    .frame(maxWidth: .infinity, alignment: .center)
    .padding(.horizontal, 24)
    .padding(.vertical, 14)
    .background(PithVisualStyle.paneBackground)
    .animation(PithMotionStyle.quick, value: viewModel.canUseComposer())
    .animation(PithMotionStyle.quick, value: viewModel.canCancelActiveTurn())
  }

  private var composerTone: StatusTone {
    if viewModel.runtimeState == .failed {
      return .danger
    }
    if viewModel.showsComposerActivity() {
      return .active
    }
    if viewModel.canUseComposer() {
      return .ready
    }
    return .neutral
  }

  private var composerIconName: String {
    if viewModel.runtimeState == .failed {
      return "exclamationmark.triangle"
    }
    if viewModel.showsComposerActivity() {
      return "sparkles"
    }
    if viewModel.canUseComposer() {
      return "paperplane"
    }
    return "lock"
  }
}
