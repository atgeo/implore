import SwiftUI

struct SaintPickerView: View {
    @ObservedObject var catalog: ObservancesCatalog
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
                            .foregroundStyle(Color.brandAccent)
                    }
                }
            }
            .foregroundStyle(.primary)
            .paperCardRow()

            ForEach(filteredCompanions) { companion in
                Button {
                    selection = companion.id
                } label: {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(companion.name)
                            .font(.body)
                        if let date = companion.date, !date.isEmpty {
                            Text(feastLabel(date))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .overlay(alignment: .trailing) {
                        if selection == companion.id {
                            Image(systemName: "checkmark")
                                .foregroundStyle(Color.brandAccent)
                        }
                    }
                }
                .foregroundStyle(.primary)
                .paperCardRow()
            }
        }
        .paperBackground()
        .navigationTitle("Saint")
        .searchable(text: $search, prompt: "Search saints")
    }

    private var filteredCompanions: [Observance] {
        let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return catalog.companions }
        return catalog.companions.filter { companion in
            companion.name.localizedCaseInsensitiveContains(query)
                || (companion.patronage?.contains { $0.localizedCaseInsensitiveContains(query) } ?? false)
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
