import App
import SwiftUI

extension LiturgicalDay {
    /// Localized temporal-cycle title for Today. Named observances come from the catalog.
    func title(locale: Locale) -> LocalizedStringKey {
        switch self {
        case let .ordinaryTime(week, weekday):
            if weekday == 1 {
                "\(ordinal(week, locale: locale)) Sunday in Ordinary Time"
            } else {
                "\(weekdayName(weekday, locale: locale)) of the \(ordinal(week, locale: locale)) Week in Ordinary Time"
            }
        case let .advent(week, weekday):
            if weekday == 1 {
                "\(ordinal(week, locale: locale)) Sunday of Advent"
            } else {
                "\(weekdayName(weekday, locale: locale)) of the \(ordinal(week, locale: locale)) Week of Advent"
            }
        case let .lent(week, weekday):
            if weekday == 1 {
                "\(ordinal(week, locale: locale)) Sunday of Lent"
            } else {
                "\(weekdayName(weekday, locale: locale)) of the \(ordinal(week, locale: locale)) Week of Lent"
            }
        case let .easter(week, weekday):
            if weekday == 1 {
                "\(ordinal(week, locale: locale)) Sunday of Easter"
            } else {
                "\(weekdayName(weekday, locale: locale)) of the \(ordinal(week, locale: locale)) Week of Easter"
            }
        case let .afterAshWednesday(weekday):
            "\(weekdayName(weekday, locale: locale)) after Ash Wednesday"
        case let .holyWeek(weekday):
            "\(weekdayName(weekday, locale: locale)) of Holy Week"
        case .christmasSeason:
            "Christmas Weekday"
        case .christmasDay:
            "Christmas"
        case .holyFamily:
            "The Holy Family"
        case .epiphany:
            "The Epiphany of the Lord"
        case .baptismOfTheLord:
            "The Baptism of the Lord"
        case .ashWednesday:
            "Ash Wednesday"
        case .palmSunday:
            "Palm Sunday"
        case .holyThursday:
            "Holy Thursday"
        case .goodFriday:
            "Good Friday"
        case .holySaturday:
            "Holy Saturday"
        case .easterSunday:
            "Easter Sunday"
        case .pentecost:
            "Pentecost Sunday"
        case .trinitySunday:
            "The Most Holy Trinity"
        case .corpusChristi:
            "The Most Holy Body and Blood of Christ"
        case .sacredHeart:
            "The Most Sacred Heart of Jesus"
        case .christTheKing:
            "Our Lord Jesus Christ, King of the Universe"
        }
    }

    private func weekdayName(_ weekday: UInt32, locale: Locale) -> String {
        var calendar = Calendar(identifier: .gregorian)
        calendar.locale = locale
        let index = min(max(Int(weekday), 1), 7) - 1
        return calendar.standaloneWeekdaySymbols[index]
    }

    private func ordinal(_ week: UInt32, locale: Locale) -> String {
        let formatter = NumberFormatter()
        formatter.locale = locale
        formatter.numberStyle = .ordinal
        return formatter.string(from: NSNumber(value: week)) ?? String(week)
    }
}
