import Foundation

func makeTemporaryDirectory(prefix: String) throws -> URL {
  let rootURL = FileManager.default.temporaryDirectory
    .appendingPathComponent("\(prefix)-\(UUID().uuidString)", isDirectory: true)
  try FileManager.default.createDirectory(at: rootURL, withIntermediateDirectories: true)
  return rootURL
}
