import SwiftUI

struct TimelineReadinessStrip: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    if viewModel.shouldShowReadinessSteps() {
      HStack(spacing: 8) {
        ForEach(viewModel.runtimeReadinessSteps()) { step in
          ReadinessChip(
            step: step,
            actionTitle: viewModel.readinessStepActionTitle(step),
            canRunAction: viewModel.canRunReadinessStepAction(step),
            onAction: {
              viewModel.runReadinessStepAction(step)
            }
          )
        }
        Spacer()
      }
    }
  }
}

private struct ReadinessChip: View {
  let step: ReadinessStepSummary
  let actionTitle: String?
  let canRunAction: Bool
  let onAction: () -> Void

  var body: some View {
    if actionTitle != nil {
      Button(action: onAction) {
        content
      }
      .buttonStyle(.plain)
      .disabled(!canRunAction)
      .help(actionTitle.map { "\($0) \(step.label)" } ?? "\(step.label): \(step.detail)")
    } else {
      content
    }
  }

  private var content: some View {
    HStack(spacing: 5) {
      Text(step.label)
        .font(.caption2.weight(.medium))
        .foregroundColor(.secondary)

      Text(step.detail)
        .font(.caption2.weight(.semibold))
        .foregroundColor(step.tone.color)
        .lineLimit(1)
        .truncationMode(.tail)
        .frame(maxWidth: 150, alignment: .leading)

      if let actionTitle {
        Text(actionTitle)
          .font(.caption2.weight(.bold))
          .foregroundColor(canRunAction ? step.tone.color : .secondary)
      }
    }
    .padding(.horizontal, 8)
    .padding(.vertical, 5)
    .background(step.tone.color.opacity(actionTitle == nil ? 0.10 : 0.16))
    .clipShape(Capsule())
  }
}
