import SwiftUI

struct LocalModelPanel: View {
  @ObservedObject var viewModel: AppViewModel
  @AppStorage("amentia.inspector.modelChooserExpanded") private var modelChooserExpanded = false
  @AppStorage("amentia.inspector.modelTroubleshootingExpanded") private var modelTroubleshootingExpanded = false

  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      HStack(alignment: .top, spacing: 10) {
        ModelStatusGlyph(tone: modelTone, isActive: viewModel.showsModelActivity())

        VStack(alignment: .leading, spacing: 4) {
          Text(viewModel.modelDisplayName())
            .font(.headline.weight(.semibold))
            .lineLimit(2)
          Text(viewModel.modelStatusSummary())
            .font(.caption)
            .foregroundColor(.secondary)
            .fixedSize(horizontal: false, vertical: true)
        }

        Spacer()

        StatusPill(label: modelPillLabel, tone: modelTone)
      }

      HStack(spacing: 6) {
        if viewModel.showsModelActivity() {
          ProgressView()
            .controlSize(.small)
        }
        Text(viewModel.modelActionSummary())
          .font(.caption2)
          .foregroundColor(viewModel.isModelActionBlocking() ? .orange : .secondary)
          .lineLimit(2)
      }

      if viewModel.shouldShowModelDownloadProgress() {
        ModelDownloadProgressView(
          value: viewModel.modelDownloadProgressValue(),
          summary: viewModel.modelDownloadProgressSummary()
        )
        .transition(.opacity.combined(with: .move(edge: .top)))
      } else {
        Text(viewModel.modelRecoverySummary())
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }

      if let primaryActionTitle = viewModel.localModelPrimaryActionTitle() {
        HStack(spacing: 8) {
          Button(primaryActionTitle) {
            viewModel.runLocalModelPrimaryAction()
          }
          .buttonStyle(.borderedProminent)
          .controlSize(.small)
          .disabled(!viewModel.canRunLocalModelPrimaryAction())

          if let secondaryActionTitle = viewModel.localModelSecondaryActionTitle() {
            Button(secondaryActionTitle) {
              viewModel.runLocalModelSecondaryAction()
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .disabled(!viewModel.canRunLocalModelSecondaryAction())
          }
        }
      }

      DisclosureGroup("Choose Model", isExpanded: $modelChooserExpanded) {
        modelManager
      }

      DisclosureGroup("Advanced", isExpanded: $modelTroubleshootingExpanded) {
        ModelTroubleshootingPanel(viewModel: viewModel)
      }
    }
    .softPanel()
    .frame(maxWidth: .infinity, alignment: .leading)
    .animation(AmentiaMotionStyle.sectionReveal, value: viewModel.showsModelActivity())
    .animation(AmentiaMotionStyle.sectionReveal, value: viewModel.shouldShowModelDownloadProgress())
  }

  private var modelTone: StatusTone {
    if viewModel.isModelActionBlocking() {
      return .warning
    }
    if viewModel.showsModelActivity() {
      return .active
    }
    if viewModel.modelStatusSummary() != "Ready to use" {
      return .neutral
    }
    return .ready
  }

  private var modelPillLabel: String {
    if viewModel.showsModelActivity() {
      return "Working"
    }
    if viewModel.isModelActionBlocking() {
      return "Needs Action"
    }
    if viewModel.modelStatusSummary() != "Ready to use" {
      return "Setup"
    }
    return "Ready"
  }

  private var modelManager: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text(viewModel.modelManagerSummary())
          .font(.caption2)
          .foregroundColor(.secondary)
        Spacer()
        Button("Reset Model") {
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

private struct ModelStatusGlyph: View {
  let tone: StatusTone
  let isActive: Bool

  var body: some View {
    ZStack {
      Circle()
        .fill(tone.color.opacity(0.11))
        .frame(width: 32, height: 32)
      if isActive {
        ProgressView()
          .controlSize(.small)
      } else {
        Circle()
          .fill(tone.color.opacity(0.82))
          .frame(width: 9, height: 9)
      }
    }
  }
}

private struct LocalModelRow: View {
  let model: LocalModelSummary
  @ObservedObject var viewModel: AppViewModel

  var body: some View {
    VStack(alignment: .leading, spacing: 7) {
      HStack(alignment: .firstTextBaseline, spacing: 6) {
        Text(LocalModelDisplayPresenter.setupTitle(model))
          .font(.caption.weight(.semibold))
          .lineLimit(1)
        Spacer()
        StatusPill(
          label: viewModel.localModelChoiceSummary(model),
          tone: model.active ? .ready : .neutral
        )
      }
      Text(model.description)
        .font(.caption2)
        .foregroundColor(.secondary)
      Text(viewModel.localModelFitSummary(model))
        .font(.caption2)
        .foregroundColor(.secondary)
        .lineLimit(2)

      Text(viewModel.localModelStatusSummary(model))
        .font(.caption2)
        .foregroundColor(.secondary)
        .lineLimit(2)

      ScrollView(.horizontal, showsIndicators: false) {
        HStack(spacing: 8) {
          Button(modelUseButtonTitle) {
            viewModel.activateRecommendedModel(modelID: model.id)
          }
          .buttonStyle(.borderedProminent)
          .controlSize(.small)
          .disabled(!viewModel.canActivateRecommendedModel(modelID: model.id))

          Button(viewModel.localModelDownloadButtonTitle(model)) {
            viewModel.downloadRecommendedModel(modelID: model.id)
          }
          .buttonStyle(.bordered)
          .controlSize(.small)
          .disabled(!viewModel.canDownloadRecommendedModel(modelID: model.id))

          Button("Show Download") {
            viewModel.revealRecommendedModel(modelID: model.id)
          }
          .buttonStyle(.bordered)
          .controlSize(.small)
          .disabled(!model.hasLocalFile)
        }
      }

      if model.needsVerification {
        Text("Amentia found this file on your Mac. Verify it before use, or replace it with a fresh download.")
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
    }
    .softPanel(isSelected: model.active)
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
        Button("Downloaded Models") {
          viewModel.revealSuggestedModelDirectory()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canRevealSuggestedModelDirectory())

        Button("App Support") {
          viewModel.revealSuggestedBinaryDirectory()
        }
        .buttonStyle(.bordered)
        .disabled(!viewModel.canRevealSuggestedBinaryDirectory())
      }
    }
    .softPanel()
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}
