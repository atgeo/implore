import SwiftUI

@main
struct ImploreApp: App {
    @StateObject private var core = Core()
    @AppStorage("appearance") private var appearance = Appearance.system

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .preferredColorScheme(appearance.colorScheme)
        }
    }
}
