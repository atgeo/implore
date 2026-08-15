import App
import SwiftUI

extension IntentionCadence {
    /// Simple list caption; nil when unscheduled.
    var listLabel: LocalizedStringKey? {
        switch self {
        case .unscheduled: nil
        case .daily: "Daily"
        case .weekly: "Weekly"
        case .monthly: "Monthly"
        case .novena: "Novena"
        }
    }
}

enum NovenaScheduleLabel {
    static let spanDays = 9

    /// Earliest: 8 days ago (still mid-novena); latest: one year ahead.
    static func startDateRange(relativeTo now: Date = Date()) -> ClosedRange<Date> {
        let calendar = LocalTimeSync.civilCalendar
        let today = calendar.startOfDay(for: now)
        let earliest = calendar.date(byAdding: .day, value: -(spanDays - 1), to: today) ?? today
        let latest = calendar.date(byAdding: .year, value: 1, to: today) ?? today
        return earliest...latest
    }

    static func clampStart(_ date: Date, relativeTo now: Date = Date()) -> Date {
        let range = startDateRange(relativeTo: now)
        return min(max(date, range.lowerBound), range.upperBound)
    }

    /// Today / detail progress: Day N of 9, Novena (ended), or Novena.
    static func detailLabel(start: CivilDate?, localDate: CivilDate?) -> LocalizedStringKey {
        guard let start else { return "Novena" }
        guard let localDate,
              let startDate = LocalTimeSync.date(from: start),
              let onDate = LocalTimeSync.date(from: localDate)
        else {
            return "Novena"
        }

        let days = LocalTimeSync.civilCalendar
            .dateComponents([.day], from: startDate, to: onDate)
            .day ?? 0
        let day = days + 1
        if day >= 1, day <= spanDays {
            return "Day \(day) of \(spanDays)"
        }
        if day > spanDays {
            return "Novena (ended)"
        }
        return "Novena"
    }
}
