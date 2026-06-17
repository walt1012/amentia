import SwiftUI

struct DiffDetailView: View {
  let summary: String
  let lines: [DiffLineSummary]

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text(summary)
        .font(.caption.weight(.semibold))
        .foregroundColor(.secondary)
        .textSelection(.enabled)

      ScrollView(.horizontal) {
        VStack(alignment: .leading, spacing: 2) {
          ForEach(lines) { line in
            DiffLineRow(line: line)
          }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
      }
    }
    .frame(maxWidth: .infinity, alignment: .leading)
  }
}

private struct DiffLineRow: View {
  let line: DiffLineSummary

  var body: some View {
    HStack(alignment: .top, spacing: 8) {
      Text("\(line.lineNumber)")
        .font(.system(.caption2, design: .monospaced))
        .foregroundColor(.secondary)
        .frame(width: 34, alignment: .trailing)

      Text(line.text.isEmpty ? " " : line.text)
        .font(.system(.caption, design: .monospaced))
        .foregroundColor(foregroundColor)
        .textSelection(.enabled)
    }
    .padding(.vertical, 2)
    .padding(.horizontal, 6)
    .background(backgroundColor)
    .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
  }

  private var foregroundColor: Color {
    switch line.kind {
    case .addition:
      return .green
    case .deletion:
      return .red
    case .hunk:
      return .blue
    case .metadata:
      return .secondary
    case .context:
      return .primary
    }
  }

  private var backgroundColor: Color {
    switch line.kind {
    case .addition:
      return Color.green.opacity(0.10)
    case .deletion:
      return Color.red.opacity(0.10)
    case .hunk:
      return Color.blue.opacity(0.10)
    case .metadata:
      return Color.secondary.opacity(0.08)
    case .context:
      return Color.clear
    }
  }
}
