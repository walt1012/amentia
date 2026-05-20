import AppKit

final class PithAppDelegate: NSObject, NSApplicationDelegate {
  func applicationDidFinishLaunching(_ notification: Notification) {
    NSWindow.allowsAutomaticWindowTabbing = false
  }

  func applicationShouldHandleReopen(
    _ sender: NSApplication,
    hasVisibleWindows flag: Bool
  ) -> Bool {
    if !flag {
      sender.windows.first?.makeKeyAndOrderFront(nil)
    }
    return false
  }
}
