import SwiftUI

struct SetupProgressView: View {
  let summary: String
  let detail: String
  let value: Double
  let tone: StatusTone

  var body: some View {
    HStack(alignment: .center, spacing: 12) {
      ZStack {
        Circle()
          .fill(tone.color.opacity(0.12))
          .frame(width: 30, height: 30)
        Circle()
          .fill(tone.color)
          .frame(width: 8, height: 8)
      }

      VStack(alignment: .leading, spacing: 5) {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
          Text(summary)
            .font(.caption.weight(.semibold))
            .foregroundColor(.primary)
          StatusPill(label: detail, tone: tone)
          Spacer()
        }

        ProgressView(value: value)
          .progressViewStyle(.linear)
          .tint(tone.color)
      }
    }
    .softPanel(tone: tone)
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

  private let columns = [
    GridItem(.adaptive(minimum: 250), spacing: 10, alignment: .top),
  ]

  var body: some View {
    VStack(alignment: .leading, spacing: 14) {
      HStack(alignment: .top, spacing: 12) {
        ZStack {
          Circle()
            .fill(Color.accentColor.opacity(0.11))
            .frame(width: 34, height: 34)
          Image(systemName: "cpu")
            .font(.body.weight(.semibold))
            .foregroundColor(.accentColor)
        }

        VStack(alignment: .leading, spacing: 4) {
          HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text("Choose Local Model")
              .font(.headline.weight(.semibold))
            StatusPill(label: "One active model", tone: .neutral)
          }
          Text(detail)
            .font(.caption)
            .foregroundColor(.secondary)
            .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)

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

      LazyVGrid(columns: columns, alignment: .leading, spacing: 10) {
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
    .softPanel(tone: .active)
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
      VStack(alignment: .leading, spacing: 8) {
        HStack(alignment: .top, spacing: 8) {
          Text(LocalModelDisplayPresenter.setupTitle(model))
            .font(.subheadline.weight(.semibold))
            .foregroundColor(.primary)
            .lineLimit(2)

          Spacer(minLength: 8)

          if isSelected {
            Image(systemName: "checkmark.circle.fill")
              .font(.body)
              .foregroundColor(.accentColor)
          }
        }

        HStack(alignment: .firstTextBaseline, spacing: 5) {
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
      .padding(10)
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
    HStack(alignment: .top, spacing: 14) {
      ZStack {
        Circle()
          .fill(tone.color.opacity(0.12))
          .frame(width: 36, height: 36)
        Image(systemName: iconName)
          .font(.body.weight(.semibold))
          .foregroundColor(tone.color)
      }

      VStack(alignment: .leading, spacing: 6) {
        Text(title)
          .font(.headline.weight(.semibold))
          .foregroundColor(.primary)
        Text(summary)
          .font(.caption)
          .foregroundColor(.primary)
          .fixedSize(horizontal: false, vertical: true)
        Text(detail)
          .font(.caption2)
          .foregroundColor(.secondary)
          .fixedSize(horizontal: false, vertical: true)
      }
      .frame(maxWidth: .infinity, alignment: .leading)

      Spacer()

      VStack(alignment: .trailing, spacing: 6) {
        if let actionTitle {
          Button(actionTitle) {
            onAction()
          }
          .buttonStyle(.borderedProminent)
          .controlSize(.small)
          .disabled(!canRunAction)
        }

        if let secondaryActionTitle {
          Button(secondaryActionTitle) {
            onSecondaryAction()
          }
          .buttonStyle(.bordered)
          .controlSize(.small)
          .disabled(!canRunSecondaryAction)
        }
      }
    }
    .softPanel(tone: tone)
  }

  private var iconName: String {
    switch tone {
    case .ready:
      return "checkmark"
    case .active:
      return "sparkles"
    case .warning:
      return "exclamationmark"
    case .danger:
      return "xmark"
    case .neutral:
      return "circle"
    }
  }
}
