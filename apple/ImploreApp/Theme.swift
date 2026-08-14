import SwiftUI

extension Color {
    /// Brand chrome (tabs, Save, prayed check). Named asset, not system blue.
    static let brandAccent = Color("AccentColor")

    /// Page fill. Light is parchment; dark is a warm near-black.
    static let paper = Color("Paper")

    /// Grouped row / card fill. Light matches the app icon cream.
    static let paperCard = Color("PaperCard")
}

extension View {
    /// Warm paper behind a `List` or `Form`, hiding the system grouped gray.
    func paperBackground() -> some View {
        scrollContentBackground(.hidden)
            .background(Color.paper)
    }

    /// Warm card fill for a grouped list or form row.
    func paperCardRow() -> some View {
        listRowBackground(Color.paperCard)
    }
}
