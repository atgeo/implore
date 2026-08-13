import App
import SwiftUI

struct TodayView: View {
    @ObservedObject var core: Core
    @Environment(\.locale) private var locale

    private var todayPrayers: [Prayer] {
        TodaySelection.prayers(from: core.view.reminderPrayers)
    }

    var body: some View {
        NavigationStack {
            Group {
                if todayPrayers.isEmpty {
                    VStack(alignment: .leading, spacing: 0) {
                        liturgicalHeading
                        ContentUnavailableView {
                            Label(emptyTitle, systemImage: "sun.max")
                        } description: {
                            Text(emptyDescription)
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                } else {
                    List {
                        if core.view.liturgicalDay != nil {
                            liturgicalHeading
                                // Match the large navigation title’s leading edge
                                // (insetGrouped already supplies the outer margin).
                                .listRowInsets(EdgeInsets(top: 2, leading: 0, bottom: 6, trailing: 0))
                                .listRowBackground(Color.clear)
                                .listRowSeparator(.hidden)
                        }

                        ForEach(todayPrayers, id: \.id) { prayer in
                            Section {
                                HStack(spacing: 12) {
                                    PrayerToggle(
                                        prayed: TodaySelection.prayedToday(prayer)
                                    ) {
                                        logPrayer(prayer.id)
                                    }

                                    NavigationLink {
                                        IntentionDetailView(core: core, prayer: prayer)
                                    } label: {
                                        TodayIntentionRow(
                                            prayer: prayer,
                                            prayedToday: TodaySelection.prayedToday(prayer)
                                        )
                                    }
                                }
                                .listRowBackground(IntentionRowBackground(color: prayer.color))
                                .listRowSeparator(.hidden)
                            }
                        }
                    }
                    .listStyle(.insetGrouped)
                    .listSectionSpacing(12)
                }
            }
            .background(Color(.systemGroupedBackground))
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
    private var liturgicalHeading: some View {
        if let day = core.view.liturgicalDay {
            Text(day.title(locale: locale))
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, todayPrayers.isEmpty ? 16 : 0)
                .padding(.top, todayPrayers.isEmpty ? 4 : 0)
                .padding(.bottom, todayPrayers.isEmpty ? 8 : 0)
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
        guard let prayer = todayPrayers.first(where: { $0.id == id }),
              !TodaySelection.prayedToday(prayer)
        else { return }
        LocalTimeSync.sync(to: core)
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
                .foregroundStyle(prayed ? Color.accentColor : Color.secondary)
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
        }
    }
}

#Preview {
    TodayView(core: Core())
}
