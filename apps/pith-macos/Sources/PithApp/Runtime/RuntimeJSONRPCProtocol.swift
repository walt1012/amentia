import Foundation

struct JSONRPCRequest<Params: Encodable>: Encodable {
  let id: Int
  let method: String
  let params: Params
}

struct JSONRPCResponse<ResultType: Decodable>: Decodable {
  let id: Int?
  let result: ResultType?
  let error: JSONRPCError?
}

struct JSONRPCAnyResponse: Decodable {
  let id: Int?
}

struct JSONRPCNotificationEnvelope: Decodable {
  let method: String
}

struct JSONRPCNotification<Params: Decodable>: Decodable {
  let method: String
  let params: Params
}

struct JSONRPCError: Decodable {
  let code: Int
  let message: String
  let data: JSONRPCErrorData?
}

struct JSONRPCErrorData: Decodable {
  let pluginId: String?
  let commandId: String?
  let connectorId: String?
  let sourcePath: String?
  let connectorStatus: String?
  let connectorRepairHint: String?
  let runStatus: String?
  let runBlocker: String?
  let runRepairHint: String?
}

extension JSONRPCErrorData {
  var repairHint: String? {
    if let runRepairHint, !runRepairHint.isEmpty {
      return runRepairHint
    }
    if let connectorRepairHint, !connectorRepairHint.isEmpty {
      return connectorRepairHint
    }
    return nil
  }

  var recoveryAttributes: [String: String] {
    var attributes: [String: String] = [:]
    append(pluginId, forKey: "pluginId", to: &attributes)
    append(commandId, forKey: "commandId", to: &attributes)
    append(connectorId, forKey: "connectorId", to: &attributes)
    append(sourcePath, forKey: "sourcePath", to: &attributes)
    append(connectorStatus, forKey: "connectorStatus", to: &attributes)
    append(connectorRepairHint, forKey: "connectorRepairHint", to: &attributes)
    append(runStatus, forKey: "runStatus", to: &attributes)
    append(runBlocker, forKey: "runBlocker", to: &attributes)
    append(runRepairHint, forKey: "runRepairHint", to: &attributes)
    return attributes
  }

  private func append(
    _ value: String?,
    forKey key: String,
    to attributes: inout [String: String]
  ) {
    guard let value, !value.isEmpty else {
      return
    }
    attributes[key] = value
  }
}

struct OptionalRequestParams: Encodable {
  static let none = OptionalRequestParams()
}

extension RuntimeBridge {
  func responseResult<ResultType: Decodable>(
    from response: JSONRPCResponse<ResultType>
  ) throws -> ResultType {
    if let error = response.error {
      if let data = error.data {
        let attributes = data.recoveryAttributes
        if !attributes.isEmpty {
          throw RuntimeError.rpcWithRecovery(
            message: error.message,
            repairHint: data.repairHint,
            attributes: attributes
          )
        }
        if let repairHint = data.repairHint {
          throw RuntimeError.rpcWithRepair(message: error.message, repairHint: repairHint)
        }
      }
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return result
  }
}
