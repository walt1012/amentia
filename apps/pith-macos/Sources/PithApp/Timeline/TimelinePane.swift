import SwiftUI

struct TimelinePane: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      VStack(alignment: .leading, spacing: 10) {
        header
        setupSurface
        readinessSteps
      }
      .padding(20)

      Divider()

      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          ForEach(viewModel.timeline) { entry in
            TimelineCard(
              entry: entry,
              isSelected: viewModel.selectedEntryID == entry.id,
              showsApprovalActions: viewModel.isPendingApproval(entry),
              onSelect: {
                viewModel.selectTimelineEntry(id: entry.id)
              },
              onApprove: {
                guard let approvalID = viewModel.approvalID(for: entry) else {
                  return
                }
                viewModel.respondToApproval(approvalID: approvalID, decision: "approved")
              },
              onDeny: {
                guard let approvalID = viewModel.approvalID(for: entry) else {
                  return
                }
                viewModel.respondToApproval(approvalID: approvalID, decision: "denied")
              }
            )
          }
        }
        .padding(20)
      }

      Divider()
      composer
    }
    .frame(minWidth: 520)
  }

  private var header: some View {
    HStack {
      VStack(alignment: .leading, spacing: 4) {
        Text("Timeline")
          .font(.title2.weight(.semibold))
        Text(viewModel.workspaceDisplayName())
          .font(.caption)
          .foregroundColor(.secondary)
      }
      Spacer()
      VStack(alignment: .trailing, spacing: 6) {
        HStack(spacing: 6) {
          if viewModel.showsRuntimeActivity() {
            ProgressView()
              .controlSize(.small)
          }
          StatusPill(
            label: viewModel.runtimeState.rawValue.capitalized,
            tone: viewModel.runtimeStatusTone()
          )
        }
        Text(viewModel.runtimeStatusSummary())
          .font(.caption2)
          .foregroundColor(.secondary)
          .multilineTextAlignment(.trailing)
        if viewModel.shouldShowRuntimeHeaderDetail() {
          Text(viewModel.runtimeDetail)
            .font(.caption2)
            .foregroundColor(.secondary)
            .lineLimit(2)
            .multilineTextAlignment(.trailing)
        }
        if let actionTitle = viewModel.runtimePrimaryActionTitle() {
          Button(actionTitle) {
            viewModel.runRuntimePrimaryAction()
          }
          .buttonStyle(.bordered)
          .disabled(!viewModel.canRunRuntimePrimaryAction())
        }
      }
    }
  }

  @ViewBuilder
  private var setupSurface: some View {
    if viewModel.shouldShowSetupProgress() {
      SetupProgressView(
        summary: viewModel.setupProgressSummary(),
        detail: viewModel.setupProgressDetail(),
        value: viewModel.setupProgressValue(),
        tone: viewModel.setupProgressTone()
      )
    }

    if viewModel.shouldShowSetupCallout() {
      if viewModel.shouldShowSetupModelChoice() {
        SetupModelChooser(
          models: viewModel.localModels,
          selectedModelID: $viewModel.selectedSetupModelID,
          defaultModelID: viewModel.setupDefaultModelID(),
          detail: viewModel.setupModelChoiceDetail(),
          isDisabled: !viewModel.canChangeSetupModelChoice(),
          actionTitle: viewModel.modelSetupCalloutActionTitle(),
          canRunAction: viewModel.canRunModelSetupCalloutAction(),
          onAction: {
            viewModel.runModelSetupCalloutAction()
          }
        )
      }

      SetupCallout(
        title: viewModel.setupCalloutTitle(),
        summary: viewModel.setupCalloutSummary(),
        detail: viewModel.setupCalloutDetail(),
        tone: viewModel.setupCalloutTone(),
        actionTitle: viewModel.shouldShowSetupModelChoice() ? nil : viewModel.setupCalloutActionTitle(),
        canRunAction: viewModel.shouldShowSetupModelChoice() ? false : viewModel.canRunSetupCalloutAction(),
        secondaryActionTitle: viewModel.setupCalloutSecondaryActionTitle(),
        canRunSecondaryAction: viewModel.canRunSetupCalloutSecondaryAction(),
        onAction: {
          viewModel.runSetupCalloutAction()
        },
        onSecondaryAction: {
          viewModel.runSetupCalloutSecondaryAction()
        }
      )
    } else if viewModel.shouldShowFirstRequestCallout() {
      SetupCallout(
        title: viewModel.firstRequestCalloutTitle(),
        summary: viewModel.firstRequestCalloutSummary(),
        detail: viewModel.firstRequestCalloutDetail(),
        tone: .ready,
        actionTitle: viewModel.firstRequestCalloutActionTitle(),
        canRunAction: viewModel.canRunFirstRequestCalloutAction(),
        secondaryActionTitle: viewModel.firstRequestCalloutSecondaryActionTitle(),
        canRunSecondaryAction: viewModel.canRunFirstRequestCalloutSecondaryAction(),
        onAction: {
          viewModel.runFirstRequestCalloutAction()
        },
        onSecondaryAction: {
          viewModel.runFirstRequestCalloutSecondaryAction()
        }
      )
    }
  }

  @ViewBuilder
  private var readinessSteps: some View {
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

  private var composer: some View {
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
