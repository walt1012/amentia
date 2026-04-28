import AppKit
import Foundation

enum FileRevealService {
  static func revealSuggestedPath(
    metricKey: String,
    modelHealth: ModelHealthSummary?,
    successDetail: String
  ) -> String {
    guard let value = modelHealth?.metrics[metricKey], !value.isEmpty else {
      return "Local model guidance is unavailable until the runtime reports model health."
    }

    let targetURL = URL(fileURLWithPath: value)
    let directoryURL: URL
    var isDirectory = ObjCBool(false)
    if FileManager.default.fileExists(atPath: targetURL.path, isDirectory: &isDirectory) {
      directoryURL = isDirectory.boolValue ? targetURL : targetURL.deletingLastPathComponent()
    } else {
      directoryURL = targetURL.deletingLastPathComponent()
      do {
        try FileManager.default.createDirectory(
          at: directoryURL,
          withIntermediateDirectories: true
        )
      } catch {
        return "Failed to prepare \(directoryURL.path): \(error.localizedDescription)"
      }
    }

    if NSWorkspace.shared.open(directoryURL) {
      return successDetail
    }

    return "Failed to open \(directoryURL.path)"
  }

  static func hasSuggestedPath(metricKey: String, modelHealth: ModelHealthSummary?) -> Bool {
    guard let value = modelHealth?.metrics[metricKey] else {
      return false
    }

    return !value.isEmpty
  }

  static func revealFilePath(_ path: String, successDetail: String) -> String {
    guard !path.isEmpty else {
      return "The requested file path is unavailable."
    }

    let fileURL = URL(fileURLWithPath: path)
    let manager = FileManager.default
    if manager.fileExists(atPath: fileURL.path) {
      NSWorkspace.shared.activateFileViewerSelecting([fileURL])
      return successDetail
    }

    let parentURL = fileURL.deletingLastPathComponent()
    if manager.fileExists(atPath: parentURL.path) {
      NSWorkspace.shared.activateFileViewerSelecting([parentURL])
      return "Revealed the closest available folder for \(path)."
    }

    return "Failed to locate \(path)"
  }
}
