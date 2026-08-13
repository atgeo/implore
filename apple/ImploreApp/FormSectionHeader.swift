import SwiftUI

/// Form section title with stronger light-mode contrast.
/// Dark keeps system secondary; light uses a darker gray so headers stay readable.
struct FormSectionHeader: View {
    @Environment(\.colorScheme) private var colorScheme

    private let title: LocalizedStringKey

    init(_ title: LocalizedStringKey) {
        self.title = title
    }

    var body: some View {
        Text(title)
            .foregroundStyle(colorScheme == .dark ? Color.secondary : Self.lightLabel)
    }

    /// ~#3A3A3C — darker than `secondaryLabel` (~60% black) for small caps headers.
    private static let lightLabel = Color(red: 0.23, green: 0.23, blue: 0.24)
}
