import Foundation

struct RuntimeConnectionStateSnapshot {
  let previousState: RuntimeBridge.ConnectionState
  let nextState: RuntimeBridge.ConnectionState
  let detail: String
  let lastFailureDetail: String?
}

struct RuntimeConnectionStatePlan {
  let clearsActiveTurnState: Bool
  let clearsModelReadinessState: Bool
  let resetsLastFailureDetail: Bool
  let updatedLastFailureDetail: String?
  let shouldAppendFailureNotice: Bool
}

enum RuntimeConnectionStateReducer {
  static func plan(_ snapshot: RuntimeConnectionStateSnapshot) -> RuntimeConnectionStatePlan {
    switch snapshot.nextState {
    case .ready:
      return RuntimeConnectionStatePlan(
        clearsActiveTurnState: false,
        clearsModelReadinessState: false,
        resetsLastFailureDetail: true,
        updatedLastFailureDetail: nil,
        shouldAppendFailureNotice: false
      )
    case .failed:
      return RuntimeConnectionStatePlan(
        clearsActiveTurnState: true,
        clearsModelReadinessState: true,
        resetsLastFailureDetail: false,
        updatedLastFailureDetail: snapshot.detail,
        shouldAppendFailureNotice: shouldAppendFailureNotice(snapshot)
      )
    case .disconnected:
      return RuntimeConnectionStatePlan(
        clearsActiveTurnState: true,
        clearsModelReadinessState: true,
        resetsLastFailureDetail: false,
        updatedLastFailureDetail: nil,
        shouldAppendFailureNotice: false
      )
    case .launching:
      return RuntimeConnectionStatePlan(
        clearsActiveTurnState: false,
        clearsModelReadinessState: false,
        resetsLastFailureDetail: false,
        updatedLastFailureDetail: nil,
        shouldAppendFailureNotice: false
      )
    }
  }

  private static func shouldAppendFailureNotice(_ snapshot: RuntimeConnectionStateSnapshot) -> Bool {
    snapshot.previousState != .failed || snapshot.lastFailureDetail != snapshot.detail
  }
}
