import Foundation

enum PluginHookDisplay {
  static func statusLine(_ hook: PluginHookSummary) -> String {
    "\(hook.pluginDisplayName) | \(eventDetail(hook))"
  }

  static func eventDetail(_ hook: PluginHookSummary) -> String {
    switch hook.event {
    case "shell.completed":
      return "Runs after shell commands"
    case "after_action":
      return "Runs after actions"
    case "before_action":
      return "Runs before actions"
    default:
      return "Runs during plugin checks"
    }
  }

  static func disableButtonTitle(_ hook: PluginHookSummary) -> String {
    "Disable \(hook.pluginDisplayName)"
  }

  static func disableDetail(_ hook: PluginHookSummary) -> String {
    "Stops \(hook.pluginDisplayName) checks from running during future activity."
  }
}

enum PluginSkillDisplay {
  static func disableButtonTitle(_ skill: PluginSkillSummary) -> String {
    "Disable \(skill.pluginDisplayName)"
  }

  static func disableDetail(_ skill: PluginSkillSummary) -> String {
    "Stops \(skill.pluginDisplayName) guidance from being added to future requests."
  }

  static func previewLine(_ preview: String?, maxLength: Int = 160) -> String? {
    guard let preview else {
      return nil
    }

    let line = preview
      .split(whereSeparator: \.isNewline)
      .map(String.init)
      .first?
      .trimmingCharacters(in: .whitespacesAndNewlines)

    guard let line, !line.isEmpty else {
      return nil
    }

    if line.count <= maxLength {
      return line
    }

    return "\(String(line.prefix(maxLength)))..."
  }

  static func issueText(_ skill: PluginSkillSummary) -> String? {
    guard skill.status != "ready" else {
      return nil
    }

    let blocker = skill.runBlocker?.trimmingCharacters(in: .whitespacesAndNewlines)
    let hint = skill.runRepairHint?.trimmingCharacters(in: .whitespacesAndNewlines)

    switch (nonEmpty(blocker), nonEmpty(hint)) {
    case let (blocker?, hint?):
      return "\(blocker) Fix: \(hint)"
    case let (blocker?, nil):
      return blocker
    case let (nil, hint?):
      return "Fix: \(hint)"
    case (nil, nil):
      return "This guidance needs review before Amentia can use it."
    }
  }

  private static func nonEmpty(_ value: String?) -> String? {
    guard let value, !value.isEmpty else {
      return nil
    }
    return value
  }
}
