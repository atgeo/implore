import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    var body: some View {
        TabView {
            TodayView(core: core)
                .tabItem {
                    Label("Today", systemImage: "sun.max")
                }

            IntentionsView(core: core)
                .tabItem {
                    Label("Intentions", systemImage: "hands.sparkles")
                }
        }
        .background(Color.paper)
    }
}

struct IntentionsView: View {
    @ObservedObject var core: Core
    @State private var path = NavigationPath()

    var body: some View {
        NavigationStack(path: $path) {
            IntentionsList(core: core, mode: .active, onAdd: { path.append(IntentionsRoute.add) })
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
                            path.append(IntentionsRoute.archived)
                        } label: {
                            Image(systemName: "archivebox")
                                .foregroundStyle(.primary)
                        }
                        .accessibilityLabel("Archived")
                    }

                    ToolbarItem(placement: .primaryAction) {
                        Button {
                            path.append(IntentionsRoute.add)
                        } label: {
                            Image(systemName: "plus")
                        }
                        .accessibilityLabel("Add Intention")
                    }
                }
                .navigationDestination(for: IntentionsRoute.self) { route in
                    switch route {
                    case .archived:
                        ArchivedIntentionsView(core: core)
                    case .add:
                        AddIntentionView(core: core)
                    }
                }
        }
    }
}

private enum IntentionsRoute: Hashable {
    case archived
    case add
}

/// Archived list pushed from the toolbar archive button.
private struct ArchivedIntentionsView: View {
    @ObservedObject var core: Core

    var body: some View {
        IntentionsList(core: core, mode: .archived)
            .navigationTitle("Archived")
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .paperBackground()
    }
}

private enum IntentionsListMode {
    case active
    case archived
}

private struct IntentionsList: View {
    @ObservedObject var core: Core
    let mode: IntentionsListMode
    var onAdd: (() -> Void)?

    private var prayers: [Prayer] {
        switch mode {
        case .active: core.view.prayers
        case .archived: core.view.archivedPrayers
        }
    }

    var body: some View {
        Group {
            if prayers.isEmpty {
                ContentUnavailableView {
                    Label(emptyTitle, systemImage: "heart")
                } description: {
                    Text(emptyDescription)
                } actions: {
                    if mode == .active, let onAdd {
                        Button("Add Intention", systemImage: "plus") {
                            onAdd()
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.large)
                    }
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List {
                    // One section per intention so each row is its own inset card;
                    // the top accent can then follow continuous corners correctly.
                    ForEach(prayers, id: \.id) { prayer in
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
                                .tint(.red)

                                if mode == .active {
                                    Button {
                                        core.update(.archivePrayer(id: prayer.id))
                                    } label: {
                                        Label("Archive", systemImage: "archivebox")
                                    }
                                    .tint(.gray)
                                } else {
                                    Button {
                                        core.update(.unarchivePrayer(id: prayer.id))
                                    } label: {
                                        Label("Unarchive", systemImage: "tray.and.arrow.up")
                                    }
                                    .tint(.brandAccent)
                                }
                            }
                        }
                    }
                }
                .listStyle(.insetGrouped)
                .listSectionSpacing(12)
            }
        }
        .paperBackground()
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

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(prayer.intention)
                .font(.body)

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
    }

    private var cadenceLabel: LocalizedStringKey? {
        guard prayer.status != .archived else { return nil }
        return prayer.cadence.listLabel
    }
}

#Preview {
    ContentView(core: Core())
}
