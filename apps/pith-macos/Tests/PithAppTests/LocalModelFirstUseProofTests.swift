import CryptoKit
import Foundation
@testable import PithApp
import XCTest

final class LocalModelFirstUseProofTests: XCTestCase {
  func testDownloadedModelValidatesAndPreparesActivationManifest() throws {
    let rootURL = try temporaryDirectory()
    defer {
      try? FileManager.default.removeItem(at: rootURL)
    }

    let modelURL = rootURL
      .appendingPathComponent("catalog", isDirectory: true)
      .appendingPathComponent("first-use-proof", isDirectory: true)
      .appendingPathComponent("first-use-proof.gguf")
    let expectedSizeBytes: Int64 = 64 * 1024 * 1024
    let expectedSHA256 = try writeGGUFFixture(at: modelURL, sizeBytes: expectedSizeBytes)
    let model = localModelSummary(
      modelURL: modelURL,
      sizeBytes: expectedSizeBytes,
      sha256: expectedSHA256
    )

    let finalization = try LocalModelDownloadFinalizer.prepare(
      model: model,
      activationRequested: true,
      hasActiveOrPendingTurn: false
    )

    XCTAssertTrue(finalization.canActivateNow)
    let manifestPath = try XCTUnwrap(finalization.manifestPath)
    XCTAssertTrue(FileManager.default.fileExists(atPath: manifestPath))
    let manifest = try manifestJSON(at: manifestPath)
    XCTAssertEqual(manifest["id"] as? String, model.id)
    XCTAssertEqual(manifest["file_name"] as? String, model.fileName)
    XCTAssertEqual(manifest["sha256"] as? String, model.sha256)
    XCTAssertEqual((manifest["size_bytes"] as? NSNumber)?.int64Value, model.sizeBytes)
    XCTAssertEqual((manifest["context_size"] as? NSNumber)?.intValue, model.contextSize)
    XCTAssertEqual((manifest["max_output_tokens"] as? NSNumber)?.intValue, model.maxOutputTokens)

    let plan = LocalModelDownloadCompletionPlanner.plan(
      model: model,
      sourceURL: try XCTUnwrap(URL(string: model.downloadURL)),
      activationRequested: true,
      canActivateNow: finalization.canActivateNow,
      manifestPath: manifestPath
    )
    guard case .activated = plan.mode else {
      XCTFail("Expected the first-use completion plan to activate the validated model.")
      return
    }
    XCTAssertEqual(plan.attributes["result"], "activated")
    XCTAssertEqual(plan.attributes["manifestPath"], manifestPath)
  }

  func testPausedFirstUseDownloadResumesFromKnownProgress() throws {
    let rootURL = try temporaryDirectory()
    defer {
      try? FileManager.default.removeItem(at: rootURL)
    }

    let modelURL = rootURL
      .appendingPathComponent("catalog", isDirectory: true)
      .appendingPathComponent("resume-proof", isDirectory: true)
      .appendingPathComponent("resume-proof.gguf")
    try FileManager.default.createDirectory(
      at: modelURL.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    let model = localModelSummary(
      modelURL: modelURL,
      sizeBytes: 128 * 1024 * 1024,
      sha256: String(repeating: "a", count: 64)
    )
    let progress = ModelDownloadProgress(
      modelID: model.id,
      displayName: model.displayName,
      bytesReceived: 120 * 1024 * 1024,
      totalBytes: model.sizeBytes,
      startedAt: Date(timeIntervalSince1970: 1),
      updatedAt: Date(timeIntervalSince1970: 2),
      isResuming: true
    )

    let requestPlan = LocalModelDownloadRequestPlanner.plan(
      model: model,
      isDownloadRunning: false,
      pausedModelID: model.id,
      hasResumeData: true,
      resumeBytesReceived: progress.bytesReceived
    )
    XCTAssertTrue(requestPlan.canStart)

    let resumeData = Data([0x01, 0x02, 0x03])
    let startPlan = LocalModelDownloadStartPlanner.plan(
      model: model,
      sourceURL: try XCTUnwrap(requestPlan.downloadURL),
      pausedModelID: model.id,
      resumeData: resumeData,
      currentProgress: progress
    )

    guard case .resuming(let plannedResumeData) = startPlan.mode else {
      XCTFail("Expected first-use download to resume with persisted resume data.")
      return
    }
    XCTAssertEqual(plannedResumeData, resumeData)
    XCTAssertEqual(startPlan.progress.bytesReceived, progress.bytesReceived)
    XCTAssertTrue(startPlan.progress.isResuming)
    XCTAssertTrue(startPlan.runtimeDetail.hasPrefix("Continuing"))
  }

  func testLaunchValidationRejectsReplacedModel() throws {
    let rootURL = try temporaryDirectory()
    defer {
      try? FileManager.default.removeItem(at: rootURL)
    }

    let modelURL = rootURL
      .appendingPathComponent("catalog", isDirectory: true)
      .appendingPathComponent("launch-integrity-proof", isDirectory: true)
      .appendingPathComponent("launch-integrity-proof.gguf")
    let expectedSizeBytes: Int64 = 64 * 1024 * 1024
    let expectedSHA256 = try writeGGUFFixture(at: modelURL, sizeBytes: expectedSizeBytes)
    let model = localModelSummary(
      modelURL: modelURL,
      sizeBytes: expectedSizeBytes,
      sha256: expectedSHA256
    )
    let item = LocalModelCatalogItem(
      id: model.id,
      displayName: model.displayName,
      description: model.description,
      fileName: model.fileName,
      downloadURL: model.downloadURL,
      homepage: model.homepage,
      sizeBytes: model.sizeBytes,
      sha256: model.sha256,
      contextSize: model.contextSize,
      modelContextSize: model.modelContextSize,
      maxOutputTokens: model.maxOutputTokens,
      license: model.license,
      tags: model.tags,
      installSegments: []
    )

    try LocalModelIntegrity.validateDownloadedModel(model)
    let originalAttributes = try FileManager.default.attributesOfItem(atPath: modelURL.path)
    let originalModificationDate = try XCTUnwrap(originalAttributes[.modificationDate] as? Date)
    XCTAssertTrue(LocalModelIntegrity.state(at: modelURL.path, item: item).isVerified)

    _ = try writeGGUFFixture(at: modelURL, sizeBytes: expectedSizeBytes, fillByte: 0x71)
    try FileManager.default.setAttributes(
      [.modificationDate: originalModificationDate],
      ofItemAtPath: modelURL.path
    )

    XCTAssertThrowsError(try LocalModelIntegrity.validateDownloadedModel(model)) { error in
      guard case LocalModelIntegrityError.checksumMismatch = error else {
        XCTFail("Expected checksum mismatch, got \(error).")
        return
      }
    }
  }

  func testLegacyVerificationStampKeepsExistingDownloadsVisible() throws {
    let rootURL = try temporaryDirectory()
    defer {
      try? FileManager.default.removeItem(at: rootURL)
    }

    let modelURL = rootURL
      .appendingPathComponent("catalog", isDirectory: true)
      .appendingPathComponent("legacy-stamp-proof", isDirectory: true)
      .appendingPathComponent("legacy-stamp-proof.gguf")
    let expectedSizeBytes: Int64 = 64 * 1024 * 1024
    let expectedSHA256 = try writeGGUFFixture(at: modelURL, sizeBytes: expectedSizeBytes)
    let model = localModelSummary(
      modelURL: modelURL,
      sizeBytes: expectedSizeBytes,
      sha256: expectedSHA256
    )
    defer {
      UserDefaults.standard.removeObject(forKey: "pith.verifiedLocalModel.\(model.id)")
    }

    let attributes = try FileManager.default.attributesOfItem(atPath: modelURL.path)
    let size = try XCTUnwrap(attributes[.size] as? NSNumber).int64Value
    let modificationDate = try XCTUnwrap(attributes[.modificationDate] as? Date)
    let modificationMilliseconds = Int64(modificationDate.timeIntervalSince1970 * 1000)
    let legacyStamp = [
      URL(fileURLWithPath: modelURL.path).standardizedFileURL.path,
      String(size),
      String(modificationMilliseconds),
      model.sha256.lowercased(),
    ].joined(separator: "|")
    UserDefaults.standard.set(legacyStamp, forKey: "pith.verifiedLocalModel.\(model.id)")

    let item = LocalModelCatalogItem(
      id: model.id,
      displayName: model.displayName,
      description: model.description,
      fileName: model.fileName,
      downloadURL: model.downloadURL,
      homepage: model.homepage,
      sizeBytes: model.sizeBytes,
      sha256: model.sha256,
      contextSize: model.contextSize,
      modelContextSize: model.modelContextSize,
      maxOutputTokens: model.maxOutputTokens,
      license: model.license,
      tags: model.tags,
      installSegments: []
    )
    XCTAssertTrue(LocalModelIntegrity.state(at: modelURL.path, item: item).isVerified)
  }

  private func temporaryDirectory() throws -> URL {
    let rootURL = FileManager.default.temporaryDirectory
      .appendingPathComponent("pith-model-first-use-\(UUID().uuidString)", isDirectory: true)
    try FileManager.default.createDirectory(at: rootURL, withIntermediateDirectories: true)
    return rootURL
  }

  private func localModelSummary(
    modelURL: URL,
    sizeBytes: Int64,
    sha256: String
  ) -> LocalModelSummary {
    LocalModelSummary(
      id: "first-use-proof-\(UUID().uuidString)",
      displayName: "First Use Proof Model",
      description: "Small deterministic fixture for the local model manager path.",
      fileName: modelURL.lastPathComponent,
      downloadURL: "https://example.com/models/\(modelURL.lastPathComponent)",
      homepage: "https://example.com/models",
      sizeBytes: sizeBytes,
      sha256: sha256,
      contextSize: 2_048,
      modelContextSize: 2_048,
      maxOutputTokens: 256,
      license: "Apache-2.0",
      tags: ["test", "local"],
      installPath: modelURL.path,
      downloaded: false,
      active: false,
      localSizeBytes: nil
    )
  }

  private func writeGGUFFixture(
    at url: URL,
    sizeBytes: Int64,
    fillByte: UInt8 = 0x70
  ) throws -> String {
    try FileManager.default.createDirectory(
      at: url.deletingLastPathComponent(),
      withIntermediateDirectories: true
    )
    _ = FileManager.default.createFile(atPath: url.path, contents: nil)

    let handle = try FileHandle(forWritingTo: url)
    defer {
      try? handle.close()
    }

    var hasher = SHA256()
    let header = Data([0x47, 0x47, 0x55, 0x46])
    handle.write(header)
    hasher.update(data: header)

    let fullChunk = Data(repeating: fillByte, count: 1024 * 1024)
    var remainingBytes = sizeBytes - Int64(header.count)
    while remainingBytes > 0 {
      let writeCount = min(remainingBytes, Int64(fullChunk.count))
      let chunk = writeCount == Int64(fullChunk.count)
        ? fullChunk
        : Data(repeating: fillByte, count: Int(writeCount))
      handle.write(chunk)
      hasher.update(data: chunk)
      remainingBytes -= writeCount
    }

    return hasher.finalize().map { String(format: "%02x", $0) }.joined()
  }

  private func manifestJSON(at path: String) throws -> [String: Any] {
    let data = try Data(contentsOf: URL(fileURLWithPath: path))
    return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
  }
}
