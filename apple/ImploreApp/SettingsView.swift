import App
import SwiftUI

struct SettingsView: View {
    @ObservedObject var core: Core
    @AppStorage("appearance") private var appearance = Appearance.system
    @AppStorage("language") private var language = AppLanguage.system
    @State private var notificationsDenied = false
    @State private var confirmRemoveAll = false
    @State private var showPrivacyPolicy = false

    var body: some View {
        Form {
            Section {
                Toggle("Reminders", isOn: remindersEnabledBinding)
                    .paperCardRow()

                if core.view.reminderSettings.enabled {
                    HStack {
                        Text("Time")
                        Spacer()
                        QuarterHourTimePicker(date: reminderTimeBinding)
                    }
                    .paperCardRow()
                }
            } footer: {
                if notificationsDenied {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Notifications are off for Implore. Turn them on in Settings to receive reminders.")
                        Button("Open Settings") {
                            if let url = URL(string: UIApplication.openSettingsURLString) {
                                UIApplication.shared.open(url)
                            }
                        }
                    }
                } else {
                    Text("One daily digest for intentions due that day. Weekly on Sundays, monthly on the 1st.")
                }
            }

            Section {
                Picker("Appearance", selection: $appearance) {
                    ForEach(Appearance.allCases) { option in
                        Text(option.title).tag(option)
                    }
                }
                .paperCardRow()

                Picker("Language", selection: $language) {
                    ForEach(AppLanguage.allCases) { option in
                        option.label.tag(option)
                    }
                }
                .paperCardRow()
            }

            Section {
                Button("Remove All Intentions", role: .destructive) {
                    confirmRemoveAll = true
                }
                .disabled(!core.view.hasPrayers)
                .paperCardRow()
            } footer: {
                Text("Permanently deletes every intention, including archived ones.")
            }

            Section {
                Button("Privacy Policy") {
                    showPrivacyPolicy = true
                }
                .foregroundStyle(.primary)
                .paperCardRow()

                LabeledContent("Version", value: appVersion)
                    .paperCardRow()
            } header: {
                Text("About")
            }
        }
        .paperBackground()
        .navigationTitle("Settings")
        .toolbar(.hidden, for: .tabBar)
        .sheet(isPresented: $showPrivacyPolicy) {
            SafariView(url: Self.privacyPolicyURL)
                .ignoresSafeArea()
        }
        .alert(
            "Remove All Intentions?",
            isPresented: $confirmRemoveAll
        ) {
            Button("Remove All Intentions", role: .destructive) {
                core.update(.removeAllPrayers)
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This cannot be undone.")
        }
        .task {
            await refreshAuthorizationStatus()
        }
    }

    private static let privacyPolicyURL = URL(string: "https://atgeo.github.io/implore/privacy")!

    private var remindersEnabledBinding: Binding<Bool> {
        Binding(
            get: { core.view.reminderSettings.enabled },
            set: { newValue in
                Task { @MainActor in
                    if newValue {
                        let allowed = await PrayerReminderScheduler.requestAuthorization()
                        notificationsDenied = !allowed
                        guard allowed else { return }
                        applyReminderSettings(
                            enabled: true,
                            hour: core.view.reminderSettings.hour,
                            minute: core.view.reminderSettings.minute
                        )
                    } else {
                        notificationsDenied = false
                        applyReminderSettings(
                            enabled: false,
                            hour: core.view.reminderSettings.hour,
                            minute: core.view.reminderSettings.minute
                        )
                    }
                }
            }
        )
    }

    private var reminderTimeBinding: Binding<Date> {
        Binding(
            get: {
                Calendar.current.date(
                    from: DateComponents(
                        hour: Int(core.view.reminderSettings.hour),
                        minute: Int(core.view.reminderSettings.minute)
                    )
                ) ?? Date()
            },
            set: { newValue in
                let components = Calendar.current.dateComponents([.hour, .minute], from: newValue)
                applyReminderSettings(
                    enabled: core.view.reminderSettings.enabled,
                    hour: UInt8(components.hour ?? 8),
                    minute: UInt8(components.minute ?? 0)
                )
            }
        )
    }

    private func applyReminderSettings(enabled: Bool, hour: UInt8, minute: UInt8) {
        LocalTimeSync.sync(to: core)
        core.update(
            .setReminderSettings(
                enabled: enabled,
                hour: hour,
                minute: minute
            )
        )
    }

    private func refreshAuthorizationStatus() async {
        let status = await PrayerReminderScheduler.authorizationStatus()
        notificationsDenied = core.view.reminderSettings.enabled && status == .denied
    }

    private var appVersion: String {
        let build = Bundle.main
            .object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "—"
        return "\(core.view.version) (\(build))"
    }
}

#Preview {
    NavigationStack {
        SettingsView(core: Core())
    }
}
