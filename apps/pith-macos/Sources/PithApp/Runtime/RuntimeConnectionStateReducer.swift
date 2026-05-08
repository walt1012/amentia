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

struct RuntimeConnectionStateStore {
  var state: RuntimeBridge.ConnectionState
  var detail: String
  var lastFailureDetail: String?

  init(
    state: RuntimeBridge.ConnectionState,
    detail: String,
    lastFailureDetail: String? = nil
  ) {
    self.state = state
    self.detail = detail
    self.lastFailureDetail = lastFailureDetail
  }

  mutating func clearLastFailureDetail() {
    lastFailureDetail = nil
  }

  mutating func applyConnectionUpdate(
    state nextState: RuntimeBridge.ConnectionState,
    detail nextDetail: String,
    plan: RuntimeConnectionStatePlan
  ) {
    state = nextState
    detail = nextDetail

    if plan.resetsLastFailureDetail {
      lastFailureDetail = nil
    }
    if let updatedLastFailureDetail = plan.updatedLastFailureDetail {
      lastFailureDetail = updatedLastFailureDetail
    }
  }
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
