# 10 — Offline Support

## 1. Architecture

```
┌─────────────────────────────────────┐
│           UI Layer                  │
│  (Compose reads from Room Flow)     │
├─────────────────────────────────────┤
│         Repository Layer            │
│  (Checks network → API or Room)     │
├─────────────────────────────────────┤
│          Data Layer                 │
│  Room (offline) ← SyncQueue → API   │
└─────────────────────────────────────┘
```

### Sync Strategy

- **Reads**: Always from Room (offline-first). Background sync updates Room from API.
- **Writes**: Write to Room immediately (optimistic). Queue sync operation. Sync when online.
- **Conflict resolution**: Last-write-wins for non-financial data. Server-authoritative for financial operations.

---

## 2. Room Entities

```kotlin
@Entity(tableName = "payments")
data class PaymentEntity(
    @PrimaryKey val paymentIntentId: String,
    val operatorId: String,
    val status: String,
    val requestedAmountMinorUnits: Long,
    val authorizedAmountMinorUnits: Long,
    val capturedAmountMinorUnits: Long,
    val refundedAmountMinorUnits: Long,
    val currency: String,
    val gatewayProfileId: String?,
    val createdAt: Long,
    val updatedAt: Long,
    val syncStatus: SyncStatus = SyncStatus.SYNCED
)

@Entity(tableName = "sync_queue")
data class SyncQueueEntity(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val operationType: String, // 'create_payment', 'capture', 'void', 'refund'
    val payload: String, // JSON
    val createdAt: Long,
    val retryCount: Int = 0,
    val status: SyncStatus = SyncStatus.PENDING
)

enum class SyncStatus {
    SYNCED,
    PENDING,
    SYNCING,
    FAILED
}
```

---

## 3. DAO Interfaces

```kotlin
@Dao
interface PaymentDao {
    @Query("SELECT * FROM payments ORDER BY createdAt DESC")
    fun getAllPayments(): Flow<List<PaymentEntity>>

    @Query("SELECT * FROM payments WHERE paymentIntentId = :id")
    fun getPaymentById(id: String): Flow<PaymentEntity?>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertPayment(payment: PaymentEntity)

    @Update
    suspend fun updatePayment(payment: PaymentEntity)

    @Query("UPDATE payments SET syncStatus = :status WHERE paymentIntentId = :id")
    suspend fun updateSyncStatus(id: String, status: SyncStatus)
}

@Dao
interface SyncQueueDao {
    @Query("SELECT * FROM sync_queue WHERE status = 'PENDING' ORDER BY createdAt ASC")
    suspend fun getPendingOperations(): List<SyncQueueEntity>

    @Insert
    suspend fun insertOperation(operation: SyncQueueEntity)

    @Update
    suspend fun updateOperation(operation: SyncQueueEntity)

    @Query("DELETE FROM sync_queue WHERE id = :id")
    suspend fun deleteOperation(id: Long)
}
```

---

## 4. Repository with Offline Support

```kotlin
class PaymentRepositoryImpl(
    private val api: PaymentApi,
    private val paymentDao: PaymentDao,
    private val syncQueueDao: SyncQueueDao,
    private val networkMonitor: NetworkMonitor
) : PaymentRepository {

    override fun getPayments(): Flow<List<PaymentIntent>> {
        return paymentDao.getAllPayments().map { entities ->
            entities.map { it.toDomain() }
        }
    }

    override suspend fun createPayment(command: CreatePaymentCommand): Result<PaymentIntent> {
        return try {
            if (networkMonitor.isOnline()) {
                // Online: call API
                val response = api.createPayment(command.toRequest())
                val entity = response.toEntity(syncStatus = SyncStatus.SYNCED)
                paymentDao.insertPayment(entity)
                Result.success(response.toDomain())
            } else {
                // Offline: save locally, queue for sync
                val entity = PaymentEntity(
                    paymentIntentId = UUID.randomUUID().toString(),
                    status = "Created",
                    syncStatus = SyncStatus.PENDING,
                    // ... other fields
                )
                paymentDao.insertPayment(entity)
                
                syncQueueDao.insertOperation(SyncQueueEntity(
                    operationType = "create_payment",
                    payload = Gson().toJson(command)
                ))
                
                Result.success(entity.toDomain())
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    override suspend fun capturePayment(id: String, amount: Money?): Result<PaymentIntent> {
        return try {
            if (networkMonitor.isOnline()) {
                val response = api.capturePayment(id, amount?.toRequest())
                paymentDao.updatePayment(response.toEntity(syncStatus = SyncStatus.SYNCED))
                Result.success(response.toDomain())
            } else {
                // Queue for sync
                syncQueueDao.insertOperation(SyncQueueEntity(
                    operationType = "capture",
                    payload = Gson().toJson(mapOf("id" to id, "amount" to amount))
                ))
                // Update local status optimistically
                paymentDao.updateSyncStatus(id, SyncStatus.PENDING)
                Result.success(getLocalPayment(id))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
```

---

## 5. Background Sync Worker

```kotlin
@HiltWorker
class OfflineSyncWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted workerParams: WorkerParameters,
    private val paymentApi: PaymentApi,
    private val syncQueueDao: SyncQueueDao,
    private val paymentDao: PaymentDao
) : CoroutineWorker(context, workerParams) {

    override suspend fun doWork(): Result {
        val pendingOperations = syncQueueDao.getPendingOperations()
        
        for (operation in pendingOperations) {
            try {
                syncQueueDao.updateOperation(operation.copy(status = SyncStatus.SYNCING))
                
                when (operation.operationType) {
                    "create_payment" -> {
                        val command = Gson().fromJson(operation.payload, CreatePaymentCommand::class.java)
                        val response = paymentApi.createPayment(command.toRequest())
                        paymentDao.updatePayment(response.toEntity(syncStatus = SyncStatus.SYNCED))
                    }
                    "capture" -> {
                        val data = Gson().fromJson(operation.payload, Map::class.java)
                        val response = paymentApi.capturePayment(data["id"] as String, null)
                        paymentDao.updatePayment(response.toEntity(syncStatus = SyncStatus.SYNCED))
                    }
                    // ... other operations
                }
                
                syncQueueDao.deleteOperation(operation.id)
            } catch (e: Exception) {
                syncQueueDao.updateOperation(
                    operation.copy(
                        status = SyncStatus.FAILED,
                        retryCount = operation.retryCount + 1
                    )
                )
                
                if (operation.retryCount >= 3) {
                    // Max retries reached — notify user
                    return Result.failure(e)
                }
            }
        }
        
        return Result.success()
    }
}
```

---

## 6. Network Monitor

```kotlin
class NetworkMonitor @Inject constructor(
    @ApplicationContext private val context: Context
) {
    private val connectivityManager = context.getSystemService<ConnectivityManager>()!!

    val isOnline: Flow<Boolean> = callbackFlow {
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                trySend(true)
            }
            override fun onLost(network: Network) {
                trySend(false)
            }
        }
        
        val request = NetworkRequest.Builder()
            .addCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
            .build()
        
        connectivityManager.registerNetworkCallback(request, callback)
        
        // Initial state
        val currentNetwork = connectivityManager.activeNetwork
        val capabilities = connectivityManager.getNetworkCapabilities(currentNetwork)
        trySend(capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true)
        
        awaitClose {
            connectivityManager.unregisterNetworkCallback(callback)
        }
    }

    suspend fun isOnline(): Boolean {
        return isOnline.first()
    }
}
```

---

## 7. Sync Strategy Matrix

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
