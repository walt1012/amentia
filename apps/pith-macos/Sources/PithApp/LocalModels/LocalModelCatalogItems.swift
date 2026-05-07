import Foundation

extension LocalModelCatalog {
  static func items() -> [LocalModelCatalogItem] {
    [
      LocalModelCatalogItem(
        id: defaultFirstUseModelID,
        displayName: "LFM2.5-350M Q4_K_M",
        description: "Default tiny local model for the first Pith agent loop.",
        fileName: "LFM2.5-350M-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF/resolve/main/LFM2.5-350M-Q4_K_M.gguf",
        homepage: "https://huggingface.co/LiquidAI/LFM2.5-350M-GGUF",
        sizeBytes: 229_312_224,
        sha256: "7e6f72643caafc9a68256686638c4d7916f2cec76d1df478d4c3ddcd95a6aed4",
        contextSize: 4096,
        modelContextSize: 32_768,
        maxOutputTokens: 160,
        license: "lfm1.0",
        tags: ["default", "tiny", "edge"],
        installSegments: ["builtin", defaultFirstUseModelID]
      ),
      LocalModelCatalogItem(
        id: "granite-4.0-h-350m",
        displayName: "Granite 4.0-H-350M Q4_K_M",
        description: "Modern Apache-2.0 tiny model for local tool, code, and RAG workflows.",
        fileName: "granite-4.0-h-350m-Q4_K_M.gguf",
        downloadURL: "https://huggingface.co/ibm-granite/granite-4.0-h-350m-GGUF/resolve/main/granite-4.0-h-350m-Q4_K_M.gguf",
        homepage: "https://huggingface.co/ibm-granite/granite-4.0-h-350m-GGUF",
        sizeBytes: 222_662_560,
        sha256: "0a8d6a7373602fadfba274a640ba784b86cc6847f1c67f1b0a90fa2ec266b7fb",
        contextSize: 4096,
        modelContextSize: 32_768,
        maxOutputTokens: 192,
        license: "apache-2.0",
        tags: ["recommended", "tiny", "tools", "code"],
        installSegments: ["catalog", "granite-4.0-h-350m"]
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
