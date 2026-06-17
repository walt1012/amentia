import AppKit

final class PithAppDelegate: NSObject, NSApplicationDelegate {
  private static weak var mainWindow: NSWindow?
  private static let mainWindowIdentifier = NSUserInterfaceItemIdentifier("pith.main-window")

  func applicationDidFinishLaunching(_ notification: Notification) {
    NSWindow.allowsAutomaticWindowTabbing = false
  }

  func applicationShouldOpenUntitledFile(_ sender: NSApplication) -> Bool {
    !Self.showMainWindow()
  }

  func applicationShouldHandleReopen(
    _ sender: NSApplication,
    hasVisibleWindows flag: Bool
  ) -> Bool {
    if !flag {
      return !Self.showMainWindow()
    }
    return false
  }

  static func registerMainWindow(_ window: NSWindow) {
    window.identifier = mainWindowIdentifier
    window.tabbingMode = .disallowed
    window.title = "Amentia"

    if let existingWindow = mainWindow,
       existingWindow !== window,
       existingWindow.isVisible
    {
      existingWindow.makeKeyAndOrderFront(nil)
      window.close()
      return
    }

    mainWindow = window
  }

  @discardableResult
  private static func showMainWindow() -> Bool {
    if let window = mainWindow {
      window.makeKeyAndOrderFront(nil)
      return true
    }

    let candidate = NSApp.windows.first { window in
      window.identifier == mainWindowIdentifier || window.canBecomeMain
    }
    candidate?.makeKeyAndOrderFront(nil)
    return candidate != nil
  }
}
