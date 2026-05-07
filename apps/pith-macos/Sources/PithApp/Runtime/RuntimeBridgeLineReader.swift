import Foundation

enum RuntimeBridgeLineReader {
  static func readLine(from handle: FileHandle) throws -> String {
    var data = Data()

    while true {
      let chunk = try handle.read(upToCount: 1) ?? Data()

      if chunk.isEmpty {
        break
      }

      if chunk == Data([0x0A]) {
        break
      }

      data.append(chunk)
    }

    guard !data.isEmpty else {
      throw RuntimeBridge.RuntimeError.invalidResponse
    }

    return String(decoding: data, as: UTF8.self)
  }
}
