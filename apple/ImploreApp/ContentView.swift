import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core
    @State private var newPrayerText = ""

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                List {
                    ForEach(core.view.prayers, id: \.id) { prayer in
                        Text(prayer.text)
                    }
                    .onDelete(perform: removePrayers)
                }

                HStack {
                    TextField("Prayer intention", text: $newPrayerText)
                        .textFieldStyle(.roundedBorder)
                        .onSubmit(addPrayer)

                    Button("Add", action: addPrayer)
                        .disabled(newPrayerText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
                .padding()
            }
            .navigationTitle("Intentions")
        }
    }

    private func addPrayer() {
        let text = newPrayerText
        newPrayerText = ""
        core.update(.addPrayer(text: text))
    }

    private func removePrayers(at offsets: IndexSet) {
        let ids = offsets.map { core.view.prayers[$0].id }
        for id in ids {
            core.update(.removePrayer(id: id))
        }
    }
}

#Preview {
    ContentView(core: Core())
}
