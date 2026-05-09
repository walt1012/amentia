import Foundation

extension RuntimeBridge {
  struct RuntimeModelHealth {
    let packID: String
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

  struct RuntimeReadinessCheck {
    let id: String
    let title: String
    let status: String
    let detail: String
  }

  struct RuntimeReadiness {
    let status: String
    let summary: String
    let checks: [RuntimeReadinessCheck]
    let metrics: [String: String]
  }

  struct RuntimeModelBootstrap {
    let manifestPath: String
    let readmePath: String?
    let copiedFiles: [String]
  }

  func modelHealth() async throws -> RuntimeModelHealth {
    let response: JSONRPCResponse<ModelHealthResult> = try await sendRequest(
      method: "model/health",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeModelHealth(
      packID: result.packId,
      displayName: result.displayName,
      backend: result.backend,
      status: result.status,
      detail: result.detail,
      source: result.source,
      binaryPath: result.binaryPath,
      modelPath: result.modelPath,
      manifestPath: result.manifestPath,
      metrics: result.metrics
    )
  }

  func runtimeReadiness() async throws -> RuntimeReadiness {
    let response: JSONRPCResponse<RuntimeReadinessResult> = try await sendRequest(
      method: "runtime/readiness",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeReadiness(
      status: result.status,
      summary: result.summary,
      checks: result.checks.map { check in
        RuntimeReadinessCheck(
          id: check.id,
          title: check.title,
          status: check.status,
          detail: check.detail
        )
      },
      metrics: result.metrics
    )
  }

  func bootstrapModelPack() async throws -> RuntimeModelBootstrap {
    let response: JSONRPCResponse<ModelBootstrapResult> = try await sendRequest(
      method: "model/bootstrap",
      params: OptionalRequestParams.none
    )
    let result = try responseResult(from: response)

    return RuntimeModelBootstrap(
      manifestPath: result.manifestPath,
      readmePath: result.readmePath,
      copiedFiles: result.copiedFiles
    )
  }
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
