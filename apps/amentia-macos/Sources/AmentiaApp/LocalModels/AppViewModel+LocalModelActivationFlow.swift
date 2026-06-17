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
      runtimeRelaunchCoordinator.cancel()
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
    case .stopAndLaunchAfterCurrentLaunchSettles:
      let requestToken = runtimeRelaunchCoordinator.begin()
      runtimeBridge.stopRuntime(detail: plan.stopDetail ?? runningDetail)
      let task = Task {
        defer {
          runtimeRelaunchCoordinator.finish(requestToken)
        }
        for _ in 0..<10 {
          if runtimeState != .launching {
            break
          }
          try? await Task.sleep(nanoseconds: 200_000_000)
        }
        if runtimeState == .launching {
          guard runtimeRelaunchCoordinator.isCurrent(requestToken) else {
            return
          }
          runtimeDetail = plan.launchTimeoutDetail ?? idleDetail
          return
        }
        guard runtimeRelaunchCoordinator.isCurrent(requestToken) else {
          return
        }
        launchRuntime(launchDetail: plan.launchDetail ?? runningDetail)
      }
      runtimeRelaunchCoordinator.bind(task: task, token: requestToken)
    case .updateIdleDetail:
      runtimeRelaunchCoordinator.cancel()
      break
    }
  }
}

struct RuntimeRelaunchRequestToken: Equatable {
  fileprivate let id: UUID
}

final class RuntimeRelaunchCoordinator {
  private let taskSlot = CancellableTaskSlot()

  func begin() -> RuntimeRelaunchRequestToken {
    RuntimeRelaunchRequestToken(id: taskSlot.replace())
  }

  func bind(task: Task<Void, Never>, token: RuntimeRelaunchRequestToken) {
    taskSlot.bind(task: task, requestID: token.id)
  }

  func isCurrent(_ token: RuntimeRelaunchRequestToken) -> Bool {
    taskSlot.isCurrent(token.id)
  }

  func finish(_ token: RuntimeRelaunchRequestToken) {
    taskSlot.finish(token.id)
  }

  func cancel() {
    taskSlot.cancel()
  }
}
