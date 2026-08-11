import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    var body: some View {
        NavigationStack {
            List {
                ForEach(core.view.prayers, id: \.id) { prayer in
                    IntentionRow(prayer: prayer)
                }
                .onDelete(perform: removePrayers)
            }
            .navigationTitle("Intentions")
            .toolbar {
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

    private func removePrayers(at offsets: IndexSet) {
        let ids = offsets.map { core.view.prayers[$0].id }
        for id in ids {
            core.update(.removePrayer(id: id))
        }
    }
}

struct IntentionRow: View {
    let prayer: Prayer

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(prayer.intention)
                .font(.body)

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
    }
}

#Preview {
    ContentView(core: Core())
}
