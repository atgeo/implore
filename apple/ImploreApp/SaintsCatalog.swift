import Foundation

struct Saint: Codable, Identifiable, Hashable {
    let id: String
    let name: String
    let feast: String?
    let patronage: [String]?
    let summary: String?
}

private struct SaintsCatalogFile: Codable {
    let version: Int
    let locale: String
    let saints: [Saint]
}

@MainActor
final class SaintsCatalog: ObservableObject {
    static let shared = SaintsCatalog()

    private static let bucketBase =
        "https://atgeo-intercede-app-090552655796-us-east-2-an.s3.us-east-2.amazonaws.com"

    @Published private(set) var saints: [Saint] = []

    private let fileManager = FileManager.default
    private var loadTask: Task<Void, Never>?

    private init() {
        saints = loadCached(locale: "en") ?? []
    }

    func load(for language: AppLanguage) {
        loadTask?.cancel()
        let locale = Self.localeCode(for: language)
        if let cached = loadCached(locale: locale) {
            saints = cached
        }
        loadTask = Task {
            if let fetched = await fetch(locale: locale) {
                guard !Task.isCancelled else { return }
                saints = fetched
                saveCache(fetched, locale: locale)
            } else if locale != "en", let fallback = await fetch(locale: "en") {
                guard !Task.isCancelled else { return }
                saints = fallback
                saveCache(fallback, locale: "en")
            }
        }
    }

    func saint(for id: String?) -> Saint? {
        guard let id, !id.isEmpty else { return nil }
        return saints.first { $0.id == id }
    }

    private static func localeCode(for language: AppLanguage) -> String {
        switch language {
        case .system:
            Locale.current.language.languageCode?.identifier ?? "en"
        case .english: "en"
        case .french: "fr"
        case .spanish: "es"
        }
    }

    private func fetch(locale: String) async -> [Saint]? {
        guard let url = URL(string: "\(Self.bucketBase)/saints/\(locale).json") else {
            return nil
        }
        do {
            let (data, response) = try await URLSession.shared.data(from: url)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return nil
            }
            let catalog = try JSONDecoder().decode(SaintsCatalogFile.self, from: data)
            return catalog.saints.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        } catch {
            return nil
        }
    }

    private func cacheDirectory() -> URL {
        let support = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let dir = support.appendingPathComponent("saints", isDirectory: true)
        try? fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    private func cacheURL(locale: String) -> URL {
        cacheDirectory().appendingPathComponent("\(locale).json")
    }

    private func loadCached(locale: String) -> [Saint]? {
        let url = cacheURL(locale: locale)
        guard fileManager.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let catalog = try? JSONDecoder().decode(SaintsCatalogFile.self, from: data)
        else { return nil }
        return catalog.saints.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    private func saveCache(_ saints: [Saint], locale: String) {
        let catalog = SaintsCatalogFile(version: 1, locale: locale, saints: saints)
        guard let data = try? JSONEncoder().encode(catalog) else { return }
        try? data.write(to: cacheURL(locale: locale), options: .atomic)
    }
}
