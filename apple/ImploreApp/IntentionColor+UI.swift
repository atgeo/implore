import App
import SwiftUI

extension IntentionColor {
    /// Platform color for the accent strip. `nil` means no border.
    var accent: Color? {
        switch self {
        case .none: nil
        case .sky: Color(red: 0.45, green: 0.62, blue: 0.78)
        case .sage: Color(red: 0.52, green: 0.64, blue: 0.52)
        case .sand: Color(red: 0.78, green: 0.68, blue: 0.50)
        case .rose: Color(red: 0.78, green: 0.55, blue: 0.58)
        case .slate: Color(red: 0.48, green: 0.52, blue: 0.58)
        case .gold: Color(red: 0.78, green: 0.64, blue: 0.32)
        }
    }

    static let presets: [IntentionColor] = [
        .none, .sky, .sage, .sand, .rose, .slate, .gold,
    ]
}

struct IntentionColorPicker: View {
    @Binding var selection: IntentionColor

    var body: some View {
        HStack(spacing: 12) {
            ForEach(IntentionColor.presets, id: \.self) { color in
                Button {
                    selection = color
                } label: {
                    ZStack {
                        Circle()
                            .fill(color.accent ?? Color(.tertiarySystemFill))
                            .frame(width: 28, height: 28)
                        if color == .none {
                            Image(systemName: "circle.slash")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        if selection == color {
                            Circle()
                                .strokeBorder(.primary, lineWidth: 2)
                                .frame(width: 34, height: 34)
                        }
                    }
                    .frame(width: 34, height: 34)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(Text(accessibilityName(color)))
                .accessibilityAddTraits(selection == color ? .isSelected : [])
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 4)
    }

    private func accessibilityName(_ color: IntentionColor) -> LocalizedStringKey {
        switch color {
        case .none: "None"
        case .sky: "Sky"
        case .sage: "Sage"
        case .sand: "Sand"
        case .rose: "Rose"
        case .slate: "Slate"
        case .gold: "Gold"
        }
    }
}

/// Inset-grouped row chrome. Keep one intention per `Section` so UIKit clips
/// this fill to a single card; the accent then sits flush on the top edge.
struct IntentionRowBackground: View {
    let color: IntentionColor

    private let accentHeight: CGFloat = 4

    var body: some View {
        ZStack(alignment: .top) {
            Color.paperCard

            if let accent = color.accent {
                accent
                    .frame(height: accentHeight)
                    .frame(maxWidth: .infinity)
                    .accessibilityHidden(true)
            }
        }
    }
}
