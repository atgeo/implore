import App
import Foundation
import UserNotifications

enum PrayerReminderScheduler {
    static let identifierPrefix = "implore.digest"

    enum AuthorizationStatus {
        case authorized
        case notDetermined
        case denied
    }

    static func authorizationStatus() async -> AuthorizationStatus {
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        switch settings.authorizationStatus {
        case .authorized, .provisional, .ephemeral:
            return .authorized
        case .notDetermined:
            return .notDetermined
        default:
            return .denied
        }
    }

    static func requestAuthorization() async -> Bool {
        let center = UNUserNotificationCenter.current()
        let settings = await center.notificationSettings()
        switch settings.authorizationStatus {
        case .authorized, .provisional, .ephemeral:
            return true
        case .notDetermined:
            do {
                return try await center.requestAuthorization(options: [.alert, .sound, .badge])
            } catch {
                return false
            }
        default:
            return false
        }
    }

    static func reschedule(digests: [ReminderDigest], enabled: Bool) async {
        let center = UNUserNotificationCenter.current()
        let pending = await center.pendingNotificationRequests()
        let stale = pending
            .map(\.identifier)
            .filter { $0.hasPrefix(identifierPrefix) }
        center.removePendingNotificationRequests(withIdentifiers: stale)

        guard enabled else { return }

        let status = await authorizationStatus()
        guard status == .authorized else { return }

        for (index, digest) in digests.enumerated() {
            let content = UNMutableNotificationContent()
            content.title = String(localized: "Implore")
            content.body = body(for: digest.intentions)
            content.sound = .default

            var components = DateComponents()
            components.year = Int(digest.year)
            components.month = Int(digest.month)
            components.day = Int(digest.day)
            components.hour = Int(digest.hour)
            components.minute = Int(digest.minute)

            let trigger = UNCalendarNotificationTrigger(dateMatching: components, repeats: false)
            let request = UNNotificationRequest(
                identifier: "\(identifierPrefix).\(index)",
                content: content,
                trigger: trigger
            )
            try? await center.add(request)
        }
    }

    private static func body(for intentions: [String]) -> String {
        if intentions.count <= 3 {
            return intentions.joined(separator: " · ")
        }
        return String(localized: "\(intentions.count) intentions for today")
    }
}

enum LocalTimeSync {
    @MainActor
    static func sync(to core: Core) {
        let now = Date()
        let components = Calendar.current.dateComponents(
            [.year, .month, .day, .hour, .minute],
            from: now
        )
        core.update(
            .syncLocalTime(
                year: UInt16(components.year ?? 0),
                month: UInt8(components.month ?? 0),
                day: UInt8(components.day ?? 0),
                hour: UInt8(components.hour ?? 0),
                minute: UInt8(components.minute ?? 0)
            )
        )
    }
}
