import Foundation

enum FirstRequestPromptPresenter {
  static let firstAppOpenActionContractID = "map-plan-or-short-cowork-prompt"
  static let mapWorkspaceID = "map-workspace"
  static let planNextStepID = "plan-next-step"

  static func firstAppOpenActionSummary() -> String {
    "Choose Understand Project, Pick Next Step, or type a short cowork prompt."
  }

  static func firstAppOpenActionTrustSummary() -> String {
    "first app-open action offers Understand Project, Pick Next Step, or a short cowork prompt"
  }

  static func calloutSummary() -> String {
    "Core setup is ready. \(firstAppOpenActionSummary())"
  }

  static func calloutDetail(workspaceDisplayName: String?) -> String {
    guard let workspaceDisplayName, !workspaceDisplayName.isEmpty else {
      return "Choose a project before starting the first cowork session."
    }

    return "Pith will use \(workspaceDisplayName) as the working context. Short, specific cowork prompts work best for the local model."
  }

  static func primaryActionTitle(for suggestion: ComposerSuggestionSummary?) -> String? {
    suggestion?.title
  }

  static func secondaryActionTitle(for suggestion: ComposerSuggestionSummary?) -> String? {
    suggestion?.title
  }

  static func suggestions(workspaceDisplayName: String?) -> [ComposerSuggestionSummary] {
    let workspaceName = workspaceDisplayName ?? "this project"
    return [
      ComposerSuggestionSummary(
        id: mapWorkspaceID,
        title: "Understand Project",
        message: "Understand \(workspaceName) briefly. Return: 1. key folders, 2. project flow, 3. one safe next step."
      ),
      ComposerSuggestionSummary(
        id: planNextStepID,
        title: "Pick Next Step",
        message: "Help me choose the next useful step in \(workspaceName). Return only: 1. current situation, 2. safest next action, 3. what you need from me."
      ),
    ]
  }

  static func suggestion(id: String, workspaceDisplayName: String?) -> ComposerSuggestionSummary? {
    suggestions(workspaceDisplayName: workspaceDisplayName).first(where: { $0.id == id })
  }
}
