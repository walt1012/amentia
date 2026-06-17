@testable import AmentiaApp
import XCTest

final class FirstRequestPromptPresenterTests: XCTestCase {
  func testFirstRequestContractNamesPackagedFirstOpenAction() {
    XCTAssertEqual(
      FirstRequestPromptPresenter.firstAppOpenActionContractID,
      "map-plan-or-short-cowork-prompt"
    )
    XCTAssertTrue(
      FirstRequestPromptPresenter.firstAppOpenActionTrustSummary().contains("Understand Project")
    )
    XCTAssertTrue(
      FirstRequestPromptPresenter.firstAppOpenActionTrustSummary().contains("short cowork prompt")
    )
  }

  func testFirstRequestCopyFramesCoworkSession() {
    XCTAssertTrue(FirstRequestPromptPresenter.calloutSummary().contains("Understand Project"))
    XCTAssertTrue(FirstRequestPromptPresenter.calloutSummary().contains("Pick Next Step"))
    XCTAssertTrue(FirstRequestPromptPresenter.firstAppOpenActionSummary().contains("short cowork prompt"))
    XCTAssertTrue(
      FirstRequestPromptPresenter.calloutDetail(workspaceDisplayName: nil).contains("cowork session")
    )
    XCTAssertTrue(
      FirstRequestPromptPresenter.calloutDetail(workspaceDisplayName: "Amentia").contains("cowork prompts")
    )
  }

  func testFirstRequestSuggestionsOfferCoworkFirstStart() {
    let suggestions = FirstRequestPromptPresenter.suggestions(workspaceDisplayName: "Amentia")

    XCTAssertEqual(suggestions.map(\.id), [
      FirstRequestPromptPresenter.mapWorkspaceID,
      FirstRequestPromptPresenter.planNextStepID,
    ])
    XCTAssertEqual(suggestions[1].title, "Pick Next Step")
    XCTAssertTrue(suggestions[1].message.contains("next useful step"))
    XCTAssertFalse(suggestions[1].message.lowercased().contains("git"))
  }

  func testFirstRequestActionTitlesNameConcretePrompts() {
    let suggestions = FirstRequestPromptPresenter.suggestions(workspaceDisplayName: "Amentia")

    XCTAssertEqual(
      FirstRequestPromptPresenter.primaryActionTitle(for: suggestions[0]),
      "Understand Project"
    )
    XCTAssertEqual(
      FirstRequestPromptPresenter.secondaryActionTitle(for: suggestions[1]),
      "Pick Next Step"
    )
  }
}
