import SwiftUI

struct LocalModelPanel: View {
  @ObservedObject var viewModel: AppViewModel
  @AppStorage("pith.inspector.modelManagerExpanded") private var modelManagerExpanded = false
  @AppStorage("pith.inspector.modelDiagnosticsExpanded") private var modelDiagnosticsExpanded = false

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

      DisclosureGroup("Model Manager", isExpanded: $modelManagerExpanded) {
        modelManager
      }

      DisclosureGroup("Model Diagnostics", isExpanded: $modelDiagnosticsExpanded) {
        ModelDiagnosticsPanel(viewModel: viewModel)
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
        Button(model.active ? "Active" : "Use") {
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

      Text(viewModel.localModelPathSummary(model))
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
    }
    .padding(.vertical, 4)
  }
}

private struct ModelDiagnosticsPanel: View {
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(viewModel.modelDetailSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
      Text(viewModel.modelSourceSummary())
        .font(.caption)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
      Text(viewModel.modelMetricsSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
      Text(viewModel.modelReadinessSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
      Text(viewModel.modelInstallHintSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
      Text(viewModel.modelSuggestedPathSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)
      Text(viewModel.modelArtifactPathSummary())
        .font(.caption2)
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      HStack(spacing: 8) {
        Button("Reveal Model Folder") {
          viewModel.revealSuggestedModelDirectory()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canRevealSuggestedModelDirectory())

        Button("Reveal Binary Folder") {
          viewModel.revealSuggestedBinaryDirectory()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canRevealSuggestedBinaryDirectory())
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}
