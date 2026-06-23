import SwiftUI

struct TimelinePane: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      VStack(alignment: .leading, spacing: 10) {
        header
        setupSurface
        TimelineReadinessStrip(viewModel: viewModel)
      }
      .frame(maxWidth: 900, alignment: .leading)
      .frame(maxWidth: .infinity, alignment: .center)
      .padding(.horizontal, 24)
      .padding(.vertical, 18)
      .animation(AmentiaMotionStyle.sectionReveal, value: runtimeStateLabel)

      Divider()

      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          ForEach(viewModel.timeline) { entry in
            TimelineCard(
              entry: entry,
              isSelected: viewModel.selectedEntryID == entry.id,
              receiptSummary: viewModel.timelineReceiptSummary(from: entry),
              approvalOutcomeSummary: viewModel.timelineApprovalOutcomeSummary(from: entry),
              externalActionTitle: viewModel.timelineExternalAction(from: entry)?.title,
              externalCopyActionTitle: viewModel.timelineExternalAction(from: entry)?.copyTitle,
              showsApprovalActions: viewModel.isPendingApproval(entry),
              showsPluginEnableAction: viewModel.canEnablePlugin(from: entry),
              showsPluginAuthorizeAction: viewModel.canAuthorizePluginConnector(from: entry),
              showsPluginInputAction: viewModel.canRunPluginCommandWithInput(from: entry),
              showsPluginRetryAction: viewModel.canRetryPluginCommand(from: entry),
              showsPluginFollowUpAction: viewModel.canRunPluginFollowUp(from: entry),
              showsPluginGuidanceReviewAction: viewModel.canReviewPluginGuidance(from: entry),
              showsPluginGuidanceDisableAction: viewModel.canDisablePluginGuidance(from: entry),
              showsPluginSourceAction: viewModel.canRevealPluginSource(from: entry),
              showsPluginRefreshAction: viewModel.canRefreshPlugins(from: entry),
              localExecutionRecoveryTitle: viewModel.localExecutionRecoveryAction(from: entry)?.title,
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
              },
              onEnablePlugin: {
                viewModel.enablePlugin(from: entry)
              },
              onAuthorizePluginConnector: {
                viewModel.authorizePluginConnector(from: entry)
              },
              onRunPluginCommandWithInput: {
                viewModel.runPluginCommandWithInput(from: entry)
              },
              onRetry: {
                viewModel.retryPluginCommand(from: entry)
              },
              onRunPluginFollowUp: {
                viewModel.runPluginFollowUp(from: entry)
              },
              onReviewPluginGuidance: {
                viewModel.reviewPluginGuidance(from: entry)
              },
              onDisablePluginGuidance: {
                viewModel.disablePluginGuidance(from: entry)
              },
              onRevealPluginSource: {
                viewModel.revealPluginSource(from: entry)
              },
              onRefreshPlugins: {
                Task {
                  await viewModel.refreshPlugins(from: entry)
                }
              },
              onRecoverLocalExecution: {
                viewModel.recoverLocalExecutionMode(from: entry)
              },
              onOpenExternalAction: {
                viewModel.openTimelineExternalAction(from: entry)
              },
              onCopyExternalAction: {
                viewModel.copyTimelineExternalActionURL(from: entry)
              }
            )
            .transition(
              .asymmetric(
                insertion: .opacity.combined(with: .move(edge: .bottom)),
                removal: .opacity
              )
            )
          }
        }
        .frame(maxWidth: 860, alignment: .leading)
        .frame(maxWidth: .infinity, alignment: .center)
        .padding(.horizontal, 24)
        .padding(.vertical, 22)
        .animation(AmentiaMotionStyle.timelineReveal, value: viewModel.timeline.count)
        .animation(AmentiaMotionStyle.quick, value: viewModel.selectedEntryID)
      }

      Divider()
      TimelineComposerView(viewModel: viewModel)
    }
    .frame(minWidth: 520)
    .background(AmentiaVisualStyle.paneBackground)
  }

  private var header: some View {
    HStack {
      VStack(alignment: .leading, spacing: 4) {
        Text("Cowork")
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
            label: runtimeStateLabel,
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

  private var runtimeStateLabel: String {
    switch viewModel.runtimeState {
    case .disconnected:
      return "Start"
    case .launching:
      return "Starting"
    case .failed:
      return "Needs Help"
    case .ready:
      return "Ready"
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
      .transition(.opacity.combined(with: .move(edge: .top)))
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
        .transition(.opacity.combined(with: .move(edge: .top)))
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
      .transition(.opacity.combined(with: .move(edge: .top)))
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
      .transition(.opacity.combined(with: .move(edge: .top)))
    }
  }

}
