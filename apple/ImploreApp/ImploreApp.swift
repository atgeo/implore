import SwiftUI

@main
struct ImploreApp: App {
    @StateObject private var core = Core()
    @AppStorage("appearance") private var appearance = Appearance.system
    @AppStorage("language") private var language = AppLanguage.system

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .preferredColorScheme(appearance.colorScheme)
                .appLocale(language)
        }
    }
}
