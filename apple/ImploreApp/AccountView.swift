import App
import SwiftUI

/// Account and optional private sync — Settings drill-down.
struct AccountView: View {
    @ObservedObject var core: Core
    @State private var email = ""
    @State private var password = ""
    @FocusState private var focusedField: Field?

    private enum Field {
        case email
        case password
    }

    var body: some View {
        Form {
            if isSignedIn {
                signedInContent
            } else {
                signedOutContent
            }
        }
        .paperBackground()
        .scrollDismissesKeyboard(.interactively)
        .navigationTitle("Account")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            if isBusy {
                ToolbarItem(placement: .topBarTrailing) {
                    ProgressView()
                }
            }
        }
        .onChange(of: core.view.accountStatus) { _, status in
            if case .signedIn = status {
                focusedField = nil
                password = ""
            }
        }
    }

    // MARK: - Signed in

    @ViewBuilder
    private var signedInContent: some View {
        Section {
            LabeledContent {
                Text(core.view.signedInEmail)
                    .foregroundStyle(.primary)
                    .multilineTextAlignment(.trailing)
                    .textSelection(.enabled)
            } label: {
                Text("Email")
            }
            .paperCardRow()
        } footer: {
            Text("Signed in on this device.")
        }

        Section {
            LabeledContent {
                if let lastSyncedLabel {
                    Text(lastSyncedLabel)
                } else {
                    Text("Never")
                }
            } label: {
                Text("Last Synced")
            }
            .paperCardRow()

            Button {
                LocalTimeSync.sync(to: core)
                core.update(.syncRequested)
            } label: {
                HStack {
                    Text("Sync Now")
                    Spacer()
                    ProgressView()
                        .opacity(isSyncing ? 1 : 0)
                }
            }
            .disabled(isBusy)
            .paperCardRow()
        } header: {
            FormSectionHeader("Sync")
        } footer: {
            if let error = syncError {
                Text(LocalizedStringKey(error))
                    .foregroundStyle(.red)
            } else {
                Text("Compares this device with your account and keeps the newer copy.")
            }
        }

        Section {
            Button("Sign Out", role: .destructive) {
                core.update(.signOut)
                password = ""
            }
            .disabled(isBusy)
            .paperCardRow()
        } footer: {
            Text("Removes the account session from this device. Intentions already on the device are kept.")
        }
    }

    // MARK: - Signed out

    @ViewBuilder
    private var signedOutContent: some View {
        Section {
            TextField("Email", text: $email)
                .textInputAutocapitalization(.never)
                .keyboardType(.emailAddress)
                .textContentType(.username)
                .autocorrectionDisabled()
                .submitLabel(.next)
                .focused($focusedField, equals: .email)
                .onSubmit { focusedField = .password }
                .disabled(isBusy)
                .paperCardRow()

            SecureField("Password", text: $password)
                .textContentType(.password)
                .submitLabel(.go)
                .focused($focusedField, equals: .password)
                .onSubmit(signIn)
                .disabled(isBusy)
                .paperCardRow()
        } header: {
            FormSectionHeader("Sign In")
        } footer: {
            if let error = signInError {
                Text(LocalizedStringKey(error))
                    .foregroundStyle(.red)
            } else {
                Text("Optional. Sign in to back up intentions and restore them on another device. The app works fully offline without an account.")
            }
        }

        Section {
            Button(action: signIn) {
                Text("Sign In")
            }
            .disabled(!canSubmitSignIn)
            .paperCardRow()
        }

        Section {
            NavigationLink {
                CreateAccountView(core: core, email: $email)
            } label: {
                Text("Create Account")
            }
            .disabled(isBusy)
            .paperCardRow()
        } footer: {
            Text("New to sync? Create a private account with the same email you’ll use on other devices.")
        }
    }

    // MARK: - Actions

    private func signIn() {
        guard canSubmitSignIn else { return }
        focusedField = nil
        LocalTimeSync.sync(to: core)
        core.update(.signIn(email: email, password: password))
    }

    // MARK: - State

    private var isSignedIn: Bool {
        switch core.view.accountStatus {
        case .signedIn, .syncing:
            true
        default:
            !core.view.signedInEmail.isEmpty
        }
    }

    private var isSyncing: Bool {
        if case .syncing = core.view.accountStatus { true } else { false }
    }

    private var isBusy: Bool {
        switch core.view.accountStatus {
        case .signingIn, .syncing:
            true
        default:
            false
        }
    }

    private var canSubmitSignIn: Bool {
        !isBusy
            && !email.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !password.isEmpty
    }

    private var signInError: String? {
        guard core.view.accountOperation == .signIn else { return nil }
        return visibleError
    }

    private var syncError: String? {
        guard core.view.accountOperation == .sync else { return nil }
        return visibleError
    }

    private var visibleError: String? {
        let text = core.view.accountError
        return text.isEmpty ? nil : text
    }

    private var lastSyncedLabel: String? {
        guard let seconds = core.view.lastSyncedAt else { return nil }
        let date = Date(timeIntervalSince1970: TimeInterval(seconds))
        return date.formatted(date: .abbreviated, time: .shortened)
    }
}

// MARK: - Create account

private struct CreateAccountView: View {
    @ObservedObject var core: Core
    @Binding var email: String
    @Environment(\.dismiss) private var dismiss

    @State private var password = ""
    @FocusState private var focusedField: Field?

    private enum Field {
        case email
        case password
    }

    var body: some View {
        Form {
            Section {
                TextField("Email", text: $email)
                    .textInputAutocapitalization(.never)
                    .keyboardType(.emailAddress)
                    .textContentType(.username)
                    .autocorrectionDisabled()
                    .submitLabel(.next)
                    .focused($focusedField, equals: .email)
                    .onSubmit { focusedField = .password }
                    .disabled(isBusy)
                    .paperCardRow()

                SecureField("Password", text: $password)
                    .textContentType(.newPassword)
                    .submitLabel(.go)
                    .focused($focusedField, equals: .password)
                    .onSubmit(createAccount)
                    .disabled(isBusy)
                    .paperCardRow()
            } header: {
                FormSectionHeader("Account")
            } footer: {
                if let error = createError {
                    Text(LocalizedStringKey(error))
                        .foregroundStyle(.red)
                } else {
                    Text("Choose a password of at least 8 characters. You can use this email later to restore intentions on another device.")
                }
            }

            Section {
                Button(action: createAccount) {
                    Text("Create Account")
                }
                .disabled(!canSubmit)
                .paperCardRow()
            }
        }
        .paperBackground()
        .scrollDismissesKeyboard(.interactively)
        .navigationTitle("Create Account")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            if isBusy {
                ToolbarItem(placement: .topBarTrailing) {
                    ProgressView()
                }
            }
        }
        .onAppear {
            if core.view.accountOperation != .signUp {
                core.update(.dismissAccountError)
            }
        }
        .onDisappear {
            if core.view.accountOperation == .signUp,
               core.view.accountStatus != .signingIn
            {
                core.update(.dismissAccountError)
            }
        }
        .onChange(of: core.view.accountStatus) { _, status in
            if case .signedIn = status {
                focusedField = nil
                password = ""
                dismiss()
            }
        }
    }

    private func createAccount() {
        guard canSubmit else { return }
        focusedField = nil
        LocalTimeSync.sync(to: core)
        core.update(.signUp(email: email, password: password))
    }

    private var isBusy: Bool {
        if case .signingIn = core.view.accountStatus { true } else { false }
    }

    private var canSubmit: Bool {
        !isBusy
            && !email.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            && !password.isEmpty
    }

    private var createError: String? {
        guard core.view.accountOperation == .signUp else { return nil }
        let text = core.view.accountError
        return text.isEmpty ? nil : text
    }
}

#Preview("Signed out") {
    NavigationStack {
        AccountView(core: Core())
    }
}
