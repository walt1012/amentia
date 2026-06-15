import SwiftUI

struct TimelineReadinessStrip: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    if viewModel.shouldShowReadinessSteps() {
      ScrollView(.horizontal, showsIndicators: false) {
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
        }
      }
      .transition(.opacity.combined(with: .move(edge: .top)))
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
      Circle()
        .fill(step.tone.color.opacity(0.85))
        .frame(width: 6, height: 6)

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
    .padding(.horizontal, 9)
    .padding(.vertical, 5)
    .background(step.tone.color.opacity(actionTitle == nil ? 0.10 : 0.16))
    .overlay(
      Capsule()
        .stroke(step.tone.color.opacity(actionTitle == nil ? 0.12 : 0.22), lineWidth: 1)
    )
    .clipShape(Capsule())
  }
}
