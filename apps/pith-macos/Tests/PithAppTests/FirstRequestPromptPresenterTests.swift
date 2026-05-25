@testable import PithApp
import XCTest

final class FirstRequestPromptPresenterTests: XCTestCase {
  func testFirstRequestCopyFramesCoworkSession() {
    XCTAssertTrue(FirstRequestPromptPresenter.calloutSummary().contains("plan one useful next step"))
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
      "Use Map Prompt"
    )
    XCTAssertEqual(
      FirstRequestPromptPresenter.secondaryActionTitle(for: suggestions[1]),
      "Use Next Step Prompt"
    )
  }
}
