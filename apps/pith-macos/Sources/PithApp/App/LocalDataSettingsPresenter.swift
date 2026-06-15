import Foundation

struct LocalDataSettingsSnapshot: Equatable {
  let downloadedModelBytes: Int64
  let canDeleteLocalData: Bool
  let localDataPath: String
}

struct LocalDataSettingsSummary: Equatable {
  let storageSummary: String
  let ownershipDetail: String
  let uninstallDetail: String
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
    "Finish active work, model downloads, model checks, or plugin and connection operations before deleting local data."

  static func summary(_ snapshot: LocalDataSettingsSnapshot) -> LocalDataSettingsSummary {
    LocalDataSettingsSummary(
      storageSummary: storageSummary(downloadedModelBytes: snapshot.downloadedModelBytes),
      ownershipDetail:
        "Pith local data includes models, sessions, plugins, connection credentials, download recovery data, and preferences. Project folders are never deleted here.",
      uninstallDetail:
        "Removing Pith.app does not remove this data. Use Delete Local Data here when you want Pith to forget local setup.",
      blockedDetail: blockedDetail(canDeleteLocalData: snapshot.canDeleteLocalData),
      localDataPath: snapshot.localDataPath,
      revealButtonTitle: "Show Local Data",
      deleteButtonTitle: "Delete Local Data...",
      confirmationTitle: "Delete Pith Local Data?",
      confirmationMessage:
        "Pith will remove downloaded models, sessions, plugins, connection credentials, download recovery data, and preferences from this Mac. Your project folders and repositories will not be deleted.",
      canDeleteLocalData: snapshot.canDeleteLocalData
    )
  }

  static func resetSummary(_ result: AppDataResetResult) -> LocalDataResetSummary {
    LocalDataResetSummary(
      runtimeDetail: "Deleted Pith local data. Restart Pith to set up again.",
      timelineTitle: "Local Data Deleted",
      timelineBody:
        "Pith removed downloaded models, sessions, plugins, connection credentials, download recovery data, and known preferences. Project folders on disk were not deleted.",
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

    return "No downloaded model files yet. Sessions, plugins, connections, and preferences stay local."
  }

  private static func blockedDetail(canDeleteLocalData: Bool) -> String? {
    if canDeleteLocalData {
      return nil
    }

    return deleteBlockedDetail
  }
}
