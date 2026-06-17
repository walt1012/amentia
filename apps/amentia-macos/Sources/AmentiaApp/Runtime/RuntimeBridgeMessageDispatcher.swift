import Foundation

enum RuntimeBridgeIncomingMessage {
  case response(id: Int, data: Data)
  case threadUpdated(RuntimeBridge.RuntimeThreadState)
  case ignored
}

struct RuntimeBridgeMessageDispatcher {
  private let decoder = JSONDecoder()

  func decode(_ data: Data) -> RuntimeBridgeIncomingMessage {
    if let response = try? decoder.decode(JSONRPCAnyResponse.self, from: data),
       let responseID = response.id
    {
      return .response(id: responseID, data: data)
    }

    guard let envelope = try? decoder.decode(JSONRPCNotificationEnvelope.self, from: data) else {
      return .ignored
    }

    switch envelope.method {
    case "thread/updated":
      return decodeThreadUpdated(data)
    default:
      return .ignored
    }
  }

  private func decodeThreadUpdated(_ data: Data) -> RuntimeBridgeIncomingMessage {
    guard let notification = try? decoder.decode(
      JSONRPCNotification<ThreadUpdatedNotificationParams>.self,
      from: data
    ) else {
      return .ignored
    }

    let state = RuntimeBridgePayloadMapper.threadState(
      id: notification.params.thread.id,
      title: notification.params.thread.title,
      status: notification.params.thread.status,
      items: notification.params.items,
      pendingApprovals: notification.params.pendingApprovals,
      activeTurnID: notification.params.activeTurnId
    )
    return .threadUpdated(state)
  }
}
