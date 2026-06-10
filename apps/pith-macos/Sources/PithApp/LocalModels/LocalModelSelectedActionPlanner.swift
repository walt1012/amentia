import Foundation

struct LocalModelSelectedActionSnapshot {
  let selectedModel: LocalModelSummary?
  let requestPlan: LocalModelDownloadRequestPlan?
  let canActivateDownloadedModel: Bool
  let activationBlockedDetail: String
}

enum LocalModelSelectedAction {
  case activate(modelID: String)
  case download(modelID: String)
  case blocked(detail: String)
}

enum LocalModelSelectedActionPlanner {
  static func action(_ snapshot: LocalModelSelectedActionSnapshot) -> LocalModelSelectedAction {
    guard let model = snapshot.selectedModel else {
      return .blocked(detail: "Choose a local model before downloading.")
    }

    if model.active {
      return .blocked(
        detail: "\(LocalModelDisplayPresenter.actionName(model)) is already the active local model."
      )
    }

    if model.downloaded || model.needsVerification {
      guard snapshot.canActivateDownloadedModel else {
        return .blocked(detail: snapshot.activationBlockedDetail)
      }

      return .activate(modelID: model.id)
    }

    guard let requestPlan = snapshot.requestPlan else {
      return .blocked(detail: "The selected local model is not ready to download.")
    }

    if requestPlan.canStart {
      return .download(modelID: model.id)
    }

    return .blocked(
      detail: requestPlan.blockedDetail ?? "The selected local model is not ready to download."
    )
  }

  static func canRun(_ action: LocalModelSelectedAction) -> Bool {
    switch action {
    case .activate, .download:
      return true
    case .blocked:
      return false
    }
  }
}
