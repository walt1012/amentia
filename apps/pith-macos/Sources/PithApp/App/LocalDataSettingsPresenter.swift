import Foundation

struct LocalDataSettingsSnapshot: Equatable {
  let downloadedModelBytes: Int64
  let canDeleteLocalData: Bool
  let localDataPath: String
}

struct LocalDataSettingsSummary: Equatable {
  let storageSummary: String
  let ownershipDetail: String
  let blockedDetail: String?
  let localDataPath: String
  let revealButtonTitle: String
  let deleteButtonTitle: String
  let confirmationTitle: String
  let confirmationMessage: String
  let canDeleteLocalData: Bool
}

struct LocalDataResetSummary: Equatable {
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let attributes: [String: String]
}

enum LocalDataSettingsPresenter {
  static let deleteBlockedDetail =
    "Finish active local work, model downloads, model checks, or connector operations before deleting local data."

  static func summary(_ snapshot: LocalDataSettingsSnapshot) -> LocalDataSettingsSummary {
    LocalDataSettingsSummary(
      storageSummary: storageSummary(downloadedModelBytes: snapshot.downloadedModelBytes),
      ownershipDetail:
        "Pith local data includes models, sessions, connectors, download recovery data, and preferences. Workspaces are never deleted here.",
      blockedDetail: blockedDetail(canDeleteLocalData: snapshot.canDeleteLocalData),
      localDataPath: snapshot.localDataPath,
      revealButtonTitle: "Show Local Data",
      deleteButtonTitle: "Delete Local Data...",
      confirmationTitle: "Delete Pith Local Data?",
      confirmationMessage:
        "Pith will remove its downloaded models, sessions, connectors, download recovery data, and preferences. Your workspaces and repositories will not be deleted.",
      canDeleteLocalData: snapshot.canDeleteLocalData
    )
  }

  static func resetSummary(_ result: AppDataResetResult) -> LocalDataResetSummary {
    LocalDataResetSummary(
      runtimeDetail: "Deleted Pith local data. Restart the local service to set up again.",
      timelineTitle: "Local Data Deleted",
      timelineBody:
        "Pith removed downloaded models, sessions, connectors, download recovery data, and known preferences. Workspaces on disk were not deleted.",
      attributes: [
        "appSupportPath": result.appSupportPath,
        "recreatedDirectoryCount": "\(result.recreatedDirectoryCount)",
      ]
    )
  }

  private static func storageSummary(downloadedModelBytes: Int64) -> String {
    if downloadedModelBytes > 0 {
      return "Downloaded models use \(LocalModelByteFormatter.string(downloadedModelBytes)) on this Mac."
    }

    return "No downloaded model files yet. Sessions, connectors, and preferences stay local."
  }

  private static func blockedDetail(canDeleteLocalData: Bool) -> String? {
    if canDeleteLocalData {
      return nil
    }

    return deleteBlockedDetail
  }
}
