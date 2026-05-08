import Foundation

@MainActor
extension AppViewModel {
  func applyLocalModelActivationPlan(_ plan: LocalModelActivationPlan) {
    appendEntry(
      to: selectedThreadID,
      TimelineEventPresenter.localModelActivated(plan)
    )
    relaunchRuntimeIfNeeded(
      runningDetail: plan.relaunchRunningDetail,
      idleDetail: plan.relaunchIdleDetail
    )
  }

  func applyLocalModelActivationFailure(
    _ plan: LocalModelActivationFailurePlan,
    model: LocalModelSummary
  ) {
    if plan.removesModelFile {
      removeIncompleteModelFile(modelID: model.id)
    }
    if plan.refreshesCatalog {
      refreshLocalModelCatalog()
    }
    runtimeDetail = plan.runtimeDetail
  }

  func relaunchRuntimeIfNeeded(runningDetail: String, idleDetail: String) {
    let plan = RuntimeRelaunchPlanner.plan(
      runtimeState: runtimeState,
      runningDetail: runningDetail,
      idleDetail: idleDetail
    )
    runtimeDetail = plan.runtimeDetail

    switch plan.action {
    case .stopAndLaunch:
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
    case .stopAndLaunchAfterCurrentLaunchSettles:
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      Task {
        for _ in 0..<10 {
          if runtimeState != .launching {
            break
          }
          try? await Task.sleep(nanoseconds: 200_000_000)
        }
        if runtimeState == .launching {
          runtimeDetail = plan.launchTimeoutDetail ?? idleDetail
          return
        }
        launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
      }
    case .updateIdleDetail:
      break
    }
  }
}
