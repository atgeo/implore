import App
import Foundation

enum TodaySelection {
    /// Intentions due today, least-recently-prayed first.
    static func prayers(from scheduled: [Prayer], now: Date = Date(), calendar: Calendar = .current)
        -> [Prayer]
    {
        var due = scheduled.filter { isDue($0, on: now, calendar: calendar) }
        due.sort { (lhs: Prayer, rhs: Prayer) in
            switch (lastPrayedRank(lhs), lastPrayedRank(rhs)) {
            case (nil, nil): return lhs.id < rhs.id
            case (nil, _): return true
            case (_, nil): return false
            case let (a?, b?):
                if a != b { return a.lexicographicallyPrecedes(b) }
                return lhs.id < rhs.id
            }
        }
        return due
    }

    static func isDue(_ prayer: Prayer, on date: Date, calendar: Calendar = .current) -> Bool {
        guard prayer.status == .active else { return false }
        switch prayer.cadence {
        case .unscheduled: return false
        case .daily: return true
        case .weekly: return calendar.component(.weekday, from: date) == 1
        case .monthly: return calendar.component(.day, from: date) == 1
        }
    }

    static func prayedToday(_ prayer: Prayer, on date: Date = Date(), calendar: Calendar = .current)
        -> Bool
    {
        let y = calendar.component(.year, from: date)
        let m = calendar.component(.month, from: date)
        let d = calendar.component(.day, from: date)
        return prayer.prayedOn.contains {
            Int($0.year) == y && Int($0.month) == m && Int($0.day) == d
        }
    }

    /// Sort key: earlier last prayer sorts first; never prayed sorts before any date.
    private static func lastPrayedRank(_ prayer: Prayer) -> [Int]? {
        guard let entry = prayer.prayedOn.last else { return nil }
        return [
            Int(entry.year),
            Int(entry.month),
            Int(entry.day),
            Int(entry.hour),
            Int(entry.minute),
        ]
    }
}
