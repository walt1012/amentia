import Foundation

enum FirstRequestPromptPresenter {
  static let mapWorkspaceID = "map-workspace"
  static let reviewChangesID = "review-changes"

  static func calloutSummary() -> String {
    "Setup is complete. Choose a short starter prompt, review it, then send it to the local model."
  }

  static func calloutDetail(workspaceDisplayName: String?) -> String {
    guard let workspaceDisplayName, !workspaceDisplayName.isEmpty else {
      return "Choose a workspace before starting the first local request."
    }

    return "Pith will use \(workspaceDisplayName) as the working context. Keep the first request small so the local model can stay focused."
  }

  static func primaryActionTitle(for suggestion: ComposerSuggestionSummary?) -> String? {
    suggestion == nil ? nil : "Use Map Prompt"
  }

  static func secondaryActionTitle(for suggestion: ComposerSuggestionSummary?) -> String? {
    suggestion == nil ? nil : "Use Review Prompt"
  }

  static func suggestions(workspaceDisplayName: String?) -> [ComposerSuggestionSummary] {
    let workspaceName = workspaceDisplayName ?? "this workspace"
    return [
      ComposerSuggestionSummary(
        id: mapWorkspaceID,
        title: "Map Workspace",
        message: "Map \(workspaceName) briefly. Return: 1. key folders, 2. runtime flow, 3. one safe next step."
      ),
      ComposerSuggestionSummary(
        id: reviewChangesID,
        title: "Review Changes",
        message: "Review current changes in \(workspaceName). Return only: 1. highest-risk issue, 2. missing test, 3. safe fix."
      ),
      ComposerSuggestionSummary(
        id: "plan-small-patch",
        title: "Plan Small Patch",
        message: "Find one small patch for \(workspaceName). Keep it local-first. Return: target file, reason, first edit."
      ),
    ]
  }

  static func suggestion(id: String, workspaceDisplayName: String?) -> ComposerSuggestionSummary? {
    suggestions(workspaceDisplayName: workspaceDisplayName).first(where: { $0.id == id })
  }
}
