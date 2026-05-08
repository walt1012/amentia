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

struct ModelHealthResult: Codable {
  let packId: String
  let displayName: String
  let backend: String
  let status: String
  let detail: String
  let source: String
  let binaryPath: String?
  let modelPath: String?
  let manifestPath: String?
  let metrics: [String: String]
}

struct RuntimeReadinessCheckResult: Codable {
  let id: String
  let title: String
  let status: String
  let detail: String
}

struct RuntimeReadinessResult: Codable {
  let status: String
  let summary: String
  let checks: [RuntimeReadinessCheckResult]
  let metrics: [String: String]
}

struct ModelBootstrapResult: Codable {
  let manifestPath: String
  let readmePath: String?
  let copiedFiles: [String]
}

struct MemoryStatusResult: Codable {
  let noteCount: Int
  let latestTitle: String?
  let summary: String
}

struct MemoryListResult: Codable {
  let notes: [RuntimeMemoryNotePayload]
}

struct MemoryCreateParams: Codable {
  let title: String
  let body: String
}

struct MemoryCreateResult: Codable {
  let note: RuntimeMemoryNotePayload
}

struct RuntimeMemoryNotePayload: Codable {
  let id: String
  let title: String
  let body: String
  let scope: String
  let source: String
  let createdAt: Int
  let tags: [String]
}
