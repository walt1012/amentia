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
      .padding(20)

      Divider()

      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          ForEach(viewModel.timeline) { entry in
            TimelineCard(
              entry: entry,
              isSelected: viewModel.selectedEntryID == entry.id,
              showsApprovalActions: viewModel.isPendingApproval(entry),
              showsPluginEnableAction: viewModel.canEnablePlugin(from: entry),
              showsPluginAuthorizeAction: viewModel.canAuthorizePluginConnector(from: entry),
              showsPluginInputAction: viewModel.canRunPluginCommandWithInput(from: entry),
              showsPluginRetryAction: viewModel.canRetryPluginCommand(from: entry),
              showsPluginFollowUpAction: viewModel.canRunPluginFollowUp(from: entry),
              showsPluginSourceAction: viewModel.canRevealPluginSource(from: entry),
              showsPluginRefreshAction: viewModel.canRefreshPlugins(from: entry),
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
              onRevealPluginSource: {
                viewModel.revealPluginSource(from: entry)
              },
              onRefreshPlugins: {
                Task {
                  await viewModel.refreshPlugins(from: entry)
                }
              }
            )
          }
        }
        .padding(20)
      }

      Divider()
      TimelineComposerView(viewModel: viewModel)
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

}
