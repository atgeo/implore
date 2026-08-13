import App
import SwiftUI

struct AddIntentionView: View {
    @ObservedObject var core: Core
    @ObservedObject private var saintsCatalog = SaintsCatalog.shared
    @Environment(\.dismiss) private var dismiss

    private let prayer: Prayer?

    @State private var intention: String
    @State private var details: String
    @State private var tags: [String]
    @State private var tagDraft: String = ""
    @State private var cadence: IntentionCadence
    @State private var saintId: String?
    @State private var color: IntentionColor

    init(core: Core, prayer: Prayer? = nil) {
        self.core = core
        self.prayer = prayer
        _intention = State(initialValue: prayer?.intention ?? "")
        _details = State(initialValue: prayer?.details ?? "")
        _tags = State(initialValue: prayer?.tags ?? [])
        _cadence = State(initialValue: prayer?.cadence ?? .unscheduled)
        _saintId = State(initialValue: prayer?.saintId)
        _color = State(initialValue: prayer?.color ?? .none)
    }

    private var isEditing: Bool { prayer != nil }

    private var canSave: Bool {
        !intention.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var selectedSaintLabel: String {
        if let saintId, let saint = saintsCatalog.saint(for: saintId) {
            return saint.name
        }
        return String(localized: "None")
    }

    var body: some View {
        Form {
            Section {
                TextField("Someone you are carrying", text: $intention)
            } header: {
                FormSectionHeader("Intention")
            } footer: {
                Text("A person, family, or cause to pray for.")
            }

            Section {
                TextField("Details", text: $details, axis: .vertical)
                    .lineLimit(3...6)
            } header: {
                FormSectionHeader("Details")
            } footer: {
                Text("A private note for this prayer.")
            }

            Section {
                TagEditor(tags: $tags, draft: $tagDraft)
            } header: {
                FormSectionHeader("Tags")
            } footer: {
                Text("Optional. Type a tag and press return.")
            }

            Section {
                NavigationLink {
                    SaintPickerView(
                        catalog: saintsCatalog,
                        selection: $saintId
                    )
                } label: {
                    HStack {
                        Text("Saint")
                        Spacer()
                        Text(selectedSaintLabel)
                            .foregroundStyle(.secondary)
                    }
                }
            } footer: {
                Text("Optional. Ask this saint to pray with you.")
            }

            Section {
                IntentionColorPicker(selection: $color)
            } header: {
                FormSectionHeader("Color")
            } footer: {
                Text("Optional. A quiet accent on the list.")
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
        commitDraft()

        if let prayer {
            core.update(
                .updatePrayer(
                    id: prayer.id,
                    intention: intention,
                    details: details,
                    tags: tags,
                    cadence: cadence,
                    saintId: saintId ?? "",
                    color: color
                )
            )
        } else {
            core.update(
                .addPrayer(
                    intention: intention,
                    details: details,
                    tags: tags,
                    cadence: cadence,
                    saintId: saintId ?? "",
                    color: color
                )
            )
        }
        dismiss()
    }

    private func commitDraft() {
        let trimmed = tagDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        tagDraft = ""
        guard !trimmed.isEmpty, !tags.contains(trimmed) else { return }
        tags.append(trimmed)
    }
}

private struct TagEditor: View {
    @Binding var tags: [String]
    @Binding var draft: String

    var body: some View {
        FlowLayout(spacing: 8) {
            ForEach(tags, id: \.self) { tag in
                TagChip(title: tag) {
                    tags.removeAll { $0 == tag }
                }
            }

            TextField("Add a tag", text: $draft)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .submitLabel(.done)
                .onSubmit(commitDraft)
                .onChange(of: draft) { _, newValue in
                    commitCommas(in: newValue)
                }
                .onKeyPress(.delete, phases: .down) { _ in
                    guard draft.isEmpty, !tags.isEmpty else {
                        return .ignored
                    }
                    tags.removeLast()
                    return .handled
                }
                .frame(minWidth: 120)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func commitDraft() {
        let trimmed = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        draft = ""
        guard !trimmed.isEmpty, !tags.contains(trimmed) else { return }
        tags.append(trimmed)
    }

    private func commitCommas(in value: String) {
        guard value.contains(",") else { return }
        let parts = value.split(separator: ",", omittingEmptySubsequences: false)
        guard let last = parts.last else { return }
        for part in parts.dropLast() {
            let trimmed = part.trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty, !tags.contains(trimmed) {
                tags.append(trimmed)
            }
        }
        draft = String(last)
    }
}

private struct TagChip: View {
    let title: String
    let onRemove: () -> Void

    var body: some View {
        HStack(spacing: 4) {
            Text(title)
                .font(.subheadline)

            Button(action: onRemove) {
                Image(systemName: "xmark.circle.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Remove \(title)")
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(.fill.tertiary, in: Capsule())
    }
}

private struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let frames = frames(for: subviews, in: proposal.width)
        let width = proposal.width
            ?? frames.map(\.maxX).max()
            ?? 0
        let height = frames.map(\.maxY).max() ?? 0
        return CGSize(width: width, height: height)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let frames = frames(for: subviews, in: bounds.width)
        for (subview, frame) in zip(subviews, frames) {
            subview.place(
                at: CGPoint(x: bounds.minX + frame.minX, y: bounds.minY + frame.minY),
                proposal: ProposedViewSize(frame.size)
            )
        }
    }

    private func frames(for subviews: Subviews, in width: CGFloat?) -> [CGRect] {
        let maxWidth = width ?? .greatestFiniteMagnitude
        var frames: [CGRect] = []
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowHeight: CGFloat = 0
        var rowStart = 0

        for (index, subview) in subviews.enumerated() {
            let isLast = index == subviews.count - 1
            let ideal = subview.sizeThatFits(.unspecified)
            let remaining = max(maxWidth - x, 0)
            let size: CGSize
            if isLast, maxWidth.isFinite {
                let fieldWidth = x == 0 ? maxWidth : max(ideal.width, remaining)
                if x > 0, fieldWidth > remaining {
                    centerRow(&frames, rowStart: rowStart, rowHeight: rowHeight)
                    x = 0
                    y += rowHeight + spacing
                    rowHeight = 0
                    rowStart = index
                    size = CGSize(
                        width: maxWidth,
                        height: subview.sizeThatFits(ProposedViewSize(width: maxWidth, height: nil)).height
                    )
                } else {
                    size = CGSize(
                        width: min(fieldWidth, maxWidth),
                        height: subview.sizeThatFits(
                            ProposedViewSize(width: min(fieldWidth, maxWidth), height: nil)
                        ).height
                    )
                }
            } else {
                size = CGSize(width: min(ideal.width, maxWidth), height: ideal.height)
                if x > 0, x + size.width > maxWidth {
                    centerRow(&frames, rowStart: rowStart, rowHeight: rowHeight)
                    x = 0
                    y += rowHeight + spacing
                    rowHeight = 0
                    rowStart = index
                }
            }

            frames.append(CGRect(origin: CGPoint(x: x, y: y), size: size))
            rowHeight = max(rowHeight, size.height)
            x += size.width + spacing
        }

        centerRow(&frames, rowStart: rowStart, rowHeight: rowHeight)
        return frames
    }

    private func centerRow(_ frames: inout [CGRect], rowStart: Int, rowHeight: CGFloat) {
        guard rowStart < frames.count else { return }
        for index in rowStart..<frames.count {
            let frame = frames[index]
            frames[index].origin.y = frame.minY + (rowHeight - frame.height) / 2
        }
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
                cadence: .daily,
                saintId: "st-joseph",
                color: .sage,
                prayedOn: []
            )
        )
    }
}
