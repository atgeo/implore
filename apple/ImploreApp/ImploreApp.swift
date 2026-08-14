import SwiftUI

@main
struct ImploreApp: App {
    @Environment(\.scenePhase) private var scenePhase
    @StateObject private var core = Core()
    @AppStorage("appearance") private var appearance = Appearance.system
    @AppStorage("language") private var language = AppLanguage.system

    var body: some Scene {
        WindowGroup {
            ContentView(core: core)
                .preferredColorScheme(appearance.colorScheme)
                .appLocale(language)
                .task {
                    LocalTimeSync.sync(to: core)
                    ObservancesCatalog.shared.load(for: language)
                    await syncReminders()
                }
                .onChange(of: language) { _, newLanguage in
                    ObservancesCatalog.shared.load(for: newLanguage)
                }
                .onChange(of: core.view.reminderDigests) { _, _ in
                    Task { await syncReminders() }
                }
                .onChange(of: core.view.reminderSettings.enabled) { _, _ in
                    Task { await syncReminders() }
                }
                .onChange(of: scenePhase) { _, phase in
                    if phase == .active {
                        LocalTimeSync.sync(to: core)
                    }
                }
        }
    }

    @MainActor
    private func syncReminders() async {
        await PrayerReminderScheduler.reschedule(
            digests: core.view.reminderDigests,
            enabled: core.view.reminderSettings.enabled
        )
    }
}
