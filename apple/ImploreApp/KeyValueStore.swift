import Foundation

/// Local key-value store backed by Application Support.
/// Session tokens are stored in the Keychain instead of on disk.
final class KeyValueStore {
    private static let sessionKey = "session"

    private let directory: URL
    private let fileManager = FileManager.default

    init(directoryName: String = "ImploreKV") {
        let support = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        directory = support.appendingPathComponent(directoryName, isDirectory: true)
        try? fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    func get(key: String) -> Data? {
        if key == Self.sessionKey {
            if let data = KeychainStore.get(key: key) {
                return data
            }
            if let data = readFile(key: key) {
                KeychainStore.set(key: key, value: data)
                deleteFile(key: key)
                return data
            }
            return nil
        }
        return readFile(key: key)
    }

    func set(key: String, value: Data) {
        if key == Self.sessionKey {
            KeychainStore.set(key: key, value: value)
            deleteFile(key: key)
            return
        }
        writeFile(key: key, value: value)
    }

    func delete(key: String) {
        if key == Self.sessionKey {
            KeychainStore.delete(key: key)
        }
        deleteFile(key: key)
    }

    func exists(key: String) -> Bool {
        if key == Self.sessionKey {
            return KeychainStore.exists(key: key) || fileManager.fileExists(atPath: fileURL(for: key).path)
        }
        return fileManager.fileExists(atPath: fileURL(for: key).path)
    }

    func listKeys(prefix: String, cursor: UInt64) -> [String] {
        guard let contents = try? fileManager.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: nil
        ) else {
            return []
        }

        var keys = contents
            .map(\.lastPathComponent)
            .compactMap(decodeKey)
            .filter { $0.hasPrefix(prefix) }

        if Self.sessionKey.hasPrefix(prefix),
           KeychainStore.exists(key: Self.sessionKey),
           !keys.contains(Self.sessionKey)
        {
            keys.append(Self.sessionKey)
        }
        keys.sort()

        if cursor == 0 {
            return keys
        }

        let start = Int(cursor)
        guard start < keys.count else { return [] }
        return Array(keys[start...])
    }

    private func readFile(key: String) -> Data? {
        let url = fileURL(for: key)
        guard fileManager.fileExists(atPath: url.path) else { return nil }
        return try? Data(contentsOf: url)
    }

    private func writeFile(key: String, value: Data) {
        let url = fileURL(for: key)
        try? value.write(to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
    }

    private func deleteFile(key: String) {
        let url = fileURL(for: key)
        try? fileManager.removeItem(at: url)
    }

    private func fileURL(for key: String) -> URL {
        directory.appendingPathComponent(encodeKey(key), isDirectory: false)
    }

    private func encodeKey(_ key: String) -> String {
        Data(key.utf8).base64EncodedString()
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "+", with: "-")
    }

    private func decodeKey(_ filename: String) -> String? {
        let base64 = filename
            .replacingOccurrences(of: "_", with: "/")
            .replacingOccurrences(of: "-", with: "+")
        guard let data = Data(base64Encoded: base64) else { return nil }
        return String(data: data, encoding: .utf8)
    }
}
