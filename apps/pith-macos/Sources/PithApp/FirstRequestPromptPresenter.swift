import Foundation

enum FirstRequestPromptPresenter {
  static let mapWorkspaceID = "map-workspace"
  static let reviewChangesID = "review-changes"

  static func calloutSummary() -> String {
    "Setup is complete. Choose a starter prompt, review it in the composer, then send locally."
  }

  static func calloutDetail(workspaceDisplayName: String?) -> String {
    guard let workspaceDisplayName, !workspaceDisplayName.isEmpty else {
      return "Choose a workspace before starting the first local request."
    }

    return "Pith will use \(workspaceDisplayName) as the working context and keep the first request scoped."
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
        message: "Map \(workspaceName). Explain the main modules, runtime flow, and one safe next development step."
      ),
      ComposerSuggestionSummary(
        id: reviewChangesID,
        title: "Review Changes",
        message: "Review the current changes in \(workspaceName). Call out the highest-risk issues first."
      ),
      ComposerSuggestionSummary(
        id: "plan-small-patch",
        title: "Plan Small Patch",
        message: "Find one small high-leverage patch for \(workspaceName) that keeps Pith lightweight and local-first."
      ),
    ]
  }

  static func suggestion(id: String, workspaceDisplayName: String?) -> ComposerSuggestionSummary? {
    suggestions(workspaceDisplayName: workspaceDisplayName).first(where: { $0.id == id })
  }
}
