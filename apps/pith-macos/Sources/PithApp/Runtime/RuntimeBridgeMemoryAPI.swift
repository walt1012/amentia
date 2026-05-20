import Foundation

extension RuntimeBridge {
  struct RuntimeMemoryStatus {
    let noteCount: Int
    let latestTitle: String?
    let summary: String
  }

  struct RuntimeMemoryNote {
    let id: String
    let title: String
    let body: String
    let scope: String
    let source: String
    let createdAt: Int
    let tags: [String]
  }

  func memoryStatus() async throws -> RuntimeMemoryStatus {
    let response: JSONRPCResponse<MemoryStatusResult> = try await sendRequest(
      method: "memory/status",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeMemoryStatus(
      noteCount: result.noteCount,
      latestTitle: result.latestTitle,
      summary: result.summary
    )
  }

  func listMemoryNotes() async throws -> [RuntimeMemoryNote] {
    let response: JSONRPCResponse<MemoryListResult> = try await sendRequest(
      method: "memory/list",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return result.notes.map { note in
      RuntimeMemoryNote(
        id: note.id,
        title: note.title,
        body: note.body,
        scope: note.scope,
        source: note.source,
        createdAt: note.createdAt,
        tags: note.tags
      )
    }
  }

  func createMemoryNote(title: String, body: String) async throws -> RuntimeMemoryNote {
    let response: JSONRPCResponse<MemoryCreateResult> = try await sendRequest(
      method: "memory/create",
      params: MemoryCreateParams(title: title, body: body)
    )
    let result = try responseResult(from: response)

    return RuntimeMemoryNote(
      id: result.note.id,
      title: result.note.title,
      body: result.note.body,
      scope: result.note.scope,
      source: result.note.source,
      createdAt: result.note.createdAt,
      tags: result.note.tags
    )
  }
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
