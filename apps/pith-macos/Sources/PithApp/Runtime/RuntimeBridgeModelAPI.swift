import Foundation

extension RuntimeBridge {
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
