import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core
    @State private var showArchived = false

    var body: some View {
        NavigationStack {
            IntentionsList(core: core, mode: .active)
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

                    ToolbarItem(placement: .topBarLeading) {
                        Button {
                            showArchived = true
                        } label: {
                            Image(systemName: "archivebox")
                        }
                        .accessibilityLabel("Archived")
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
                .navigationDestination(isPresented: $showArchived) {
                    ArchivedIntentionsView(core: core)
                }
                .onChange(of: showArchived) { _, presented in
                    if !presented {
                        core.update(.setFilter(filter: .active))
                    }
                }
                .onAppear {
                    if !showArchived, core.view.filter != .active {
                        core.update(.setFilter(filter: .active))
                    }
                }
        }
    }
}

/// Archived list pushed from the toolbar archive button.
private struct ArchivedIntentionsView: View {
    @ObservedObject var core: Core

    var body: some View {
        IntentionsList(core: core, mode: .archived)
            .navigationTitle("Archived")
            .onAppear {
                if core.view.filter != .archived {
                    core.update(.setFilter(filter: .archived))
                }
            }
    }
}

private enum IntentionsListMode {
    case active
    case archived
}

private struct IntentionsList: View {
    @ObservedObject var core: Core
    let mode: IntentionsListMode

    var body: some View {
        List {
            if core.view.prayers.isEmpty {
                Section {
                    ContentUnavailableView(
                        emptyTitle,
                        systemImage: "heart",
                        description: Text(emptyDescription)
                    )
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                }
            } else {
                // One section per intention so each row is its own inset card;
                // the top accent can then follow continuous corners correctly.
                ForEach(core.view.prayers, id: \.id) { prayer in
                    Section {
                        NavigationLink {
                            IntentionDetailView(core: core, prayer: prayer)
                        } label: {
                            IntentionRow(prayer: prayer)
                        }
                        .listRowBackground(IntentionRowBackground(color: prayer.color))
                        .listRowSeparator(.hidden)
                        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                            Button(role: .destructive) {
                                core.update(.removePrayer(id: prayer.id))
                            } label: {
                                Label("Delete", systemImage: "trash")
                            }

                            if mode == .active {
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
        }
        .listStyle(.insetGrouped)
        .listSectionSpacing(12)
    }

    private var emptyTitle: LocalizedStringKey {
        mode == .archived ? "Nothing set aside" : "No intentions yet"
    }

    private var emptyDescription: LocalizedStringKey {
        mode == .archived
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
