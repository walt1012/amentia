import Foundation

@MainActor
final class LocalModelDownloadCoordinator {
  private(set) var task: Task<Void, Never>?
  private var transfer: ModelDownloadTransfer?
  var resumeData: Data?

  init(resumeData: Data? = nil) {
    self.resumeData = resumeData
  }

  var isDownloading: Bool {
    task != nil
  }

  var canPause: Bool {
    task != nil
  }

  func start(_ task: Task<Void, Never>) {
    self.task = task
  }

  func attachTransfer(_ transfer: ModelDownloadTransfer) {
    self.transfer = transfer
  }

  func pauseActiveTransfer() {
    transfer?.pause()
  }

  func cancelActiveDownload() {
    task?.cancel()
    transfer?.cancel()
  }

  func finishActiveDownload() {
    task = nil
    transfer = nil
  }

  func clearResumeData() {
    resumeData = nil
  }
}

struct LocalModelDownloadRuntimeState: Hashable {
  var activeModelID: String?
  var pausedModelID: String?
  var progress: ModelDownloadProgress?

  var hasActiveDownload: Bool {
    activeModelID != nil
  }

  var hasPausedDownload: Bool {
    pausedModelID != nil
  }

  var hasAnyDownloadState: Bool {
    hasActiveDownload || hasPausedDownload
  }

  mutating func applyStart(_ sessionState: LocalModelDownloadSessionStartState) {
    activeModelID = sessionState.activeModelID
    pausedModelID = sessionState.pausedModelID
    progress = sessionState.progress
  }

  mutating func clearActiveDownload() {
    activeModelID = nil
  }

  mutating func markPaused(modelID: String) {
    pausedModelID = modelID
  }

  mutating func clearPausedDownload() {
    pausedModelID = nil
  }

  mutating func clearProgress() {
    progress = nil
  }

  mutating func updateProgress(_ nextProgress: ModelDownloadProgress) {
    progress = nextProgress
  }
}

enum LocalModelDownloadStartMode {
  case newDownload
  case resuming(resumeData: Data)
}

struct LocalModelDownloadStartPlan {
  let mode: LocalModelDownloadStartMode
  let progress: ModelDownloadProgress
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let attributes: [String: String]

  var isResuming: Bool {
    switch mode {
    case .newDownload:
      return false
    case .resuming:
      return true
    }
  }

  var resumeData: Data? {
    switch mode {
    case .newDownload:
      return nil
    case .resuming(let data):
      return data
    }
  }
}

enum LocalModelDownloadStartPlanner {
  static func plan(
    model: LocalModelSummary,
    sourceURL: URL,
    pausedModelID: String?,
    resumeData: Data?,
    currentProgress: ModelDownloadProgress?
  ) -> LocalModelDownloadStartPlan {
    let now = Date()
    let mode: LocalModelDownloadStartMode
    let resumedBytes: Int64
    let isResuming: Bool

    if pausedModelID == model.id, let resumeData {
      mode = .resuming(resumeData: resumeData)
      resumedBytes = currentProgress?.modelID == model.id
        ? currentProgress?.bytesReceived ?? 0
        : 0
      isResuming = true
    } else {
      mode = .newDownload
      resumedBytes = 0
      isResuming = false
    }

    let verb = isResuming ? "Continuing" : "Downloading"
    let eventVerb = isResuming ? "continued" : "started"

    return LocalModelDownloadStartPlan(
      mode: mode,
      progress: ModelDownloadProgress(
        modelID: model.id,
        displayName: model.displayName,
        bytesReceived: resumedBytes,
        totalBytes: model.sizeBytes,
        startedAt: now,
        updatedAt: now,
        isResuming: isResuming
      ),
      runtimeDetail: "\(verb) \(model.displayName) (\(formattedByteCount(model.sizeBytes)))...",
      timelineTitle: isResuming ? "Local Model Download Continued" : "Local Model Download Started",
      timelineBody:
        "\(model.displayName) download \(eventVerb) from \(sourceURL.absoluteString).",
      attributes: [
        "downloadUrl": sourceURL.absoluteString,
        "result": isResuming ? "continued" : "started",
        "size": formattedByteCount(model.sizeBytes),
      ]
    )
  }

  private static func formattedByteCount(_ byteCount: Int64) -> String {
    let formatter = ByteCountFormatter()
    formatter.countStyle = .file
    return formatter.string(fromByteCount: byteCount)
  }
}

enum LocalModelDownloadCompletionMode {
  case downloadedOnly
  case activated
  case waitingForTurn
}

struct LocalModelDownloadCompletionPlan {
  let mode: LocalModelDownloadCompletionMode
  let runtimeDetail: String
  let timelineBody: String
  let attributes: [String: String]
  let relaunchRunningDetail: String?
  let relaunchIdleDetail: String?
}

enum LocalModelDownloadCompletionPlanner {
  static func plan(
    model: LocalModelSummary,
    sourceURL: URL,
    activationRequested: Bool,
    canActivateNow: Bool,
    manifestPath: String?
  ) -> LocalModelDownloadCompletionPlan {
    if activationRequested, canActivateNow, let manifestPath {
      return LocalModelDownloadCompletionPlan(
        mode: .activated,
        runtimeDetail: "Downloaded and selected \(model.displayName).",
        timelineBody: "\(model.displayName) was downloaded and selected as the active local model.",
        attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
          [
            "manifestPath": manifestPath,
            "result": "activated",
          ],
          uniquingKeysWith: { _, new in new }
        ),
        relaunchRunningDetail: "Restarting local runtime with \(model.displayName)...",
        relaunchIdleDetail: "\(model.displayName) will be used when the runtime launches."
      )
    }

    if activationRequested {
      return LocalModelDownloadCompletionPlan(
        mode: .waitingForTurn,
        runtimeDetail: "Downloaded \(model.displayName). Finish the current turn before selecting it.",
        timelineBody:
          "\(model.displayName) was downloaded, but activation is waiting for the current local turn to finish.",
        attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
          [
            "result": "downloaded_pending_activation",
          ],
          uniquingKeysWith: { _, new in new }
        ),
        relaunchRunningDetail: nil,
        relaunchIdleDetail: nil
      )
    }

    return LocalModelDownloadCompletionPlan(
      mode: .downloadedOnly,
      runtimeDetail: "Downloaded \(model.displayName) to \(model.installPath).",
      timelineBody: "\(model.displayName) was downloaded to \(model.installPath).",
      attributes: baseAttributes(model: model, sourceURL: sourceURL).merging(
        [
          "result": "downloaded",
        ],
        uniquingKeysWith: { _, new in new }
      ),
      relaunchRunningDetail: nil,
      relaunchIdleDetail: nil
    )
  }

  private static func baseAttributes(model: LocalModelSummary, sourceURL: URL) -> [String: String] {
    [
      "modelPath": model.installPath,
      "source": sourceURL.absoluteString,
    ]
  }
}

struct LocalModelDownloadFinalizationPlan {
  let canActivateNow: Bool
  let preparedActivation: PreparedLocalModelActivation?

  var manifestPath: String? {
    preparedActivation?.manifestPath
  }
}

enum LocalModelDownloadFinalizer {
  static func prepare(
    model: LocalModelSummary,
    activationRequested: Bool,
    hasActiveOrPendingTurn: Bool
  ) throws -> LocalModelDownloadFinalizationPlan {
    try LocalModelActivationPreparer.validateDownloadedModel(model)

    let canActivateNow = !hasActiveOrPendingTurn
    guard activationRequested && canActivateNow else {
      return LocalModelDownloadFinalizationPlan(
        canActivateNow: canActivateNow,
        preparedActivation: nil
      )
    }

    return LocalModelDownloadFinalizationPlan(
      canActivateNow: canActivateNow,
      preparedActivation: PreparedLocalModelActivation(
        manifestPath: try LocalModelActivationPreparer.writeManifest(for: model)
      )
    )
  }
}

enum LocalModelDownloadInterruptionMode {
  case paused(resumeData: Data)
  case cancelled
  case failed
}

struct LocalModelDownloadInterruptionPlan {
  let mode: LocalModelDownloadInterruptionMode
  let runtimeDetail: String
  let timelineTitle: String
  let timelineBody: String
  let timelineKind: TimelineEntry.Kind
  let attributes: [String: String]
  let clearsPausedState: Bool
  let clearsProgress: Bool
  let removesPartialFile: Bool
}

enum LocalModelDownloadInterruptionPlanner {
  static func plan(model: LocalModelSummary, error: Error) -> LocalModelDownloadInterruptionPlan {
    if let paused = error as? ModelDownloadPaused {
      return LocalModelDownloadInterruptionPlan(
        mode: .paused(resumeData: paused.resumeData),
        runtimeDetail: "Paused \(model.displayName) download. Continue to resume from the saved partial state.",
        timelineTitle: "Local Model Download Paused",
        timelineBody:
          "\(model.displayName) download was paused and can continue from the saved local state.",
        timelineKind: .system,
        attributes: [
          "result": "paused",
        ],
        clearsPausedState: false,
        clearsProgress: false,
        removesPartialFile: false
      )
    }

    if isCancellation(error) {
      return cancellationPlan(model: model)
    }

    return LocalModelDownloadInterruptionPlan(
      mode: .failed,
      runtimeDetail: "Model download failed: \(error.localizedDescription)",
      timelineTitle: "Local Model Download Failed",
      timelineBody: "\(model.displayName) download failed: \(error.localizedDescription)",
      timelineKind: .warning,
      attributes: [
        "error": error.localizedDescription,
        "result": "failed",
      ],
      clearsPausedState: true,
      clearsProgress: true,
      removesPartialFile: false
    )
  }

  static func cancellationPlan(model: LocalModelSummary) -> LocalModelDownloadInterruptionPlan {
    LocalModelDownloadInterruptionPlan(
      mode: .cancelled,
      runtimeDetail: "Cancelled \(model.displayName) download and cleared partial state.",
      timelineTitle: "Local Model Download Cancelled",
      timelineBody: "\(model.displayName) download was cancelled and the partial file was cleared.",
      timelineKind: .system,
      attributes: [
        "result": "cancelled",
      ],
      clearsPausedState: true,
      clearsProgress: true,
      removesPartialFile: true
    )
  }

  private static func isCancellation(_ error: Error) -> Bool {
    if error is CancellationError {
      return true
    }

    return (error as? URLError)?.code == .cancelled
  }
}

enum LocalModelDownloadCancelMode {
  case running
  case paused(model: LocalModelSummary)
  case orphanedPaused(modelID: String)
}

struct LocalModelDownloadCancelPlan {
  let mode: LocalModelDownloadCancelMode
  let runtimeDetail: String
}

enum LocalModelDownloadControlPlanner {
  static func pauseDetail(activeModelID: String?, models: [LocalModelSummary]) -> String {
    "Pausing \(displayName(for: activeModelID, models: models)) download..."
  }

  static func cancelPlan(
    isDownloading: Bool,
    activeModelID: String?,
    pausedModelID: String?,
    models: [LocalModelSummary]
  ) -> LocalModelDownloadCancelPlan? {
    if isDownloading {
      return LocalModelDownloadCancelPlan(
        mode: .running,
        runtimeDetail: "Cancelling \(displayName(for: activeModelID, models: models)) download..."
      )
    }

    guard let pausedModelID else {
      return nil
    }

    guard let model = models.first(where: { $0.id == pausedModelID }) else {
      return LocalModelDownloadCancelPlan(
        mode: .orphanedPaused(modelID: pausedModelID),
        runtimeDetail: "Cancelled local model download and cleared partial state."
      )
    }

    return LocalModelDownloadCancelPlan(
      mode: .paused(model: model),
      runtimeDetail: LocalModelDownloadInterruptionPlanner
        .cancellationPlan(model: model)
        .runtimeDetail
    )
  }

  private static func displayName(for modelID: String?, models: [LocalModelSummary]) -> String {
    modelID
      .flatMap { id in models.first(where: { $0.id == id })?.displayName }
      ?? "local model"
  }
}

struct LocalModelDownloadProgressUpdate {
  let modelID: String
  let activeModelID: String?
  let currentProgress: ModelDownloadProgress?
  let bytesReceived: Int64
  let totalBytes: Int64
  let updatedAt: Date
}

enum LocalModelDownloadProgressUpdater {
  static func updatedProgress(
    _ update: LocalModelDownloadProgressUpdate
  ) -> ModelDownloadProgress? {
    guard update.activeModelID == update.modelID,
          update.currentProgress?.modelID == update.modelID,
          var progress = update.currentProgress
    else {
      return nil
    }

    progress.bytesReceived = max(update.bytesReceived, progress.bytesReceived)
    progress.totalBytes = update.totalBytes > 0 ? update.totalBytes : progress.totalBytes
    progress.updatedAt = update.updatedAt
    return progress
  }
}

struct LocalModelDownloadSessionStartState {
  let activeModelID: String
  let pausedModelID: String?
  let progress: ModelDownloadProgress
  let clearsPausedState: Bool
  let shouldActivateAfterDownload: Bool
}

struct LocalModelDownloadSessionCompletionState {
  let completionPlan: LocalModelDownloadCompletionPlan
  let preparedActivation: PreparedLocalModelActivation?
}

enum LocalModelDownloadSessionPlanner {
  static func startState(
    model: LocalModelSummary,
    startPlan: LocalModelDownloadStartPlan,
    activateAfterDownload: Bool,
    isLocalModelReady: Bool
  ) -> LocalModelDownloadSessionStartState {
    LocalModelDownloadSessionStartState(
      activeModelID: model.id,
      pausedModelID: nil,
      progress: startPlan.progress,
      clearsPausedState: true,
      shouldActivateAfterDownload: activateAfterDownload || !isLocalModelReady
    )
  }

  static func completionState(
    model: LocalModelSummary,
    sourceURL: URL,
    activationRequested: Bool,
    hasActiveOrPendingTurn: Bool
  ) throws -> LocalModelDownloadSessionCompletionState {
    let finalizationPlan = try LocalModelDownloadFinalizer.prepare(
      model: model,
      activationRequested: activationRequested,
      hasActiveOrPendingTurn: hasActiveOrPendingTurn
    )
    let completionPlan = LocalModelDownloadCompletionPlanner.plan(
      model: model,
      sourceURL: sourceURL,
      activationRequested: activationRequested,
      canActivateNow: finalizationPlan.canActivateNow,
      manifestPath: finalizationPlan.manifestPath
    )

    return LocalModelDownloadSessionCompletionState(
      completionPlan: completionPlan,
      preparedActivation: finalizationPlan.preparedActivation
    )
  }
}
