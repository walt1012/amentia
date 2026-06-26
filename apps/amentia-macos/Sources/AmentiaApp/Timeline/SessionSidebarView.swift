import SwiftUI

struct SessionSidebarView: View {
  @ObservedObject var viewModel: AppViewModel
  @State private var sessionDeleteCandidate: ThreadSummary?
  @State private var sessionRevertCandidate: SessionRevertCandidate?
  @State private var sessionSearchText = ""
  @State private var confirmsResetAmentia = false

  var body: some View {
    List(selection: selectedSessionBinding) {
      Section("Sessions") {
        if viewModel.threads.isEmpty {
          SidebarEmptyState(
            title: "No Sessions Yet",
            detail: viewModel.sidebarEmptyStateDetail()
          )
        } else if filteredSessions.isEmpty {
          SidebarEmptyState(
            title: "No Matching Sessions",
            detail: SessionSearchPresenter.emptyMatchDetail
          )
        } else {
          ForEach(filteredSessions) { thread in
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

      Section("Manage") {
        Button("Review Changes...") {
          reviewSelectedSessionChanges()
        }
        .disabled(selectedSession.map { !viewModel.canRevertThreadChanges($0) } ?? true)

        Button("Delete Session...", role: .destructive) {
          sessionDeleteCandidate = selectedSession
        }
        .disabled(selectedSession.map { !viewModel.canDeleteThread($0) } ?? true)

        Button(resetSummary.deleteButtonTitle, role: .destructive) {
          confirmsResetAmentia = true
        }
        .disabled(!viewModel.canDeleteLocalData())
      }
    }
    .frame(minWidth: 240)
    .listStyle(.sidebar)
    .background(AmentiaVisualStyle.windowBackground)
    .animation(AmentiaMotionStyle.sectionReveal, value: viewModel.threads.count)
    .searchable(text: $sessionSearchText)
    .alert(item: $sessionDeleteCandidate) { thread in
      let prompt = SessionChangePresenter.deletePrompt(threadTitle: thread.title)
      return Alert(
        title: Text(prompt.title),
        message: Text(prompt.message),
        primaryButton: .destructive(Text(prompt.confirmButtonTitle)) {
          viewModel.deleteThread(thread)
        },
        secondaryButton: .cancel()
      )
    }
    .alert(item: $sessionRevertCandidate) { candidate in
      let prompt = SessionChangePresenter.revertPrompt(
        for: candidate.preview,
        threadTitle: candidate.thread.title
      )
      if !prompt.allowsRevert {
        return Alert(
          title: Text(prompt.title),
          message: Text(prompt.message),
          dismissButton: .default(Text("OK"))
        )
      }

      return Alert(
        title: Text(prompt.title),
        message: Text(prompt.message),
        primaryButton: .destructive(Text(prompt.confirmButtonTitle)) {
          viewModel.revertThreadChanges(candidate.thread)
        },
        secondaryButton: .cancel()
      )
    }
    .alert(resetSummary.confirmationTitle, isPresented: $confirmsResetAmentia) {
      Button("Delete All Local Data", role: .destructive) {
        viewModel.deleteLocalData()
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text(resetSummary.confirmationMessage)
    }
  }

  private var selectedSessionBinding: Binding<String?> {
    Binding(
      get: { viewModel.selectedThreadID },
      set: { viewModel.selectThread(id: $0) }
    )
  }

  private var selectedSession: ThreadSummary? {
    viewModel.threads.first { $0.id == viewModel.selectedThreadID }
  }

  private var filteredSessions: [ThreadSummary] {
    SessionSearchPresenter.filteredSessions(
      viewModel.threads,
      query: sessionSearchText
    )
  }

  private var resetSummary: LocalDataSettingsSummary {
    viewModel.localDataSettingsSummary()
  }

  private func reviewSelectedSessionChanges() {
    guard let selectedSession else {
      return
    }

    Task {
      if let preview = await viewModel.previewThreadChanges(selectedSession) {
        sessionRevertCandidate = SessionRevertCandidate(
          thread: selectedSession,
          preview: preview
        )
      }
    }
  }
}

private struct SessionRevertCandidate: Identifiable {
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
