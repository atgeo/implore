import Foundation

struct Observance: Codable, Identifiable, Hashable {
    let id: String
    let name: String
    let companion: Bool
    let date: String?
    let rank: String?
    let patronage: [String]?
    let summary: String?
}

private struct ObservancesCatalogFile: Codable {
    let version: Int
    let locale: String
    let observances: [Observance]
}

@MainActor
final class ObservancesCatalog: ObservableObject {
    static let shared = ObservancesCatalog()

    private static let bucketBase =
        "https://atgeo-intercede-app-090552655796-us-east-2-an.s3.us-east-2.amazonaws.com"

    @Published private(set) var observances: [Observance] = []

    private let fileManager = FileManager.default
    private var loadTask: Task<Void, Never>?

    private init() {
        observances = loadCached(locale: "en") ?? loadBundled(locale: "en") ?? []
    }

    /// Companions for the intention picker (`companion == true`), sorted by name.
    var companions: [Observance] {
        observances
            .filter(\.companion)
            .sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
    }

    func load(for language: AppLanguage) {
        loadTask?.cancel()
        let locale = Self.localeCode(for: language)
        if let cached = loadCached(locale: locale) {
            observances = cached
        } else if let bundled = loadBundled(locale: locale) {
            observances = bundled
        } else if locale != "en", let bundledEn = loadBundled(locale: "en") {
            observances = bundledEn
        }
        loadTask = Task {
            if let fetched = await fetch(locale: locale) {
                guard !Task.isCancelled else { return }
                observances = fetched
                saveCache(fetched, locale: locale)
            } else if locale != "en", let fallback = await fetch(locale: "en") {
                guard !Task.isCancelled else { return }
                observances = fallback
                saveCache(fallback, locale: "en")
            }
        }
    }

    func companion(for id: String?) -> Observance? {
        guard let id, !id.isEmpty else { return nil }
        return observances.first { $0.companion && $0.id == id }
    }

    func observances(onMonthDay monthDay: String) -> [Observance] {
        observances
            .filter { $0.date == monthDay }
            .sorted(by: Self.rankSort)
    }

    private static func rankSort(_ a: Observance, _ b: Observance) -> Bool {
        let ra = rankWeight(a.rank)
        let rb = rankWeight(b.rank)
        if ra != rb { return ra < rb }
        return a.name.localizedCaseInsensitiveCompare(b.name) == .orderedAscending
    }

    private static func rankWeight(_ rank: String?) -> Int {
        switch rank {
        case "solemnity": 0
        case "feast": 1
        case "memorial": 2
        case "commemoration": 3
        default: 4
        }
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

    private func fetch(locale: String) async -> [Observance]? {
        guard let url = URL(string: "\(Self.bucketBase)/observances/\(locale).json") else {
            return nil
        }
        do {
            let (data, response) = try await URLSession.shared.data(from: url)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return nil
            }
            let catalog = try JSONDecoder().decode(ObservancesCatalogFile.self, from: data)
            return catalog.observances
        } catch {
            return nil
        }
    }

    private func cacheDirectory() -> URL {
        let support = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        let dir = support.appendingPathComponent("observances", isDirectory: true)
        try? fileManager.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir
    }

    private func cacheURL(locale: String) -> URL {
        cacheDirectory().appendingPathComponent("\(locale).json")
    }

    private func loadCached(locale: String) -> [Observance]? {
        let url = cacheURL(locale: locale)
        guard fileManager.fileExists(atPath: url.path),
              let data = try? Data(contentsOf: url),
              let catalog = try? JSONDecoder().decode(ObservancesCatalogFile.self, from: data)
        else { return nil }
        return catalog.observances
    }

    private func loadBundled(locale: String) -> [Observance]? {
        let url =
            Bundle.main.url(forResource: locale, withExtension: "json", subdirectory: "observances")
            ?? Bundle.main.url(forResource: locale, withExtension: "json")
        guard let url,
              let data = try? Data(contentsOf: url),
              let catalog = try? JSONDecoder().decode(ObservancesCatalogFile.self, from: data)
        else { return nil }
        return catalog.observances
    }

    private func saveCache(_ observances: [Observance], locale: String) {
        let catalog = ObservancesCatalogFile(version: 1, locale: locale, observances: observances)
        guard let data = try? JSONEncoder().encode(catalog) else { return }
        try? data.write(to: cacheURL(locale: locale), options: .atomic)
    }
}
