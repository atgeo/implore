import App
import SwiftUI

struct AddIntentionView: View {
    @ObservedObject var core: Core
    @Environment(\.dismiss) private var dismiss

    @State private var intention = ""
    @State private var details = ""
    @State private var tagsText = ""

    private var canSave: Bool {
        !intention.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        Form {
            Section {
                TextField("Intention", text: $intention)
            }

            Section {
                TextField("Details", text: $details, axis: .vertical)
                    .lineLimit(3...6)
            } header: {
                Text("Details")
            } footer: {
                Text("Optional note about this intention.")
            }

            Section {
                TextField("family, health", text: $tagsText)
                    .autocorrectionDisabled()
            } header: {
                Text("Tags")
            } footer: {
                Text("Optional. Separate tags with commas.")
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
                tags: tags
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
