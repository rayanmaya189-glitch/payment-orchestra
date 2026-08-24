# 00 — iOS Architecture

## 1. Tech Stack

```swift
// Package.swift (Swift Package Manager)
dependencies: [
    .package(url: "https://github.com/Alamofire/Alamofire", from: "5.9.0"),
    .package(url: "https://github.com/onevcat/Kingfisher", from: "8.0.0"),
    .package(url: "https://github.com/nicklockwood/SwiftFormat", from: "0.54.0"),
    .package(url: "https://github.com/realm/SwiftLint", from: "0.56.0"),
]
```

### Why This Stack

| Choice | Rationale |
|--------|-----------|
| **Swift 6.0** | Strict concurrency (Sendable, actors), performance, memory safety |
| **SwiftUI** | Declarative UI, native HIG compliance, RTL support, accessibility built-in |
| **SwiftData** | Modern persistence (replaces Core Data), native Swift, iCloud sync ready |
| **Alamofire** | Mature networking with interceptors, retry, certificate pinning |
| **Kingfisher** | Image caching, downsample, memory/disk cache |
| **Swift Charts** | Native charting framework (iOS 16+) |
| **Factory/DI** | Lightweight DI without runtime reflection |

---

## 2. Project Structure

```
PaymentPlatform/
├── App/
│   ├── PaymentPlatformApp.swift      # App entry point
│   ├── ContentView.swift             # Root view with auth routing
│   └── AppDelegate.swift             # Scene lifecycle
│
├── Features/
│   ├── Auth/
│   │   ├── LoginView.swift
│   │   ├── RegisterView.swift
│   │   └── AuthViewModel.swift
│   │
│   ├── Dashboard/
│   │   ├── DashboardView.swift
│   │   ├── DashboardViewModel.swift
│   │   └── Components/
│   │       ├── StatsCard.swift
│   │       ├── AuthRateChart.swift
│   │       └── RecentActivityList.swift
│   │
│   ├── Payments/
│   │   ├── PaymentListView.swift
│   │   ├── PaymentDetailView.swift
│   │   ├── CreatePaymentView.swift
│   │   ├── PaymentListViewModel.swift
│   │   ├── PaymentDetailViewModel.swift
│   │   └── Components/
│   │       ├── GatewayProfileSection.swift
│   │       ├── RoutingTimeline.swift
│   │       └── FeeBreakdown.swift
│   │
│   ├── Reconciliation/
│   │   ├── ReconciliationView.swift
│   │   ├── ExceptionQueueView.swift
│   │   ├── ReconciliationViewModel.swift
│   │   └── Components/
│   │       └── ExceptionCard.swift
│   │
│   ├── Assistant/
│   │   ├── AssistantView.swift
│   │   ├── AssistantViewModel.swift
│   │   └── Components/
│   │       ├── ChatBubble.swift
│   │       ├── CitationChip.swift
│   │       └── ChatInput.swift
│   │
│   ├── Invoices/
│   │   ├── InvoiceListView.swift
│   │   ├── InvoiceDetailView.swift
│   │   └── InvoiceViewModel.swift
│   │
│   ├── Subscriptions/
│   │   ├── SubscriptionListView.swift
│   │   └── SubscriptionViewModel.swift
│   │
│   ├── Connectors/
│   │   ├── ConnectorListView.swift
│   │   ├── ConnectorDetailView.swift
│   │   ├── ConnectorViewModel.swift
│   │   └── Components/
│   │       ├── GatewayProfileCard.swift
│   │       └── RotationStrategyPicker.swift
│   │
│   └── Settings/
│       ├── SettingsView.swift
│       ├── ApiKeysView.swift
│       ├── UsersView.swift
│       ├── RoutingConfigView.swift
│       ├── ComplianceView.swift
│       └── SettingsViewModel.swift
│
├── Core/
│   ├── Network/
│   │   ├── APIClient.swift           # Alamofire configuration
│   │   ├── APIRouter.swift           # Endpoint definitions
│   │   ├── interceptors/
│   │   │   ├── AuthInterceptor.swift
│   │   │   ├── IdempotencyInterceptor.swift
│   │   │   └── ErrorInterceptor.swift
│   │   └── WebSocket/
│   │       └── PaymentWebSocket.swift
│   │
│   ├── Storage/
│   │   ├── Models/                    # SwiftData models
│   │   │   ├── PaymentModel.swift
│   │   │   └── SyncQueueModel.swift
│   │   ├── Repositories/
│   │   │   ├── PaymentRepository.swift
│   │   │   └── SyncRepository.swift
│   │   └── Keychain/
│   │       └── KeychainManager.swift
│   │
│   └── Security/
│       ├── BiometricAuth.swift
│       ├── CertificatePinning.swift
│       └── KeychainStorage.swift
│
├── Shared/
│   ├── Components/
│   │   ├── StatusBadge.swift
│   │   ├── MoneyDisplay.swift
│   │   ├── LoadingView.swift
│   │   └── EmptyState.swift
│   ├── Extensions/
│   │   ├── Date+Extensions.swift
│   │   ├── Money+Extensions.swift
│   │   └── View+Extensions.swift
│   └── Models/
│       ├── Money.swift
│       ├── Currency.swift
│       └── PaymentStatus.swift
│
└── Resources/
    ├── Assets.xcassets
    ├── Localizable.xcstrings        # Arabic + English
    └── Info.plist
```

---

## 3. MVVM + Clean Architecture

```
┌─────────────────────────────────────┐
│            View Layer               │
│  (SwiftUI Views + ViewModels)       │
├─────────────────────────────────────┤
│         Domain Layer                │
│  (Use Cases + Domain Models)        │
├─────────────────────────────────────┤
│          Data Layer                 │
│  (Repositories + SwiftData + API)   │
└─────────────────────────────────────┘
```

### Data Flow

```
User Action → ViewModel → UseCase → Repository → API/SwiftData → UseCase → ViewModel → View
                                        ↓
                              Offline: SwiftData
                              Online: Alamofire API
                              Sync: BackgroundTasks
```

---

## 4. Dependency Injection

```swift
// Using Factory (https://github.com/hmlongco/Factory)
import Factory

extension Container {
    // Network
    static let apiClient = Factory<APIClient> { APIClient() }
    static let paymentAPI = Factory<PaymentAPI> { PaymentAPI(client: Container.apiClient()) }
    
    // Storage
    static let keychainManager = Factory<KeychainManager> { KeychainManager() }
    static let syncRepository = Factory<SyncRepository> { SyncRepository() }
    
    // ViewModels
    static let dashboardVM = Factory<DashboardViewModel> {
        DashboardViewModel(paymentAPI: Container.paymentAPI())
    }
}

// Usage in View
struct DashboardView: View {
    @StateObject var viewModel = Container.dashboardVM()
    
    var body: some View {
        // ...
    }
}
```

---

## 5. Navigation (SwiftUI Navigation)

```swift
// App entry point
@main
struct PaymentPlatformApp: App {
    @StateObject var authManager = AuthManager()
    
    var body: some Scene {
        WindowGroup {
            NavigationStack {
                if authManager.isAuthenticated {
                    MainTabView()
                } else {
                    LoginView()
                }
            }
            .environmentObject(authManager)
        }
    }
}

// Tab-based navigation
struct MainTabView: View {
    var body: some View {
        TabView {
            DashboardView()
                .tabItem { Label("Dashboard", systemImage: "chart.bar") }
            
            PaymentListView()
                .tabItem { Label("Payments", systemImage: "creditcard") }
            
            ReconciliationView()
                .tabItem { Label("Reconcile", systemImage: "checkmark.circle") }
            
            AssistantView()
                .tabItem { Label("Assistant", systemImage: "brain") }
            
            SettingsView()
                .tabItem { Label("Settings", systemImage: "gearshape") }
        }
    }
}
```

---

## 6. Keychain Storage

```swift
class KeychainManager {
    private let service = "com.platform.payment"
    
    func save(key: String, value: Data, accessible: CFString = kSecAttrAccessibleWhenUnlockedThisDeviceOnly) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecValueData as String: value,
            kSecAttrAccessible as String: accessible
        ]
        
        SecItemDelete(query as CFDictionary) // Remove existing
        SecItemAdd(query as CFDictionary, nil)
    }
    
    func load(key: String) -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        var result: AnyObject?
        SecItemCopyMatching(query as CFDictionary, &result)
        return result as? Data
    }
    
    func delete(key: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key
        ]
        SecItemDelete(query as CFDictionary)
    }
    
    func clearAll() {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service
        ]
        SecItemDelete(query as CFDictionary)
    }
}
```

---

## 7. Offline Support

```swift
// BackgroundTasks for sync
class BackgroundTaskManager {
    func registerTasks() {
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.platform.payment.offlineSync",
            using: nil
        ) { task in
            self.handleSync(task: task as! BGAppRefreshTask)
        }
    }
    
    func scheduleSync() {
        let request = BGAppRefreshTaskRequest(identifier: "com.platform.payment.offlineSync")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 minutes
        try? BGTaskScheduler.shared.submit(request)
    }
    
    func handleSync(task: BGAppRefreshTask) {
        let syncTask = Task {
            do {
                try await syncRepository.syncPendingOperations()
                task.setTaskCompleted(success: true)
            } catch {
                task.setTaskCompleted(success: false)
            }
        }
        task.expirationHandler = { syncTask.cancel() }
    }
}
```

---

## 8. Push Notifications

```swift
import UserNotifications

class PushNotificationManager: NSObject, UNUserNotificationCenterDelegate {
    func registerForPushNotifications() {
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound]) { granted, _ in
            guard granted else { return }
            DispatchQueue.main.async {
                UIApplication.shared.registerForRemoteNotifications()
            }
        }
    }
    
    func application(_ application: UIApplication, didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data) {
        let token = deviceToken.map { String(format: "%02.2hhx", $0) }.joined()
        // Register token with backend
        registerTokenWithBackend(token)
    }
    
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification) async -> UNNotificationPresentationOptions {
        // Show notification even when app is in foreground
        return [.banner, .badge, .sound]
    }
}
```
