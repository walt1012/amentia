import SwiftUI

struct LocalModelPanel: View {
  @ObservedObject var viewModel: AppViewModel
  @AppStorage("pith.inspector.modelChooserExpanded") private var modelChooserExpanded = false
  @AppStorage("pith.inspector.modelTroubleshootingExpanded") private var modelTroubleshootingExpanded = false

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(viewModel.modelDisplayName())
        .font(.headline)
      Text(viewModel.modelStatusSummary())
        .font(.subheadline)
        .foregroundColor(.secondary)

      HStack(spacing: 6) {
        if viewModel.showsModelActivity() {
          ProgressView()
            .controlSize(.small)
        }
        Text(viewModel.modelActionSummary())
          .font(.caption)
          .foregroundColor(viewModel.isModelActionBlocking() ? .orange : .secondary)
      }

      Text(viewModel.modelRecoverySummary())
        .font(.caption2)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)

      if viewModel.shouldShowModelDownloadProgress() {
        ModelDownloadProgressView(
          value: viewModel.modelDownloadProgressValue(),
          summary: viewModel.modelDownloadProgressSummary()
        )
      }

      if let primaryActionTitle = viewModel.localModelPrimaryActionTitle() {
        HStack(spacing: 8) {
          Button(primaryActionTitle) {
            viewModel.runLocalModelPrimaryAction()
          }
          .buttonStyle(.borderedProminent)
          .disabled(!viewModel.canRunLocalModelPrimaryAction())

          if let secondaryActionTitle = viewModel.localModelSecondaryActionTitle() {
            Button(secondaryActionTitle) {
              viewModel.runLocalModelSecondaryAction()
            }
            .buttonStyle(.bordered)
            .disabled(!viewModel.canRunLocalModelSecondaryAction())
          }
        }
      }

      DisclosureGroup("Choose Model", isExpanded: $modelChooserExpanded) {
        modelManager
      }

      DisclosureGroup("Troubleshooting", isExpanded: $modelTroubleshootingExpanded) {
        ModelTroubleshootingPanel(viewModel: viewModel)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }

  private var modelManager: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(viewModel.modelManagerSummary())
          .font(.caption2)
          .foregroundColor(.secondary)
        Spacer()
        Button("Reset Active") {
          viewModel.resetActiveLocalModel()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canResetActiveLocalModel())
      }

      Text(viewModel.localModelManagerRuleSummary())
        .font(.caption2)
        .foregroundColor(.secondary)

      ForEach(viewModel.localModels) { model in
        LocalModelRow(model: model, viewModel: viewModel)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}

private struct ModelDownloadProgressView: View {
  let value: Double?
  let summary: String

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      if let value {
        ProgressView(value: value)
          .progressViewStyle(.linear)
      } else {
        ProgressView()
          .progressViewStyle(.linear)
      }

      Text(summary)
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
    }
  }
}

private struct LocalModelRow: View {
  let model: LocalModelSummary
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 5) {
      Text(viewModel.localModelChoiceSummary(model))
        .font(.caption2)
        .fontWeight(.medium)
        .foregroundColor(model.active ? .green : .secondary)
      Text(model.displayName)
        .font(.caption)
        .fontWeight(.semibold)
      Text(model.description)
        .font(.caption2)
        .foregroundColor(.secondary)
      Text(viewModel.localModelStatusSummary(model))
        .font(.caption2)
        .foregroundColor(.secondary)
      Text(viewModel.localModelTagSummary(model))
        .font(.caption2)
        .foregroundColor(.secondary)

      HStack(spacing: 8) {
        Button(modelUseButtonTitle) {
          viewModel.activateRecommendedModel(modelID: model.id)
        }
        .buttonStyle(.borderedProminent)
        .disabled(!viewModel.canActivateRecommendedModel(modelID: model.id))

        Button(viewModel.localModelDownloadButtonTitle(model)) {
          viewModel.downloadRecommendedModel(modelID: model.id)
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canDownloadRecommendedModel(modelID: model.id))

        Button("Reveal") {
          viewModel.revealRecommendedModel(modelID: model.id)
        }
        .buttonStyle(.bordered)
        .disabled(!model.downloaded)
      }

      if model.needsVerification {
        Text("Pith found a local file. Verify it before use, or replace it with a fresh download.")
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
    }
    .padding(.vertical, 4)
  }

  private var modelUseButtonTitle: String {
    if model.active {
      return "Active"
    }
    if model.needsVerification {
      return "Verify"
    }
    return "Use"
  }
}

private struct ModelTroubleshootingPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(viewModel.modelDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)

      HStack(spacing: 8) {
        Button("Show Model Folder") {
          viewModel.revealSuggestedModelDirectory()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canRevealSuggestedModelDirectory())

        Button("Show Runtime Folder") {
          viewModel.revealSuggestedBinaryDirectory()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canRevealSuggestedBinaryDirectory())
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}
