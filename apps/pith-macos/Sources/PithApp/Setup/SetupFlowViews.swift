import SwiftUI

struct SetupProgressView: View {
  let summary: String
  let detail: String
  let value: Double
  let tone: StatusTone

  var body: some View {
    VStack(alignment: .leading, spacing: 4) {
      HStack {
        Text(summary)
          .font(.caption2.weight(.semibold))
          .foregroundColor(tone.color)
        Spacer()
        Text(detail)
          .font(.caption2)
          .foregroundColor(.secondary)
          .lineLimit(1)
      }
      ProgressView(value: value)
        .progressViewStyle(.linear)
        .tint(tone.color)
    }
  }
}

struct SetupModelChooser: View {
  let models: [LocalModelSummary]
  @Binding var selectedModelID: String
  let defaultModelID: String
  let detail: String
  let isDisabled: Bool
  let actionTitle: String?
  let canRunAction: Bool
  let onAction: () -> Void

  var body: some View {
    VStack(alignment: .leading, spacing: 10) {
      HStack(alignment: .firstTextBaseline, spacing: 8) {
        Text("Choose Local Model")
          .font(.caption.weight(.semibold))
        StatusPill(label: "One active model", tone: .neutral)
        Spacer()

        if let actionTitle {
          Button(actionTitle) {
            onAction()
          }
          .buttonStyle(.borderedProminent)
          .controlSize(.small)
          .disabled(!canRunAction)
        }
      }

      Text(detail)
        .font(.caption2)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)

      VStack(alignment: .leading, spacing: 8) {
        ForEach(models) { model in
          SetupModelOptionRow(
            model: model,
            isSelected: selectedModelID == model.id,
            isDefault: model.id == defaultModelID,
            defaultModelID: defaultModelID,
            isDisabled: isDisabled,
            onSelect: {
              selectedModelID = model.id
            }
          )
        }
      }
    }
    .softPanel()
  }
}

private struct SetupModelOptionRow: View {
  let model: LocalModelSummary
  let isSelected: Bool
  let isDefault: Bool
  let defaultModelID: String
  let isDisabled: Bool
  let onSelect: () -> Void

  var body: some View {
    Button(action: onSelect) {
      VStack(alignment: .leading, spacing: 5) {
        HStack(alignment: .firstTextBaseline, spacing: 6) {
          Text(LocalModelDisplayPresenter.setupTitle(model))
            .font(.caption.weight(.semibold))
            .foregroundColor(.primary)
          if isDefault {
            SetupModelBadge(label: "Default", tone: .ready)
          }
          if model.tags.contains("recommended") {
            SetupModelBadge(label: "Recommended", tone: .ready)
          }
          if model.active {
            SetupModelBadge(label: "Active", tone: .active)
          } else if model.downloaded {
            SetupModelBadge(label: "Downloaded", tone: .neutral)
          }
          if isSelected {
            Spacer()
            SetupModelBadge(label: "Selected", tone: .warning)
          }
        }

        Text(fit)
          .font(.caption2)
          .foregroundColor(.secondary)

        Text(capability)
          .font(.caption2)
          .foregroundColor(.secondary)

        Text(footprint)
          .font(.caption2)
          .foregroundColor(.secondary)
      }
      .padding(8)
      .frame(maxWidth: .infinity, alignment: .leading)
      .softPanel(isSelected: isSelected)
    }
    .buttonStyle(.plain)
    .disabled(isDisabled)
  }

  private var capability: String {
    LocalModelDisplayPresenter.setupCapabilitySummary(model)
  }

  private var footprint: String {
    LocalModelDisplayPresenter.setupFootprintSummary(model)
  }

  private var fit: String {
    LocalModelDisplayPresenter.setupFitSummary(model, defaultModelID: defaultModelID)
  }
}

private struct SetupModelBadge: View {
  let label: String
  let tone: StatusTone

  var body: some View {
    Text(label)
      .font(.caption2.weight(.semibold))
      .foregroundColor(tone.color)
      .padding(.horizontal, 6)
      .padding(.vertical, 2)
      .background(tone.color.opacity(0.12))
      .clipShape(Capsule())
  }
}

struct SetupCallout: View {
  let title: String
  let summary: String
  let detail: String
  let tone: StatusTone
  let actionTitle: String?
  let canRunAction: Bool
  let secondaryActionTitle: String?
  let canRunSecondaryAction: Bool
  let onAction: () -> Void
  let onSecondaryAction: () -> Void

  var body: some View {
    HStack(alignment: .top, spacing: 12) {
      VStack(alignment: .leading, spacing: 4) {
        Text(title)
          .font(.caption.weight(.semibold))
          .foregroundColor(tone.color)
        Text(summary)
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
        Text(detail)
          .font(.caption2)
          .foregroundColor(.secondary)
      }

      Spacer()

      VStack(alignment: .trailing, spacing: 6) {
        if let actionTitle {
          Button(actionTitle) {
            onAction()
          }
          .buttonStyle(.borderedProminent)
          .disabled(!canRunAction)
        }

        if let secondaryActionTitle {
          Button(secondaryActionTitle) {
            onSecondaryAction()
          }
          .buttonStyle(.bordered)
          .disabled(!canRunSecondaryAction)
        }
      }
    }
    .softPanel(tone: tone)
  }
}
