import SwiftUI

struct SessionSidebarView: View {
  @ObservedObject var viewModel: AppViewModel
  @Binding var sessionDeleteCandidate: ThreadSummary?
  @Binding var sessionRevertCandidate: SessionRevertCandidate?

  var body: some View {
    List(selection: selectedSessionBinding) {
      Section("Sessions") {
        if viewModel.threads.isEmpty {
          SidebarEmptyState(
            title: "No Sessions Yet",
            detail: viewModel.sidebarEmptyStateDetail()
          )
        } else {
          ForEach(viewModel.threads) { thread in
            SidebarSessionRow(thread: thread)
              .tag(thread.id)
              .contextMenu {
                Button("Review Session Changes...") {
                  Task {
                    if let preview = await viewModel.previewThreadChanges(thread) {
                      sessionRevertCandidate = SessionRevertCandidate(
                        thread: thread,
                        preview: preview
                      )
                    }
                  }
                }
                .disabled(!viewModel.canRevertThreadChanges(thread))

                Button("Delete Session...", role: .destructive) {
                  sessionDeleteCandidate = thread
                }
                .disabled(!viewModel.canDeleteThread(thread))
              }
          }
        }
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
    .background(PithVisualStyle.windowBackground)
    .animation(PithMotionStyle.sectionReveal, value: viewModel.threads.count)
  }

  private var selectedSessionBinding: Binding<String?> {
    Binding(
      get: { viewModel.selectedThreadID },
      set: { viewModel.selectThread(id: $0) }
    )
  }
}

struct SessionRevertCandidate: Identifiable {
  let thread: ThreadSummary
  let preview: RuntimeBridge.RuntimeThreadChangePreview

  var id: String {
    thread.id
  }
}

private struct SidebarSessionRow: View {
  let thread: ThreadSummary

  var body: some View {
    HStack(alignment: .top, spacing: 8) {
      Image(systemName: "bubble.left")
        .font(.caption.weight(.medium))
        .foregroundColor(.secondary)
        .frame(width: 16)

      VStack(alignment: .leading, spacing: 3) {
        Text(thread.title)
          .font(.subheadline.weight(.medium))
          .lineLimit(1)
        Text(thread.preview)
          .font(.caption2)
          .foregroundColor(.secondary)
          .lineLimit(2)
      }
    }
    .padding(.vertical, 3)
    .help(thread.preview)
  }
}

private struct SidebarEmptyState: View {
  let title: String
  let detail: String

  var body: some View {
    VStack(alignment: .leading, spacing: 7) {
      Image(systemName: "sparkles")
        .font(.title3)
        .foregroundColor(.secondary)

      Text(title)
        .font(.subheadline.weight(.semibold))

      Text(detail)
        .font(.caption)
        .foregroundColor(.secondary)
        .fixedSize(horizontal: false, vertical: true)
    }
    .padding(.vertical, 8)
  }
}

private extension AppViewModel {
  func sidebarEmptyStateDetail() -> String {
    let progress = setupProgressSummary()
    let nextStep = setupProgressDetail()

    if nextStep == "Ready" {
      return "Create a session to keep prompts, approvals, and changes together."
    }

    return "\(progress). \(nextStep)."
  }
}
