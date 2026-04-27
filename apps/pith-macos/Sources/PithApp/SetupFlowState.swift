import Foundation

struct SetupFlowSnapshot {
  let runtimeState: RuntimeBridge.ConnectionState
  let isLocalModelReady: Bool
  let hasWorkspace: Bool
  let hasRuntimeThreadSelection: Bool
  let isWaitingForFirstMessage: Bool
}

enum SetupFlowState {
  static let coreStepCount = 4
  static let stepCount = 5

  static func readyStepCount(_ snapshot: SetupFlowSnapshot) -> Int {
    let readyCount = coreReadyStepCount(snapshot)
    guard readyCount == coreStepCount,
          !snapshot.isWaitingForFirstMessage
    else {
      return readyCount
    }

    return readyCount + 1
  }

  static func isCoreReadyForFirstRequest(_ snapshot: SetupFlowSnapshot) -> Bool {
    coreReadyStepCount(snapshot) == coreStepCount
  }

  static func shouldAnnotateLaunch(_ snapshot: SetupFlowSnapshot) -> Bool {
    readyStepCount(snapshot) < stepCount || snapshot.isWaitingForFirstMessage
  }

  private static func coreReadyStepCount(_ snapshot: SetupFlowSnapshot) -> Int {
    var readyCount = 0
    if snapshot.runtimeState == .ready {
      readyCount += 1
    }
    if snapshot.isLocalModelReady {
      readyCount += 1
    }
    if snapshot.hasWorkspace {
      readyCount += 1
    }
    if snapshot.isLocalModelReady,
       snapshot.hasWorkspace,
       snapshot.hasRuntimeThreadSelection
    {
      readyCount += 1
    }
    return readyCount
  }
}
