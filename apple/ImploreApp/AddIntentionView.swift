import App
import SwiftUI

struct AddIntentionView: View {
    @ObservedObject var core: Core
    @Environment(\.dismiss) private var dismiss

    @State private var intention = ""
    @State private var details = ""
    @State private var tagsText = ""
    @State private var cadence = IntentionCadence.unscheduled

    private var canSave: Bool {
        !intention.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        Form {
            Section {
                TextField("Someone you are carrying", text: $intention)
            } header: {
                Text("Intention")
            } footer: {
                Text("A person, family, or cause to pray for.")
            }

            Section {
                TextField("Details", text: $details, axis: .vertical)
                    .lineLimit(3...6)
            } header: {
                Text("Details")
            } footer: {
                Text("A private note for this prayer.")
            }

            Section {
                TextField("family, sick, holy souls", text: $tagsText)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
            } header: {
                Text("Tags")
            } footer: {
                Text("Optional. Separate with commas.")
            }

            Section {
                Picker("Schedule", selection: $cadence) {
                    Text("No schedule").tag(IntentionCadence.unscheduled)
                    Text("Daily").tag(IntentionCadence.daily)
                    Text("Weekly").tag(IntentionCadence.weekly)
                    Text("Monthly").tag(IntentionCadence.monthly)
                }
            } footer: {
                Text("How often you hope to pray this.")
            }
        }
        .navigationTitle("Add Intention")
        .toolbar {
            ToolbarItem(placement: .confirmationAction) {
                Button("Save", action: save)
                    .disabled(!canSave)
            }
        }
    }

    private func save() {
        let tags = tagsText
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }

        core.update(
            .addPrayer(
                intention: intention,
                details: details,
                tags: tags,
                cadence: cadence
            )
        )
        dismiss()
    }
}

#Preview {
    NavigationStack {
        AddIntentionView(core: Core())
    }
}
