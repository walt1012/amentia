@testable import AmentiaApp
import XCTest

final class SessionSearchPresenterTests: XCTestCase {
  func testEmptyQueryKeepsSessionsInOrder() {
    let sessions = [
      session(id: "one", title: "Plan launch"),
      session(id: "two", title: "Review notes"),
    ]

    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "   ").map(\.id),
      ["one", "two"]
    )
  }

  func testLocalWelcomeSessionStaysVisibleDuringSearch() {
    let sessions = [
      session(id: "local-welcome", title: "Welcome to Amentia"),
    ]

    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "missing").map(\.id),
      ["local-welcome"]
    )
  }

  func testSearchMatchesTitlePreviewAndProject() {
    let sessions = [
      session(
        id: "model",
        title: "Model setup",
        preview: "Ready to continue in Amentia.",
        workspaceDisplayName: "Amentia"
      ),
      session(
        id: "release",
        title: "Release packaging",
        preview: "Prepare the installer.",
        workspaceDisplayName: "Distribution"
      ),
    ]

    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "model").map(\.id),
      ["model"]
    )
    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "installer").map(\.id),
      ["release"]
    )
    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "distribution").map(\.id),
      ["release"]
    )
  }

  func testSearchMatchesAllTermsAndIgnoresCase() {
    let sessions = [
      session(
        id: "cowork",
        title: "Cowork polish",
        preview: "Ready to continue.",
        workspaceDisplayName: "Amentia"
      ),
      session(
        id: "workspace",
        title: "Workspace polish",
        preview: "Ready to continue.",
        workspaceDisplayName: "Amentia"
      ),
    ]

    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "AMENTIA cowork").map(\.id),
      ["cowork"]
    )
  }

  func testSearchMatchesWorkspaceFolderName() {
    let sessions = [
      session(
        id: "root",
        title: "Review",
        workspaceRootPath: "/Users/example/ClientDeck"
      ),
      session(
        id: "other",
        title: "Review",
        workspaceRootPath: "/Users/example/Amentia"
      ),
    ]

    XCTAssertEqual(
      SessionSearchPresenter.filteredSessions(sessions, query: "clientdeck").map(\.id),
      ["root"]
    )
  }

  private func session(
    id: String,
    title: String,
    preview: String = "Ready.",
    workspaceRootPath: String? = nil,
    workspaceDisplayName: String? = nil
  ) -> ThreadSummary {
    ThreadSummary(
      id: id,
      title: title,
      preview: preview,
      workspaceRootPath: workspaceRootPath,
      workspaceDisplayName: workspaceDisplayName
    )
  }
}
