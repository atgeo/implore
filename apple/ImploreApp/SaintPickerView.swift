import SwiftUI

struct SaintPickerView: View {
    @ObservedObject var catalog: SaintsCatalog
    @Binding var selection: String?
    @State private var search = ""

    var body: some View {
        List {
            Button {
                selection = nil
            } label: {
                HStack {
                    Text("None")
                    Spacer()
                    if selection == nil {
                        Image(systemName: "checkmark")
                            .foregroundStyle(Color.accentColor)
                    }
                }
            }
            .foregroundStyle(.primary)

            ForEach(filteredSaints) { saint in
                Button {
                    selection = saint.id
                } label: {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(saint.name)
                            .font(.body)
                        if let feast = saint.feast, !feast.isEmpty {
                            Text(feastLabel(feast))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .overlay(alignment: .trailing) {
                        if selection == saint.id {
                            Image(systemName: "checkmark")
                                .foregroundStyle(Color.accentColor)
                        }
                    }
                }
                .foregroundStyle(.primary)
            }
        }
        .navigationTitle("Saint")
        .searchable(text: $search, prompt: "Search saints")
    }

    private var filteredSaints: [Saint] {
        let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return catalog.saints }
        return catalog.saints.filter { saint in
            saint.name.localizedCaseInsensitiveContains(query)
                || (saint.patronage?.contains { $0.localizedCaseInsensitiveContains(query) } ?? false)
        }
    }

    private func feastLabel(_ feast: String) -> String {
        let parts = feast.split(separator: "-")
        guard parts.count == 2,
              let month = Int(parts[0]),
              let day = Int(parts[1])
        else { return feast }
        var components = DateComponents()
        components.month = month
        components.day = day
        let formatter = DateFormatter()
        formatter.locale = Locale.current
        formatter.setLocalizedDateFormatFromTemplate("MMMMd")
        if let date = Calendar.current.date(from: components) {
            return formatter.string(from: date)
        }
        return feast
    }
}
