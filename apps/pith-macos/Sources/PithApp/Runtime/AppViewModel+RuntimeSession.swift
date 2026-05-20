import Foundation

@MainActor
extension AppViewModel {
  func runtimeLaunchButtonTitle() -> String {
    SessionActionPlanner.runtimeLaunchButtonTitle(sessionActionSnapshot())
  }

  func shouldShowRuntimeLaunchToolbarAction() -> Bool {
    runtimeState == .disconnected || runtimeState == .failed
  }

  func runtimeStatusSummary() -> String {
    RuntimeHeaderPresenter.statusSummary(runtimeHeaderSnapshot())
  }

  func runtimeStatusTone() -> StatusTone {
    RuntimeHeaderPresenter.statusTone(runtimeHeaderSnapshot())
  }

  func showsRuntimeActivity() -> Bool {
    RuntimeHeaderPresenter.showsActivity(runtimeHeaderSnapshot())
  }

  func shouldShowRuntimeHeaderDetail() -> Bool {
    RuntimeHeaderPresenter.shouldShowDetail(runtimeHeaderSnapshot())
  }

  func runtimeReadinessSteps() -> [ReadinessStepSummary] {
    RuntimeReadinessPresenter.steps(runtimeReadinessSnapshot())
  }

  func setupProgressSummary() -> String {
    SetupProgressPresenter.summary(setupProgressSnapshot())
  }

  func setupProgressDetail() -> String {
    SetupProgressPresenter.detail(setupProgressSnapshot())
  }

  func setupProgressValue() -> Double {
    SetupProgressPresenter.value(setupProgressSnapshot())
  }

  func setupProgressTone() -> StatusTone {
    SetupProgressPresenter.tone(setupProgressSnapshot())
  }

  func shouldShowSetupProgress() -> Bool {
    let snapshot = setupProgressSnapshot()
    return snapshot.readyStepCount < snapshot.stepCount
      || snapshot.runtimeState == .launching
      || modelDownloadState.hasAnyDownloadState
  }

  func shouldShowReadinessSteps() -> Bool {
    shouldShowSetupProgress()
  }

  func inspectorSessionTitle() -> String {
    InspectorSessionPresenter.title(inspectorSessionSnapshot())
  }

  func inspectorSessionDetail() -> String {
    InspectorSessionPresenter.detail(inspectorSessionSnapshot())
  }

  func inspectorSessionMetaSummary() -> String {
    InspectorSessionPresenter.metaSummary(inspectorSessionSnapshot())
  }

  func shouldShowSetupCallout() -> Bool {
    runtimeState != .launching
      && (!isLocalModelReady() || workspace == nil || !hasRuntimeThreadSelection())
  }

  func setupCalloutTitle() -> String {
    SetupCalloutPresenter.title(setupCalloutSnapshot())
  }

  func setupCalloutSummary() -> String {
    SetupCalloutPresenter.summary(setupCalloutSnapshot())
  }

  func setupCalloutDetail() -> String {
    SetupCalloutPresenter.detail(setupCalloutSnapshot())
  }

  func setupCalloutTone() -> StatusTone {
    SetupCalloutPresenter.tone(setupCalloutSnapshot())
  }

  var trimmedDraftMessage: String {
    draftMessage.trimmingCharacters(in: .whitespacesAndNewlines)
  }
}
