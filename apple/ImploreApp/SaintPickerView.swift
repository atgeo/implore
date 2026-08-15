import SwiftUI

private enum SaintSort: String, CaseIterable, Identifiable {
    case name
    case date

    var id: String { rawValue }

    var title: LocalizedStringKey {
        switch self {
        case .name: "Name"
        case .date: "Date"
        }
    }
}

struct SaintPickerView: View {
    @ObservedObject var catalog: ObservancesCatalog
    @Binding var selection: String?
    @AppStorage("saintSort") private var sort = SaintSort.name
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
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Menu {
                    Picker("Sort", selection: $sort) {
                        ForEach(SaintSort.allCases) { option in
                            Text(option.title).tag(option)
                        }
                    }
                } label: {
                    Image(systemName: "arrow.up.arrow.down")
                }
                .accessibilityLabel("Sort")
            }
        }
    }

    private var filteredCompanions: [Observance] {
        let query = search.trimmingCharacters(in: .whitespacesAndNewlines)
        let matches = query.isEmpty
            ? catalog.companions
            : catalog.companions.filter { companion in
                companion.name.localizedCaseInsensitiveContains(query)
                    || (companion.patronage?.contains { $0.localizedCaseInsensitiveContains(query) } ?? false)
            }
        return sorted(matches)
    }

    private func sorted(_ companions: [Observance]) -> [Observance] {
        switch sort {
        case .name:
            companions
        case .date:
            companions.sorted { lhs, rhs in
                switch (monthDay(lhs.date), monthDay(rhs.date)) {
                case let (left?, right?) where left != right:
                    left < right
                case (_?, nil):
                    true
                case (nil, _?):
                    false
                default:
                    lhs.name.localizedCaseInsensitiveCompare(rhs.name) == .orderedAscending
                }
            }
        }
    }

    private func monthDay(_ date: String?) -> (Int, Int)? {
        guard let date, !date.isEmpty else { return nil }
        let parts = date.split(separator: "-")
        guard parts.count == 2,
              let month = Int(parts[0]),
              let day = Int(parts[1])
        else { return nil }
        return (month, day)
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
        if let date = LocalTimeSync.civilCalendar.date(from: components) {
            return formatter.string(from: date)
        }
        return feast
    }
}
