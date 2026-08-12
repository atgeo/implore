import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    var body: some View {
        NavigationStack {
            List {
                if core.view.prayers.isEmpty {
                    ContentUnavailableView(
                        emptyTitle,
                        systemImage: "heart",
                        description: Text(emptyDescription)
                    )
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                } else {
                    ForEach(core.view.prayers, id: \.id) { prayer in
                        NavigationLink {
                            AddIntentionView(core: core, prayer: prayer)
                        } label: {
                            IntentionRow(prayer: prayer, showStatus: core.view.filter == .all)
                        }
                        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                                Button(role: .destructive) {
                                    core.update(.removePrayer(id: prayer.id))
                                } label: {
                                    Label("Delete", systemImage: "trash")
                                }

                                if prayer.status == .active {
                                    Button {
                                        core.update(.archivePrayer(id: prayer.id))
                                    } label: {
                                        Label("Archive", systemImage: "archivebox")
                                    }
                                    .tint(.orange)
                                } else {
                                    Button {
                                        core.update(.unarchivePrayer(id: prayer.id))
                                    } label: {
                                        Label("Unarchive", systemImage: "tray.and.arrow.up")
                                    }
                                    .tint(.blue)
                                }
                            }
                    }
                }
            }
            .navigationTitle("Intentions")
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    NavigationLink {
                        SettingsView(core: core)
                    } label: {
                        Image(systemName: "gearshape")
                    }
                    .accessibilityLabel("Settings")
                }

                ToolbarItem(placement: .principal) {
                    Picker("Filter", selection: filterBinding) {
                        Text("Active").tag(IntentionFilter.active)
                        Text("Archived").tag(IntentionFilter.archived)
                        Text("All").tag(IntentionFilter.all)
                    }
                    .pickerStyle(.segmented)
                    .frame(maxWidth: 280)
                }

                ToolbarItem(placement: .primaryAction) {
                    NavigationLink {
                        AddIntentionView(core: core)
                    } label: {
                        Image(systemName: "plus")
                    }
                    .accessibilityLabel("Add Intention")
                }
            }
        }
    }

    private var filterBinding: Binding<IntentionFilter> {
        Binding(
            get: { core.view.filter },
            set: { core.update(.setFilter(filter: $0)) }
        )
    }

    private var emptyTitle: LocalizedStringKey {
        core.view.filter == .archived ? "Nothing set aside" : "No intentions yet"
    }

    private var emptyDescription: LocalizedStringKey {
        core.view.filter == .archived
            ? "Intentions you set aside will appear here."
            : "Add someone you are carrying in prayer."
    }
}

struct IntentionRow: View {
    let prayer: Prayer
    var showStatus = false

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(prayer.intention)
                    .font(.body)

                if showStatus, prayer.status == .archived {
                    Spacer(minLength: 8)
                    Text("Archived")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                }
            }

            if let details = prayer.details, !details.isEmpty {
                Text(details)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

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
        .opacity(prayer.status == .archived ? 0.7 : 1)
    }

    private var cadenceLabel: LocalizedStringKey? {
        guard prayer.status != .archived else { return nil }
        switch prayer.cadence {
        case .unscheduled: return nil
        case .daily: return "Daily"
        case .weekly: return "Weekly"
        case .monthly: return "Monthly"
        }
    }
}

#Preview {
    ContentView(core: Core())
}
