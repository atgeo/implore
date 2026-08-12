import App
import SwiftUI

struct AddIntentionView: View {
    @ObservedObject var core: Core
    @Environment(\.dismiss) private var dismiss

    private let prayer: Prayer?

    @State private var intention: String
    @State private var details: String
    @State private var tagsText: String
    @State private var cadence: IntentionCadence

    init(core: Core, prayer: Prayer? = nil) {
        self.core = core
        self.prayer = prayer
        _intention = State(initialValue: prayer?.intention ?? "")
        _details = State(initialValue: prayer?.details ?? "")
        _tagsText = State(initialValue: prayer?.tags.joined(separator: ", ") ?? "")
        _cadence = State(initialValue: prayer?.cadence ?? .unscheduled)
    }

    private var isEditing: Bool { prayer != nil }

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

            if prayer?.status != .archived {
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
        }
        .navigationTitle(isEditing ? "Edit Intention" : "Add Intention")
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

        if let prayer {
            core.update(
                .updatePrayer(
                    id: prayer.id,
                    intention: intention,
                    details: details,
                    tags: tags,
                    cadence: cadence
                )
            )
        } else {
            core.update(
                .addPrayer(
                    intention: intention,
                    details: details,
                    tags: tags,
                    cadence: cadence
                )
            )
        }
        dismiss()
    }
}

#Preview("Add") {
    NavigationStack {
        AddIntentionView(core: Core())
    }
}

#Preview("Edit") {
    NavigationStack {
        AddIntentionView(
            core: Core(),
            prayer: Prayer(
                id: 0,
                intention: "Mom",
                details: "Surgery recovery",
                tags: ["family", "sick"],
                status: .active,
                cadence: .daily
            )
        )
    }
}
