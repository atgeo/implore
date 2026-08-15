import App
import SwiftUI

struct CalendarView: View {
    @ObservedObject var core: Core
    @ObservedObject private var observancesCatalog = ObservancesCatalog.shared
    @Environment(\.locale) private var locale

    @State private var selectedDate = Date()

    private var calendarPrayers: [TodayPrayer] {
        core.view.calendarPrayers
    }

    private var dayObservances: [Observance] {
        guard let date = core.view.calendarDate else { return [] }
        return observancesCatalog.observances(
            onMonthDay: String(format: "%02d-%02d", Int(date.month), Int(date.day))
        )
    }

    private var showsDayHeading: Bool {
        core.view.calendarLiturgicalDay != nil || !dayObservances.isEmpty
    }

    private var isSelectedToday: Bool {
        guard let selected = core.view.calendarDate,
              let today = core.view.localDate
        else { return false }
        return selected == today
    }

    private var dateSelectionRange: ClosedRange<Date>? {
        guard let min = core.view.calendarMinDate.flatMap(LocalTimeSync.date(from:)),
              let max = core.view.calendarMaxDate.flatMap(LocalTimeSync.date(from:))
        else { return nil }
        return min...max
    }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    if let range = dateSelectionRange {
                        DatePicker(
                            "Date",
                            selection: $selectedDate,
                            in: range,
                            displayedComponents: .date
                        )
                        .datePickerStyle(.graphical)
                        .environment(\.calendar, LocalTimeSync.civilCalendar)
                        .labelsHidden()
                        .listRowInsets(EdgeInsets(top: 8, leading: 8, bottom: 8, trailing: 8))
                        .paperCardRow()
                        .onChange(of: selectedDate) { _, newValue in
                            select(newValue)
                        }
                    }
                }

                Section {
                    if showsDayHeading {
                        dayHeading
                            .listRowBackground(Color.clear)
                            .listRowSeparator(.hidden)
                            .listRowInsets(EdgeInsets(top: 4, leading: 16, bottom: 4, trailing: 16))
                    }

                    if calendarPrayers.isEmpty {
                        ContentUnavailableView {
                            Label(emptyTitle, systemImage: "calendar")
                        } description: {
                            Text(emptyDescription)
                        }
                        .frame(maxWidth: .infinity)
                        .listRowBackground(Color.clear)
                        .listRowSeparator(.hidden)
                    } else {
                        ForEach(calendarPrayers, id: \.prayer.id) { item in
                            HStack(spacing: 12) {
                                if isSelectedToday {
                                    PrayerToggle(prayed: item.prayedToday) {
                                        logPrayer(item.prayer.id)
                                    }
                                }

                                NavigationLink {
                                    IntentionDetailView(core: core, prayer: item.prayer)
                                } label: {
                                    TodayIntentionRow(
                                        prayer: item.prayer,
                                        prayedToday: item.prayedToday,
                                        localDate: core.view.calendarDate
                                    )
                                }
                            }
                            .listRowBackground(IntentionRowBackground(color: item.prayer.color))
                            .listRowSeparator(.hidden)
                        }
                    }
                }
            }
            .listStyle(.insetGrouped)
            .listSectionSpacing(12)
            .paperBackground()
            .navigationTitle("Calendar")
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    NavigationLink {
                        SettingsView(core: core)
                    } label: {
                        Image(systemName: "gearshape")
                    }
                    .accessibilityLabel("Settings")
                }

                ToolbarItem(placement: .primaryAction) {
                    if !isSelectedToday {
                        Button("Today") {
                            jumpToToday()
                        }
                    }
                }
            }
            .onAppear {
                syncSelectionFromCore()
            }
            .onChange(of: core.view.calendarDate) { _, _ in
                syncSelectionFromCore()
            }
            .onChange(of: core.view.localDate) { _, _ in
                // Range may shift at midnight; keep picker in bounds.
                if let date = core.view.calendarDate.flatMap(LocalTimeSync.date(from:)) {
                    selectedDate = date
                }
            }
        }
    }

    @ViewBuilder
    private var dayHeading: some View {
        if showsDayHeading {
            VStack(alignment: .leading, spacing: 4) {
                if let day = core.view.calendarLiturgicalDay {
                    Text(day.title(locale: locale))
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                ForEach(dayObservances) { observance in
                    Text(observance.name)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.primary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 4)
            .padding(.bottom, 4)
            .accessibilityElement(children: .combine)
            .accessibilityAddTraits(.isHeader)
        }
    }

    private var emptyTitle: LocalizedStringKey {
        "Nothing due"
    }

    private var emptyDescription: LocalizedStringKey {
        "Nothing is due on this day."
    }

    private func select(_ date: Date) {
        guard let civil = LocalTimeSync.civilDate(from: date) else { return }
        guard (0...Int(UInt16.max)).contains(Int(civil.year)),
              (1...12).contains(Int(civil.month)),
              (1...31).contains(Int(civil.day))
        else { return }
        core.update(
            .selectCalendarDate(
                year: UInt16(civil.year),
                month: UInt8(civil.month),
                day: UInt8(civil.day)
            )
        )
    }

    private func jumpToToday() {
        guard let today = core.view.localDate.flatMap(LocalTimeSync.date(from:)) else { return }
        selectedDate = today
        select(today)
    }

    private func syncSelectionFromCore() {
        guard let date = core.view.calendarDate.flatMap(LocalTimeSync.date(from:)) else { return }
        if !LocalTimeSync.civilCalendar.isDate(selectedDate, inSameDayAs: date) {
            selectedDate = date
        }
    }

    private func logPrayer(_ id: UInt64) {
        LocalTimeSync.sync(to: core)
        guard isSelectedToday,
              let item = calendarPrayers.first(where: { $0.prayer.id == id }),
              !item.prayedToday
        else { return }
        core.update(.logPrayer(id: id))
    }
}

#Preview {
    CalendarView(core: Core())
}
