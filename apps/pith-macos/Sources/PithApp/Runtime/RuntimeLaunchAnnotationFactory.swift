import Foundation

struct RuntimeLaunchAnnotationSnapshot {
  let serverName: String
  let serverVersion: String
  let shouldAnnotateSetupLaunch: Bool
  let restoredWorkspace: WorkspaceSummary?
  let skippedWorkspaceRestorePath: String?
  let workspaceRestoreErrorDetail: String?
  let modelHealth: ModelHealthSummary?
  let isLocalModelReady: Bool
  let localModelRequiredSummary: String
}

enum RuntimeLaunchAnnotationFactory {
  static func entries(_ snapshot: RuntimeLaunchAnnotationSnapshot) -> [TimelineEntry] {
    var entries: [TimelineEntry] = []

    if snapshot.shouldAnnotateSetupLaunch {
      entries.append(
        TimelineEntryFactory.system(
          title: "Local Engine Ready",
          body: "Connected to the local engine.",
          attributes: [:]
        )
      )
    }

    if let restoredWorkspace = snapshot.restoredWorkspace {
      entries.append(
        TimelineEntryFactory.system(
          title: "Workspace Restored",
          body: "Restored \(restoredWorkspace.displayName) at \(restoredWorkspace.rootPath).",
          attributes: [
            "workspacePath": restoredWorkspace.rootPath
          ]
        )
      )
    }

    if let skippedWorkspaceRestorePath = snapshot.skippedWorkspaceRestorePath {
      entries.append(
        TimelineEntryFactory.warning(
          title: "Workspace Restore Skipped",
          body: "The last workspace no longer exists. Open a workspace to continue.",
          attributes: [
            "workspacePath": skippedWorkspaceRestorePath
          ]
        )
      )
    }

    if let workspaceRestoreErrorDetail = snapshot.workspaceRestoreErrorDetail {
      entries.append(
        TimelineEntryFactory.warning(
          title: "Workspace Restore Failed",
          body: workspaceRestoreErrorDetail,
          attributes: [:]
        )
      )
    }

    entries.append(contentsOf: modelEntries(snapshot))
    return entries
  }

  private static func modelEntries(_ snapshot: RuntimeLaunchAnnotationSnapshot) -> [TimelineEntry] {
    guard let modelHealth = snapshot.modelHealth else {
      return [
        TimelineEntryFactory.warning(
          title: "Local Engine Required",
          body: snapshot.localModelRequiredSummary,
          attributes: [
            "modelStatus": "unavailable"
          ]
        )
      ]
    }

    let attributes = [
      "modelId": modelHealth.packID,
      "modelBackend": modelHealth.backend,
      "modelStatus": modelHealth.status,
      "modelSource": modelHealth.source,
    ]
    if snapshot.isLocalModelReady {
      guard snapshot.shouldAnnotateSetupLaunch else {
        return []
      }

      return [
        TimelineEntryFactory.system(
          title: "Local Engine Ready",
          body:
            "\(modelHealth.displayName) is ready for local work.",
          attributes: attributes
        )
      ]
    }

    return [
      TimelineEntryFactory.warning(
        title: "Local Engine Required",
        body: snapshot.localModelRequiredSummary,
        attributes: attributes
      )
    ]
  }
}
