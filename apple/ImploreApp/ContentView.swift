import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    var body: some View {
        NavigationStack {
            List {
                ForEach(core.view.prayers, id: \.id) { prayer in
                    IntentionRow(prayer: prayer, showStatus: core.view.filter == .all)
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
            .navigationTitle("Intentions")
            .toolbar {
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
        }
        .padding(.vertical, 2)
        .opacity(prayer.status == .archived ? 0.7 : 1)
    }
}

#Preview {
    ContentView(core: Core())
}
