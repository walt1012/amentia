import Foundation

enum RuntimeRelaunchAction {
  case stopAndLaunch
  case stopAndLaunchAfterCurrentLaunchSettles
  case updateIdleDetail
}

struct RuntimeRelaunchPlan {
  let action: RuntimeRelaunchAction
  let runtimeDetail: String
  let stopDetail: String?
  let launchDetail: String?
  let launchTimeoutDetail: String?
}

enum RuntimeRelaunchPlanner {
  static func plan(
    runtimeState: RuntimeBridge.ConnectionState,
    runningDetail: String,
    idleDetail: String
  ) -> RuntimeRelaunchPlan {
    switch runtimeState {
    case .ready:
      return RuntimeRelaunchPlan(
        action: .stopAndLaunch,
        runtimeDetail: runningDetail,
        stopDetail: runningDetail,
        launchDetail: runningDetail,
        launchTimeoutDetail: nil
      )
    case .launching:
      return RuntimeRelaunchPlan(
        action: .stopAndLaunchAfterCurrentLaunchSettles,
        runtimeDetail: runningDetail,
        stopDetail: runningDetail,
        launchDetail: runningDetail,
        launchTimeoutDetail: "Local engine is still launching. Relaunch it after model setup finishes."
      )
    case .disconnected, .failed:
      return RuntimeRelaunchPlan(
        action: .updateIdleDetail,
        runtimeDetail: idleDetail,
        stopDetail: nil,
        launchDetail: nil,
        launchTimeoutDetail: nil
      )
    }
  }
}
