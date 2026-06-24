import Foundation

enum PluginValidationCopy {
  static func userFacingMessage(_ message: String) -> String {
    let normalized = message.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !normalized.isEmpty else {
      return "Setup needs review."
    }

    if containsAny(normalized, [
      "does not contain amentia-plugin.json",
      "must be a plugin directory or amentia-plugin.json file",
      "Select a plugin folder or the amentia-plugin.json manifest",
    ]) {
      return "Plugin setup file is missing."
    }

    if normalized.contains("cannot contain nested amentia-plugin.json manifests") {
      return "Plugin bundle contains another plugin bundle."
    }

    if normalized.contains("cannot contain symbolic links") {
      return "Plugin bundle must be self-contained."
    }

    if normalized.contains("is already installed") {
      return "Plugin is already installed."
    }

    if normalized.contains("Plugin manifest name") {
      return "Plugin name needs review."
    }

    if containsAny(normalized, ["correct format", "is missing", "failed to parse"]) {
      return "Plugin setup file needs review."
    }

    if containsRawSetupDetail(normalized) {
      return "Plugin setup needs review."
    }

    return normalized
  }

  static func userFacingRepairHint(_ hint: String) -> String {
    let normalized = hint.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !normalized.isEmpty else {
      return "Review the plugin setup and try again."
    }

    if containsRawSetupDetail(normalized)
      || containsAny(normalized, ["correct format", "camelCase", "displayName"])
    {
      return "Check the plugin setup file, then refresh the plugin."
    }

    return normalized
  }

  private static func containsRawSetupDetail(_ value: String) -> Bool {
    value.contains("/")
      || value.contains("\\")
      || value.contains("amentia-plugin.json")
      || value.contains("manifest")
      || value.contains("Manifest")
      || value.contains("sourcePath")
      || value.contains("definitionPath")
  }

  private static func containsAny(_ value: String, _ needles: [String]) -> Bool {
    needles.contains { value.contains($0) }
  }
}
