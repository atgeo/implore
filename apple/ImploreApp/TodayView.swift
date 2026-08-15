import App
import SwiftUI

struct TodayView: View {
    @ObservedObject var core: Core
    @ObservedObject private var observancesCatalog = ObservancesCatalog.shared
    @Environment(\.locale) private var locale

    private var todayPrayers: [TodayPrayer] {
        core.view.todayPrayers
    }

    private var todayObservances: [Observance] {
        guard let date = core.view.localDate else { return [] }
        return observancesCatalog.observances(
            onMonthDay: String(format: "%02d-%02d", Int(date.month), Int(date.day))
        )
    }

    private var showsDayHeading: Bool {
        core.view.liturgicalDay != nil || !todayObservances.isEmpty
    }

    var body: some View {
        NavigationStack {
            Group {
                if todayPrayers.isEmpty {
                    VStack(alignment: .leading, spacing: 0) {
                        dayHeading(horizontalPadding: 16)
                        ContentUnavailableView {
                            Label(emptyTitle, systemImage: "sun.max")
                        } description: {
                            Text(emptyDescription)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                } else {
                    List {
                        ForEach(Array(todayPrayers.enumerated()), id: \.element.prayer.id) { index, item in
                            Section {
                                HStack(spacing: 12) {
                                    PrayerToggle(
                                        prayed: item.prayedToday
                                    ) {
                                        logPrayer(item.prayer.id)
                                    }

                                    NavigationLink {
                                        IntentionDetailView(core: core, prayer: item.prayer)
                                    } label: {
                                        TodayIntentionRow(
                                            prayer: item.prayer,
                                            prayedToday: item.prayedToday,
                                            localDate: core.view.localDate
                                        )
                                    }
                                }
                                .listRowBackground(IntentionRowBackground(color: item.prayer.color))
                                .listRowSeparator(.hidden)
                            } header: {
                                if index == 0 {
                                    dayHeading(horizontalPadding: 0)
                                        .textCase(nil)
                                }
                            }
                        }
                    }
                    .listStyle(.insetGrouped)
                    .listSectionSpacing(12)
                }
            }
            .paperBackground()
            .navigationTitle("Today")
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    NavigationLink {
                        SettingsView(core: core)
                    } label: {
                        Image(systemName: "gearshape")
                    }
                    .accessibilityLabel("Settings")
                }
            }
        }
    }

    @ViewBuilder
    private func dayHeading(horizontalPadding: CGFloat) -> some View {
        if showsDayHeading {
            VStack(alignment: .leading, spacing: 4) {
                if let day = core.view.liturgicalDay {
                    Text(day.title(locale: locale))
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                ForEach(todayObservances) { observance in
                    Text(observance.name)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(.primary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, horizontalPadding)
            .padding(.top, 4)
            .padding(.bottom, 4)
            .accessibilityElement(children: .combine)
            .accessibilityAddTraits(.isHeader)
        }
    }

    private var emptyTitle: LocalizedStringKey {
        core.view.hasPrayers ? "Nothing for today" : "No intentions yet"
    }

    private var emptyDescription: LocalizedStringKey {
        core.view.hasPrayers
            ? "Nothing is due today. Set a schedule on an intention to see it here."
            : "Add someone you are carrying in prayer from the Intentions tab."
    }

    private func logPrayer(_ id: UInt64) {
        LocalTimeSync.sync(to: core)
        guard let item = todayPrayers.first(where: { $0.prayer.id == id }),
              !item.prayedToday
        else { return }
        core.update(.logPrayer(id: id))
    }
}

/// Leading control: empty circle → check. Tap only logs; does not un-log.
private struct PrayerToggle: View {
    let prayed: Bool
    let action: () -> Void

    @State private var bounce = 0

    var body: some View {
        Button {
            guard !prayed else { return }
            bounce += 1
            action()
        } label: {
            Image(systemName: prayed ? "checkmark.circle.fill" : "circle")
                .font(.title2)
                .symbolRenderingMode(.hierarchical)
                .foregroundStyle(prayed ? Color.brandAccent : Color.secondary)
                .symbolEffect(.bounce, value: bounce)
                .frame(width: 32, height: 32)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(prayed)
        .accessibilityLabel(prayed ? "Prayed today" : "Mark as prayed")
        .sensoryFeedback(.success, trigger: bounce)
    }
}

private struct TodayIntentionRow: View {
    let prayer: Prayer
    let prayedToday: Bool
    let localDate: CivilDate?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(prayer.intention)
                .font(.body)
                .strikethrough(prayedToday, color: .secondary)
                .foregroundStyle(prayedToday ? .secondary : .primary)

            if !prayer.tags.isEmpty {
                Text(prayer.tags.joined(separator: " · "))
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }

            if let cadence = cadenceLabel {
                Text(cadence)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .padding(.vertical, 2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .opacity(prayedToday ? 0.7 : 1)
    }

    private var cadenceLabel: LocalizedStringKey? {
        switch prayer.cadence {
        case .unscheduled: nil
        case .daily: "Daily"
        case .weekly: "Weekly"
        case .monthly: "Monthly"
        case .novena:
            NovenaScheduleLabel.detailLabel(
                start: prayer.novenaStart,
                localDate: localDate
            )
        }
    }
}

#Preview {
    TodayView(core: Core())
}
