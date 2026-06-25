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
    "Finish active work, model checks, or plugin and connection operations before deleting local data."

  static func summary(_ snapshot: LocalDataSettingsSnapshot) -> LocalDataSettingsSummary {
    LocalDataSettingsSummary(
      storageSummary: storageSummary(downloadedModelBytes: snapshot.downloadedModelBytes),
      ownershipDetail:
        "Amentia keeps downloaded models, sessions, plugins, saved connection sign-ins, preferences, caches, and window layout on this Mac. Project folders are never deleted here.",
      uninstallDetail:
        "Removing Amentia.app does not remove this data. Use Delete All Local Data when you want a fresh first-run setup.",
      blockedDetail: blockedDetail(canDeleteLocalData: snapshot.canDeleteLocalData),
      localDataPath: snapshot.localDataPath,
      revealButtonTitle: "Show Local Data",
      deleteButtonTitle: "Delete All Local Data...",
      confirmationTitle: "Delete All Local Amentia Data?",
      confirmationMessage:
        "Amentia will remove Amentia data from this Mac: downloaded models, sessions, plugins, saved connection sign-ins, paused downloads, preferences, caches, and saved window layout. Your project folders and repositories will not be deleted.",
      canDeleteLocalData: snapshot.canDeleteLocalData
    )
  }

  static func resetSummary(_ result: AppDataResetResult) -> LocalDataResetSummary {
    LocalDataResetSummary(
      runtimeDetail: "Deleted Amentia local data. Restart Amentia to set up again.",
      timelineTitle: "Local Data Deleted",
      timelineBody:
        "Amentia removed Amentia data, including downloaded models, sessions, plugins, saved connection sign-ins, paused downloads, preferences, caches, saved window layout, and Amentia support folders. Project folders on disk were not deleted.",
      attributes: [
        "appSupportPath": result.appSupportPath,
        "remainingAppOwnedDirectoryCount": "\(result.remainingAppOwnedDirectoryCount)",
      ]
    )
  }

  private static func storageSummary(downloadedModelBytes: Int64) -> String {
    if downloadedModelBytes > 0 {
      return "Downloaded models use \(LocalModelByteFormatter.string(downloadedModelBytes)) on this Mac."
    }

    return "No downloaded model files yet. Sessions, plugins, saved connections, preferences, and caches stay on this Mac."
  }

  private static func blockedDetail(canDeleteLocalData: Bool) -> String? {
    if canDeleteLocalData {
      return nil
    }

    return deleteBlockedDetail
  }
}
