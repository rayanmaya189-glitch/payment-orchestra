# 10 — Offline Support

## 1. Architecture

```
┌─────────────────────────────────────┐
│           View Layer                │
│  (SwiftUI reads from SwiftData)     │
├─────────────────────────────────────┤
│         Repository Layer            │
│  (Checks network → API or SwiftData)│
├─────────────────────────────────────┤
│          Data Layer                 │
│  SwiftData ← SyncQueue → API       │
└─────────────────────────────────────┘
```

---

## 2. SwiftData Models

```swift
import SwiftData

@Model
class PaymentRecord {
    var paymentIntentId: String
    var operatorId: String
    var status: String
    var requestedAmountMinorUnits: Int64
    var authorizedAmountMinorUnits: Int64
    var capturedAmountMinorUnits: Int64
    var refundedAmountMinorUnits: Int64
    var currency: String
    var gatewayProfileId: String?
    var createdAt: Date
    var updatedAt: Date
    var syncStatus: SyncStatus
    
    init(paymentIntentId: String, operatorId: String, status: String, ...) {
        // ... initialize fields
        self.syncStatus = .synced
    }
}

@Model
class SyncQueueRecord {
    var id: UUID
    var operationType: String
    var payload: String // JSON
    var createdAt: Date
    var retryCount: Int
    var status: SyncStatus
    
    init(operationType: String, payload: String) {
        self.id = UUID()
        self.operationType = operationType
        self.payload = payload
        self.createdAt = Date()
        self.retryCount = 0
        self.status = .pending
    }
}

enum SyncStatus: Int, Codable {
    case synced = 0
    case pending = 1
    case syncing = 2
    case failed = 3
}
```

---

## 3. Network Monitor

```swift
import Network

class NetworkMonitor: ObservableObject {
    private let monitor = NWPathMonitor()
    private let queue = DispatchQueue(label: "NetworkMonitor")
    
    @Published var isConnected = true
    
    func startMonitoring() {
        monitor.pathUpdateHandler = { [weak self] path in
            DispatchQueue.main.async {
                self?.isConnected = path.status == .satisfied
            }
        }
        monitor.start(queue: queue)
    }
    
    func stopMonitoring() {
        monitor.cancel()
    }
}
```

---

## 4. Repository with Offline Support

```swift
class PaymentRepository {
    private let api: PaymentAPI
    private let modelContainer: ModelContainer
    private let networkMonitor: NetworkMonitor
    
    @MainActor
    func getPayments() -> [PaymentRecord] {
        let descriptor = FetchDescriptor<PaymentRecord>(sortBy: [SortDescriptor(\.createdAt, order: .reverse)])
        return (try? modelContainer.mainContext.fetch(descriptor)) ?? []
    }
    
    @MainActor
    func createPayment(command: CreatePaymentCommand) async throws -> PaymentRecord {
        if networkMonitor.isConnected {
            let response = try await api.createPayment(command)
            let record = PaymentRecord(from: response, syncStatus: .synced)
            modelContainer.mainContext.insert(record)
            try modelContainer.mainContext.save()
            return record
        } else {
            // Offline: save locally, queue for sync
            let record = PaymentRecord(
                paymentIntentId: UUID().uuidString,
                status: "Created",
                syncStatus: .pending,
                ...
            )
            modelContainer.mainContext.insert(record)
            
            let syncRecord = SyncQueueRecord(
                operationType: "create_payment",
                payload: try JSONEncoder().encode(command).toString()
            )
            modelContainer.mainContext.insert(syncRecord)
            
            try modelContainer.mainContext.save()
            return record
        }
    }
}
```

---

## 5. Background Sync

```swift
class BackgroundSyncManager {
    func registerBackgroundTasks() {
        BGTaskScheduler.shared.register(
            forTaskWithIdentifier: "com.platform.payment.sync",
            using: nil
        ) { task in
            self.handleSync(task: task as! BGAppRefreshTask)
        }
    }
    
    func scheduleSync() {
        let request = BGAppRefreshTaskRequest(identifier: "com.platform.payment.sync")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60)
        try? BGTaskScheduler.shared.submit(request)
    }
    
    private func handleSync(task: BGAppRefreshTask) {
        let task = Task {
            do {
                try await syncPendingOperations()
                task.setTaskCompleted(success: true)
            } catch {
                task.setTaskCompleted(success: false)
            }
        }
        task.expirationHandler = { task.cancel() }
    }
    
    private func syncPendingOperations() async throws {
        // Implementation: fetch pending from SwiftData, call API, update status
    }
}
```

---

## 6. Sync Strategy Matrix

| Operation | Offline Behavior | Online Sync | Conflict Resolution |
|-----------|-----------------|-------------|---------------------|
| View payments | Show cached data | Background refresh | N/A (read-only) |
| Create payment | Save locally, queue sync | POST to API | Server-authoritative |
| Capture payment | Update local status, queue sync | POST to API | Server-authoritative |
| Void payment | Update local status, queue sync | POST to API | Server-authoritative |
| Refund | Update local status, queue sync | POST to API | Server-authoritative |
| View reconciliation | Show cached data | Background refresh | N/A (read-only) |
| Resolve exception | Queue sync | POST to API | Server-authoritative |
| Query AI Assistant | Show "offline" message | POST to API | N/A (no caching) |
| Send feedback | Queue sync | POST to API | Last-write-wins |
