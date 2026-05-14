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
  let connectorStatus: String?
  let connectorRepairHint: String?
  let runStatus: String?
  let runBlocker: String?
  let runRepairHint: String?
}

struct OptionalRequestParams: Encodable {
  static let none = OptionalRequestParams()
}

extension RuntimeBridge {
  func responseResult<ResultType: Decodable>(
    from response: JSONRPCResponse<ResultType>
  ) throws -> ResultType {
    if let error = response.error {
      if let repairHint = error.data?.runRepairHint, !repairHint.isEmpty {
        throw RuntimeError.rpcWithRepair(message: error.message, repairHint: repairHint)
      }
      if let repairHint = error.data?.connectorRepairHint, !repairHint.isEmpty {
        throw RuntimeError.rpcWithRepair(message: error.message, repairHint: repairHint)
      }
      throw RuntimeError.rpc(error.message)
    }

    guard let result = response.result else {
      throw RuntimeError.invalidResponse
    }

    return result
  }
}
