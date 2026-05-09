import Foundation

@MainActor
extension AppViewModel {
  func revealRecommendedModel(modelID: String) {
    guard let model = localModel(for: modelID) else {
      runtimeDetail = "The selected local model is unavailable."
      return
    }

    runtimeDetail = FileRevealService.revealFilePath(
      model.installPath,
      successDetail: "Revealed \(model.displayName)."
    )
  }

  func revealSuggestedModelDirectory() {
    runtimeDetail = FileRevealService.revealSuggestedPath(
      metricKey: "suggestedModelPath",
      modelHealth: modelHealth,
      successDetail: "Opened the suggested local model folder."
    )
  }

  func canRevealSuggestedModelDirectory() -> Bool {
    FileRevealService.hasSuggestedPath(metricKey: "suggestedModelPath", modelHealth: modelHealth)
  }

  func revealSuggestedBinaryDirectory() {
    runtimeDetail = FileRevealService.revealSuggestedPath(
      metricKey: "suggestedBinaryPath",
      modelHealth: modelHealth,
      successDetail: "Opened the suggested llama.cpp binary folder."
    )
  }

  func canRevealSuggestedBinaryDirectory() -> Bool {
    FileRevealService.hasSuggestedPath(metricKey: "suggestedBinaryPath", modelHealth: modelHealth)
  }
}
