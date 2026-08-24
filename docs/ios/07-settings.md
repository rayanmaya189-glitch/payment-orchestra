# 07 — Settings

## 1. Settings Screen

```swift
struct SettingsView: View {
    @StateObject var viewModel = Container.settingsVM()
    
    var body: some View {
        List {
            Section("Account") {
                NavigationLink(destination: ApiKeysView()) {
                    Label("API Keys", systemImage: "key")
                }
                NavigationLink(destination: UsersView()) {
                    Label("Users", systemImage: "person.2")
                }
            }
            
            Section("Configuration") {
                NavigationLink(destination: RoutingConfigView()) {
                    Label("Routing Policy", systemImage: "arrow.triangle.branch")
                }
                NavigationLink(destination: ComplianceView()) {
                    Label("Compliance", systemImage: "lock.shield")
                }
            }
            
            Section("Preferences") {
                NavigationLink(destination: NotificationSettingsView()) {
                    Label("Notifications", systemImage: "bell")
                }
                
                Picker("Language", selection: $viewModel.language) {
                    Text("English").tag("en")
                    Text("العربية").tag("ar")
                }
            }
            
            Section {
                Button("Logout", role: .destructive) {
                    viewModel.logout()
                }
            }
        }
        .navigationTitle("Settings")
    }
}
```

---

## 2. API Keys

```swift
struct ApiKeysView: View {
    @StateObject var viewModel = Container.apiKeysVM()
    @State private var showCreate = false
    
    var body: some View {
        List {
            ForEach(viewModel.apiKeys) { key in
                ApiKeyRow(
                    apiKey: key,
                    onRotate: { viewModel.rotate(key) },
                    onRevoke: { viewModel.revoke(key) }
                )
            }
        }
        .navigationTitle("API Keys")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button { showCreate = true } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showCreate) {
            CreateApiKeySheet { name, scopes in
                viewModel.create(name: name, scopes: scopes)
                showCreate = false
            }
        }
    }
}

struct ApiKeyRow: View {
    let apiKey: ApiKey
    let onRotate: () -> Void
    let onRevoke: () -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(apiKey.name)
                    .font(.headline)
                Spacer()
                StatusBadge(status: apiKey.status)
            }
            
            HStack {
                Image(systemName: "key")
                    .font(.caption)
                Text(apiKey.keyPrefix)
                    .font(.caption.monospaced())
                Button { copyToClipboard(apiKey.keyPrefix) } label: {
                    Image(systemName: "doc.on.doc")
                        .font(.caption)
                }
            }
            
            HStack {
                ForEach(apiKey.scopes, id: \.self) { scope in
                    Text(scope)
                        .font(.caption2)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(.blue.opacity(0.1))
                        .clipShape(Capsule())
                }
            }
            
            Text("Expires: \(apiKey.expiresAt.relativeFormatted)")
                .font(.caption)
                .foregroundStyle(.secondary)
            
            HStack(spacing: 8) {
                Button("Rotate", action: onRotate).buttonStyle(.bordered)
                Button("Revoke", role: .destructive, action: onRevoke).buttonStyle(.bordered)
            }
        }
    }
}
```
