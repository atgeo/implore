import SwiftUI

enum AppLanguage: String, CaseIterable, Identifiable {
    case system
    case english
    case french
    case spanish

    var id: String { rawValue }

    var locale: Locale? {
        switch self {
        case .system: nil
        case .english: Locale(identifier: "en")
        case .french: Locale(identifier: "fr")
        case .spanish: Locale(identifier: "es")
        }
    }

    @ViewBuilder
    var label: some View {
        switch self {
        case .system: Text("System")
        case .english: Text(verbatim: "English")
        case .french: Text(verbatim: "Français")
        case .spanish: Text(verbatim: "Español")
        }
    }
}

extension View {
    @ViewBuilder
    func appLocale(_ language: AppLanguage) -> some View {
        if let locale = language.locale {
            environment(\.locale, locale)
        } else {
            self
        }
    }
}
