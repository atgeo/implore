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
    /// Gregorian civil calendar in the device time zone (matches Rust `CivilDate`).
    /// Week starts on Sunday (`firstWeekday = 1`) so `weekOfYear` matches core cadence.
    static var civilCalendar: Calendar {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = .current
        calendar.firstWeekday = 1
        return calendar
    }

    @MainActor
    static func sync(to core: Core) {
        let components = civilCalendar.dateComponents(
            [.year, .month, .day, .hour, .minute],
            from: Date()
        )
        guard let year = components.year,
              let month = components.month,
              let day = components.day,
              let hour = components.hour,
              let minute = components.minute,
              (0...Int(UInt16.max)).contains(year),
              (1...12).contains(month),
              (1...31).contains(day),
              (0...23).contains(hour),
              (0...59).contains(minute)
        else { return }

        core.update(
            .syncLocalTime(
                year: UInt16(year),
                month: UInt8(month),
                day: UInt8(day),
                hour: UInt8(hour),
                minute: UInt8(minute),
                unixSeconds: UInt64(Date().timeIntervalSince1970.rounded(.down))
            )
        )
    }

    static func date(from civil: CivilDate) -> Date? {
        var components = DateComponents()
        components.year = Int(civil.year)
        components.month = Int(civil.month)
        components.day = Int(civil.day)
        return civilCalendar.date(from: components)
    }

    static func civilDate(from date: Date) -> CivilDate? {
        let components = civilCalendar.dateComponents([.year, .month, .day], from: date)
        guard let year = components.year,
              let month = components.month,
              let day = components.day,
              (1...12).contains(month),
              (1...31).contains(day)
        else { return nil }
        return CivilDate(year: Int32(year), month: UInt32(month), day: UInt32(day))
    }

    static func date(from entry: PrayerLogEntry) -> Date? {
        var components = DateComponents()
        components.year = Int(entry.year)
        components.month = Int(entry.month)
        components.day = Int(entry.day)
        components.hour = Int(entry.hour)
        components.minute = Int(entry.minute)
        return civilCalendar.date(from: components)
    }
}
