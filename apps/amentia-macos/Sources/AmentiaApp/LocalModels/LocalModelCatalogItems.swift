import Foundation

extension LocalModelCatalog {
  static func items() -> [LocalModelCatalogItem] {
    [
      LocalModelCatalogItem(
        id: defaultFirstUseModelID,
        displayName: "Granite 4.0-H-350M Q4_K_M",
        description: "Default lightweight model for fast local setup and everyday cowork.",
        fileName: "granite-4.0-h-350m-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/ibm-granite/granite-4.0-h-350m-GGUF/resolve/main/granite-4.0-h-350m-Q4_K_M.gguf",
        homepage: "https://huggingface.co/ibm-granite/granite-4.0-h-350m-GGUF",
        sizeBytes: 222_662_560,
        sha256: "0a8d6a7373602fadfba274a640ba784b86cc6847f1c67f1b0a90fa2ec266b7fb",
        contextSize: 4096,
        modelContextSize: 32_768,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["default", "recommended", "tiny", "tools", "code"],
        installSegments: ["builtin", defaultFirstUseModelID]
      ),
      LocalModelCatalogItem(
        id: "minicpm5-1b",
        displayName: "MiniCPM5-1B Q4_K_M",
        description: "Stronger local model for larger files, longer sessions, and deeper project help.",
        fileName: "MiniCPM5-1B-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/openbmb/MiniCPM5-1B-GGUF/resolve/main/MiniCPM5-1B-Q4_K_M.gguf",
        homepage: "https://huggingface.co/openbmb/MiniCPM5-1B-GGUF",
        sizeBytes: 688_065_920,
        sha256: "81b64d05a23b17b34c475f42b3e72fbde62d4b92cc34541f7a8031d0752deafa",
        contextSize: 8192,
        modelContextSize: 131_072,
        maxOutputTokens: 384,
        license: "apache-2.0",
        tags: ["optional", "small", "tools", "code", "long-context"],
        installSegments: ["catalog", "minicpm5-1b"]
      ),
    ]
  }
}

struct LocalModelCatalogItem {
  let id: String
  let displayName: String
  let description: String
  let fileName: String
  let downloadURL: String
  let homepage: String
  let sizeBytes: Int64
  let sha256: String
  let contextSize: Int
  let modelContextSize: Int
  let maxOutputTokens: Int
  let license: String
  let tags: [String]
  let installSegments: [String]

  func installPath(storageRootPath: String) -> String {
    installSegments.reduce(URL(fileURLWithPath: storageRootPath, isDirectory: true)) { url, segment in
      url.appendingPathComponent(segment)
    }
    .appendingPathComponent(fileName)
    .path
  }
}
