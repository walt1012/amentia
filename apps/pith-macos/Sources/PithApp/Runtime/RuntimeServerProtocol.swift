import Foundation

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
  let supportsMemory: Bool
  let supportsThreads: Bool
  let supportsTools: Bool
  let supportsPlugins: Bool
  let supportsRuntimeReadiness: Bool
}

struct InitializeResult: Codable {
  let serverInfo: ServerInfo
  let protocolVersion: String
  let capabilities: ServerCapabilities
}
