import Foundation

final class RuntimeBridgePendingResponses {
  private let queue = DispatchQueue(label: "pith.runtime.bridge.pending-responses")
  private var nextRequestID: Int = 1
  private var continuations: [Int: CheckedContinuation<Data, Error>] = [:]

  func reserveRequestID() -> Int {
    queue.sync {
      let requestID = nextRequestID
      nextRequestID += 1
      return requestID
    }
  }

  func store(_ continuation: CheckedContinuation<Data, Error>, requestID: Int) {
    queue.sync {
      continuations[requestID] = continuation
    }
  }

  func take(requestID: Int) -> CheckedContinuation<Data, Error>? {
    queue.sync {
      continuations.removeValue(forKey: requestID)
    }
  }

  func failAll(with error: Error) {
    let pending = queue.sync {
      let pending = Array(continuations.values)
      continuations.removeAll()
      return pending
    }

    for continuation in pending {
      continuation.resume(throwing: error)
    }
  }
}
