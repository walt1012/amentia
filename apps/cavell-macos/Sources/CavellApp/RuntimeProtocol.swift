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

struct JSONRPCError: Decodable {
  let code: Int
  let message: String
}

struct ClientInfo: Codable {
  let name: String
  let version: String
}

struct InitializeParams: Codable {
  let clientInfo: ClientInfo
}

struct ServerInfo: Codable {
  let name: String
  let version: String
}

struct ServerCapabilities: Codable {
  let supportsThreads: Bool
  let supportsTools: Bool
  let supportsPlugins: Bool
}

struct InitializeResult: Codable {
  let serverInfo: ServerInfo
  let protocolVersion: String
  let capabilities: ServerCapabilities
}

struct ThreadListResult: Codable {
  let threads: [RuntimeThreadPayload]
}

struct RuntimeThreadPayload: Codable {
  let id: String
  let title: String
  let status: String
}

struct ThreadStartParams: Codable {
  let title: String
}

struct ThreadStartResult: Codable {
  let thread: RuntimeThreadPayload
}

struct OptionalRequestParams: Encodable {
  static let none = OptionalRequestParams()
}
