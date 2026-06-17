import SwiftUI

struct TimelineComposerView: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ComposerStatusLine(
        summary: viewModel.composerStatusSummary(),
        tone: composerTone,
        isActive: viewModel.showsComposerActivity(),
        canUseComposer: viewModel.canUseComposer()
      )

      ComposerInputBar(
        placeholder: viewModel.composerPlaceholder(),
        draftMessage: $viewModel.draftMessage,
        tone: composerTone,
        iconName: composerIconName,
        canUseComposer: viewModel.canUseComposer(),
        canSend: viewModel.canSendDraftMessage(),
        canCancel: viewModel.canCancelActiveTurn(),
        onSend: {
          viewModel.sendDraftMessage()
        },
        onCancel: {
          viewModel.cancelActiveTurn()
        }
      )
    }
    .frame(maxWidth: 900, alignment: .leading)
    .frame(maxWidth: .infinity, alignment: .center)
    .padding(.horizontal, 24)
    .padding(.vertical, 12)
    .background(AmentiaVisualStyle.paneBackground)
    .animation(AmentiaMotionStyle.quick, value: viewModel.canUseComposer())
    .animation(AmentiaMotionStyle.quick, value: viewModel.canCancelActiveTurn())
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

private struct ComposerStatusLine: View {
  let summary: String
  let tone: StatusTone
  let isActive: Bool
  let canUseComposer: Bool

  var body: some View {
    HStack(spacing: 7) {
      if isActive {
        ProgressView()
          .controlSize(.small)
      } else {
        Circle()
          .fill(tone.color.opacity(canUseComposer ? 0.78 : 0.42))
          .frame(width: 6, height: 6)
      }

      Text(summary)
        .font(.caption)
        .foregroundColor(tone == .danger ? .red : .secondary)
        .lineLimit(1)

      Spacer()

      Text(canUseComposer ? "Return sends; Stop cancels work" : "Complete setup to type")
        .font(.caption2)
        .foregroundColor(.secondary.opacity(canUseComposer ? 0.82 : 0.46))
        .lineLimit(1)
    }
  }
}

private struct ComposerInputBar: View {
  let placeholder: String
  @Binding var draftMessage: String
  let tone: StatusTone
  let iconName: String
  let canUseComposer: Bool
  let canSend: Bool
  let canCancel: Bool
  let onSend: () -> Void
  let onCancel: () -> Void

  var body: some View {
    HStack(alignment: .center, spacing: 10) {
      ZStack {
        Circle()
          .fill(tone.color.opacity(0.11))
          .frame(width: 30, height: 30)
        Image(systemName: iconName)
          .font(.body.weight(.semibold))
          .foregroundColor(tone.color)
      }

      TextField(placeholder, text: $draftMessage)
        .textFieldStyle(.plain)
        .font(.body)
        .disabled(!canUseComposer)
        .onSubmit {
          if canSend {
            onSend()
          }
        }

      Button("Send") {
        onSend()
      }
      .buttonStyle(.borderedProminent)
      .controlSize(.small)
      .disabled(!canSend)

      Button("Stop") {
        onCancel()
      }
      .buttonStyle(.bordered)
      .controlSize(.small)
      .disabled(!canCancel)
    }
    .padding(.horizontal, 12)
    .padding(.vertical, 10)
    .background(AmentiaVisualStyle.panelBackground)
    .overlay(
      RoundedRectangle(cornerRadius: 16, style: .continuous)
        .stroke(tone.color.opacity(canUseComposer ? 0.22 : 0.12), lineWidth: 1)
    )
    .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    .shadow(color: AmentiaVisualStyle.panelShadow, radius: 9, x: 0, y: 3)
  }
}
