@testable import PithApp
import XCTest

final class FirstRequestPromptPresenterTests: XCTestCase {
  func testFirstRequestCopyFramesCoworkSession() {
    XCTAssertTrue(FirstRequestPromptPresenter.calloutSummary().contains("Map Workspace"))
    XCTAssertTrue(FirstRequestPromptPresenter.calloutSummary().contains("Plan Next Step"))
    XCTAssertTrue(FirstRequestPromptPresenter.firstAppOpenActionSummary().contains("short cowork request"))
    XCTAssertTrue(
      FirstRequestPromptPresenter.calloutDetail(workspaceDisplayName: nil).contains("cowork session")
    )
    XCTAssertTrue(
      FirstRequestPromptPresenter.calloutDetail(workspaceDisplayName: "Pith").contains("cowork requests")
    )
  }

  func testFirstRequestSuggestionsOfferCoworkFirstStart() {
    let suggestions = FirstRequestPromptPresenter.suggestions(workspaceDisplayName: "Pith")

    XCTAssertEqual(suggestions.map(\.id), [
      FirstRequestPromptPresenter.mapWorkspaceID,
      FirstRequestPromptPresenter.planNextStepID,
    ])
    XCTAssertEqual(suggestions[1].title, "Plan Next Step")
    XCTAssertTrue(suggestions[1].message.contains("next useful step"))
    XCTAssertFalse(suggestions[1].message.lowercased().contains("git"))
  }

  func testFirstRequestActionTitlesNameConcretePrompts() {
    let suggestions = FirstRequestPromptPresenter.suggestions(workspaceDisplayName: "Pith")

    XCTAssertEqual(
      FirstRequestPromptPresenter.primaryActionTitle(for: suggestions[0]),
      "Map Workspace"
    )
    XCTAssertEqual(
      FirstRequestPromptPresenter.secondaryActionTitle(for: suggestions[1]),
      "Plan Next Step"
    )
  }
}
