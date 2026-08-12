import App
import SwiftUI

struct IntentionDetailView: View {
    @ObservedObject var core: Core
    @ObservedObject private var saintsCatalog = SaintsCatalog.shared

    @Environment(\.locale) private var locale

    private let prayerId: UInt64
    private let fallback: Prayer

    init(core: Core, prayer: Prayer) {
        self.core = core
        self.prayerId = prayer.id
        self.fallback = prayer
    }

    private var prayer: Prayer {
        core.view.prayers.first { $0.id == prayerId } ?? fallback
    }

    var body: some View {
        Form {
            Section {
                Text(prayer.intention)
            } header: {
                Text("Intention")
            }

            if let details = prayer.details, !details.isEmpty {
                Section {
                    Text(details)
                } header: {
                    Text("Details")
                }
            }

            if !prayer.tags.isEmpty {
                Section {
                    Text(prayer.tags.joined(separator: " · "))
                        .foregroundStyle(.secondary)
                } header: {
                    Text("Tags")
                }
            }

            if let saint = saintsCatalog.saint(for: prayer.saintId) {
                Section {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(saint.name)
                        if let summary = saint.summary, !summary.isEmpty {
                            Text(summary)
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                        }
                    }
                } header: {
                    Text("Saint")
                }
            }

            if prayer.status == .archived {
                Section {
                    Text("Archived")
                        .foregroundStyle(.secondary)
                }
            } else {
                Section {
                    Text(cadenceTitle)
                } header: {
                    Text("Schedule")
                }
            }

            if prayer.status == .active {
                Section {
                    PrayerLogAction(action: addPrayerLogEntry)
                } header: {
                    Text("Prayer")
                }

                prayerLogSection(allowDelete: true)
            } else {
                prayerLogSection(allowDelete: true)
            }
        }
        .navigationTitle(prayer.intention)
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                NavigationLink {
                    AddIntentionView(core: core, prayer: prayer)
                } label: {
                    Text("Edit")
                }
            }
        }
        .onAppear {
            LocalTimeSync.sync(to: core)
        }
    }

    @ViewBuilder
    private func prayerLogSection(allowDelete: Bool) -> some View {
        if !prayerLogEntries.isEmpty {
            Section {
                ForEach(prayerLogEntries, id: \.index) { entry in
                    prayerLogLabel(for: entry.entry)
                        .foregroundStyle(.secondary)
                        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                            if allowDelete {
                                Button(role: .destructive) {
                                    removePrayerLogEntry(at: entry.index)
                                } label: {
                                    Label("Delete", systemImage: "trash")
                                }
                            }
                        }
                }
            } header: {
                Text("Prayer log")
            } footer: {
                prayerLogFooter
            }
        }
    }

    private var prayerLogFooter: Text {
        let count = prayer.prayedOn.count
        switch prayerLogSpan {
        case .today:
            return Text("\(count) times today")
        case .thisWeek:
            return Text("\(count) times this week")
        case .thisMonth:
            return Text("\(count) times this month")
        case .overall:
            return Text("\(count) times")
        }
    }

    /// Relative for recent entries, and the year is dropped inside the current year.
    private func prayerLogLabel(for entry: PrayerLogEntry) -> Text {
        guard let date = date(from: entry) else {
            return Text(verbatim: "\(entry.month)/\(entry.day)/\(entry.year)")
        }

        let calendar = Calendar.current
        let time = date.formatted(.dateTime.hour().minute().locale(locale))

        if calendar.isDateInToday(date) {
            return Text("Today, \(time)")
        }
        if calendar.isDateInYesterday(date) {
            return Text("Yesterday, \(time)")
        }

        var style = Date.FormatStyle.dateTime.month(.abbreviated).day().hour().minute()
        if !calendar.isDate(date, equalTo: Date(), toGranularity: .year) {
            style = style.year()
        }
        return Text(verbatim: date.formatted(style.locale(locale)))
    }

    private var prayerLogSpan: PrayerLogSpan {
        let calendar = Calendar.current
        let now = Date()
        let dates = prayer.prayedOn.compactMap(date(from:))
        guard !dates.isEmpty else { return .overall }

        if dates.allSatisfy({ calendar.isDateInToday($0) }) {
            return .today
        }
        if dates.allSatisfy({ calendar.isDate($0, equalTo: now, toGranularity: .weekOfYear) }) {
            return .thisWeek
        }
        if dates.allSatisfy({ calendar.isDate($0, equalTo: now, toGranularity: .month) }) {
            return .thisMonth
        }
        return .overall
    }

    private func date(from entry: PrayerLogEntry) -> Date? {
        var components = DateComponents()
        components.year = Int(entry.year)
        components.month = Int(entry.month)
        components.day = Int(entry.day)
        components.hour = Int(entry.hour)
        components.minute = Int(entry.minute)
        return Calendar.current.date(from: components)
    }

    /// Newest entries first; `index` is the stable index in `prayedOn`.
    private var prayerLogEntries: [(index: UInt64, entry: PrayerLogEntry)] {
        prayer.prayedOn.enumerated().map { index, entry in
            (UInt64(index), entry)
        }.reversed()
    }

    private var cadenceTitle: LocalizedStringKey {
        switch prayer.cadence {
        case .unscheduled: "No schedule"
        case .daily: "Daily"
        case .weekly: "Weekly"
        case .monthly: "Monthly"
        }
    }

    private func addPrayerLogEntry() {
        LocalTimeSync.sync(to: core)
        core.update(.logPrayer(id: prayer.id))
    }

    private func removePrayerLogEntry(at index: UInt64) {
        core.update(.removePrayerLogEntry(id: prayer.id, index: index))
    }
}

private enum PrayerLogSpan {
    case today
    case thisWeek
    case thisMonth
    case overall
}

private struct PrayerLogAction: View {
    let action: () -> Void

    @State private var loggedCount = 0

    var body: some View {
        Button {
            loggedCount += 1
            action()
        } label: {
            HStack(spacing: 12) {
                Image(systemName: "hands.and.sparkles.fill")
                    .font(.body)
                    .symbolEffect(.bounce, value: loggedCount)

                Text("Prayed")
                    .font(.body.weight(.semibold))

                Spacer(minLength: 0)
            }
            .foregroundStyle(Color.accentColor)
            .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .sensoryFeedback(.success, trigger: loggedCount)
    }
}

#Preview("With entries") {
    NavigationStack {
        IntentionDetailView(
            core: Core(),
            prayer: Prayer(
                id: 0,
                intention: "Mom",
                details: "Surgery recovery",
                tags: ["family", "health"],
                status: .active,
                cadence: .daily,
                saintId: "st-joseph",
                prayedOn: [
                    PrayerLogEntry(year: 2025, month: 12, day: 24, hour: 22, minute: 5),
                    PrayerLogEntry(year: 2026, month: 8, day: 10, hour: 7, minute: 40),
                    PrayerLogEntry(year: 2026, month: 8, day: 12, hour: 8, minute: 15),
                    PrayerLogEntry(year: 2026, month: 8, day: 12, hour: 21, minute: 30)
                ]
            )
        )
    }
}

#Preview("Empty log") {
    NavigationStack {
        IntentionDetailView(
            core: Core(),
            prayer: Prayer(
                id: 0,
                intention: "Mom",
                details: "Surgery recovery",
                tags: ["family", "health"],
                status: .active,
                cadence: .daily,
                saintId: nil,
                prayedOn: []
            )
        )
    }
}
