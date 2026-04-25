import Foundation

enum LocalModelDownloadRequestMode {
  case start(downloadURL: URL)
  case blocked(detail: String)
}

struct LocalModelDownloadRequestPlan {
  let mode: LocalModelDownloadRequestMode

  var canStart: Bool {
    switch mode {
    case .start:
      return true
    case .blocked:
      return false
    }
  }

  var downloadURL: URL? {
    switch mode {
    case .start(let downloadURL):
      return downloadURL
    case .blocked:
      return nil
    }
  }

  var blockedDetail: String? {
    switch mode {
    case .start:
      return nil
    case .blocked(let detail):
      return detail
    }
  }
}

enum LocalModelDownloadRequestPlanner {
  static func plan(
    model: LocalModelSummary,
    isDownloadRunning: Bool,
    pausedModelID: String?,
    hasResumeData: Bool
  ) -> LocalModelDownloadRequestPlan {
    if isDownloadRunning {
      return .blocked("Finish, pause, or cancel the current model download before starting another.")
    }

    if model.downloaded {
      return .blocked("\(model.displayName) is already downloaded.")
    }

    if let pausedModelID {
      guard pausedModelID == model.id else {
        return .blocked("Continue or cancel the paused model download before starting another model.")
      }

      guard hasResumeData else {
        return .blocked("Cancel the paused model download before trying again.")
      }
    }

    guard let downloadURL = URL(string: model.downloadURL) else {
      return .blocked("The selected local model has an invalid download URL.")
    }

    return .start(downloadURL: downloadURL)
  }
}

private extension LocalModelDownloadRequestPlan {
  static func blocked(_ detail: String) -> LocalModelDownloadRequestPlan {
    LocalModelDownloadRequestPlan(mode: .blocked(detail: detail))
  }

  static func start(downloadURL: URL) -> LocalModelDownloadRequestPlan {
    LocalModelDownloadRequestPlan(mode: .start(downloadURL: downloadURL))
  }
}
