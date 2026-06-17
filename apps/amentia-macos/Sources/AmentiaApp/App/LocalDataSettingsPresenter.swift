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
    "Finish active work, model downloads, model checks, or plugin and connection operations before resetting Amentia."

  static func summary(_ snapshot: LocalDataSettingsSnapshot) -> LocalDataSettingsSummary {
    LocalDataSettingsSummary(
      storageSummary: storageSummary(downloadedModelBytes: snapshot.downloadedModelBytes),
      ownershipDetail:
        "Amentia keeps downloaded models, sessions, plugins, connection credentials, preferences, caches, and window state on this Mac. Project folders are never deleted here.",
      uninstallDetail:
        "Removing Amentia.app does not remove this data. Use Reset Amentia when you want a fresh first-run setup.",
      blockedDetail: blockedDetail(canDeleteLocalData: snapshot.canDeleteLocalData),
      localDataPath: snapshot.localDataPath,
      revealButtonTitle: "Show Amentia Data",
      deleteButtonTitle: "Delete All Amentia Data...",
      confirmationTitle: "Delete All Amentia Data on This Mac?",
      confirmationMessage:
        "Amentia will remove all app-owned local data from this Mac: downloaded models, sessions, plugins, connection credentials, paused downloads, preferences, caches, and saved app state. Your project folders and repositories will not be deleted.",
      canDeleteLocalData: snapshot.canDeleteLocalData
    )
  }

  static func resetSummary(_ result: AppDataResetResult) -> LocalDataResetSummary {
    LocalDataResetSummary(
      runtimeDetail: "Reset Amentia. Restart Amentia to set up again.",
      timelineTitle: "Amentia Reset",
      timelineBody:
        "Amentia removed all app-owned local data, including downloaded models, sessions, plugins, connection credentials, paused downloads, preferences, caches, saved app state, and app-owned folders. Project folders on disk were not deleted.",
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

    return "No downloaded model files yet. Sessions, plugins, connections, preferences, and caches stay on this Mac."
  }

  private static func blockedDetail(canDeleteLocalData: Bool) -> String? {
    if canDeleteLocalData {
      return nil
    }

    return deleteBlockedDetail
  }
}
