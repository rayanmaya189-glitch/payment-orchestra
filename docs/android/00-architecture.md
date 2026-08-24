# 00 — Android Architecture

## 1. Tech Stack

```kotlin
// Build dependencies (build.gradle.kts)
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("com.google.dagger.hilt.android")
    id("com.google.devtools.ksp")
    id("org.jetbrains.kotlin.plugin.compose")
}

dependencies {
    // UI
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.navigation:navigation-compose")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose")
    
    // Architecture
    implementation("androidx.hilt:hilt-navigation-compose")
    implementation("androidx.lifecycle:lifecycle-runtime-compose")
    
    // Networking
    implementation("com.squareup.retrofit2:retrofit")
    implementation("com.squareup.okhttp3:okhttp")
    implementation("com.squareup.okhttp3:logging-interceptor")
    implementation("com.squareup.moshi:moshi-kotlin-codegen")
    
    // Local Storage
    implementation("androidx.room:room-runtime")
    implementation("androidx.room:room-ktx")
    implementation("androidx.datastore:datastore-preferences")
    implementation("androidx.security:security-crypto")
    
    // Image Loading
    implementation("io.coil-kt:coil-compose")
    
    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-play-services")
    
    // Testing
    testImplementation("junit:junit:4.13.2")
    testImplementation("io.mockk:mockk")
    testImplementation("app.cash.turbine:turbine")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test")
}
```

## 2. Project Structure

```
app/src/main/java/com/platform/payment/
├── App.kt                              # Hilt Application
├── MainActivity.kt                     # Single Activity (Compose)
│
├── data/
│   ├── remote/
│   │   ├── api/
│   │   │   ├── PaymentApi.kt           # Retrofit API interface
│   │   │   ├── AuthApi.kt
│   │   │   ├── ReconciliationApi.kt
│   │   │   ├── AssistantApi.kt
│   │   │   └── interceptor/
│   │   │       ├── AuthInterceptor.kt   # JWT token injection
│   │   │       ├── IdempotencyInterceptor.kt
│   │   │       └── ErrorInterceptor.kt
│   │   ├── model/                       # API response models
│   │   └── websocket/
│   │       └── PaymentWebSocket.kt      # Real-time updates
│   │
│   ├── local/
│   │   ├── db/
│   │   │   ├── AppDatabase.kt          # Room database
│   │   │   ├── dao/
│   │   │   │   ├── PaymentDao.kt
│   │   │   │   ├── ReconciliationDao.kt
│   │   │   │   └── SyncQueueDao.kt
│   │   │   └── entity/
│   │   │       ├── PaymentEntity.kt
│   │   │       └── SyncQueueEntity.kt
│   │   ├── datastore/
│   │   │   ├── AuthDataStore.kt        # Encrypted token storage
│   │   │   └── SettingsDataStore.kt
│   │   └── crypto/
│   │       └── EncryptedStorage.kt     # EncryptedSharedPreferences wrapper
│   │
│   └── repository/
│       ├── PaymentRepository.kt
│       ├── ReconciliationRepository.kt
│       ├── AuthRepository.kt
│       └── OfflineSyncRepository.kt
│
├── domain/
│   ├── model/                          # Domain models
│   │   ├── PaymentIntent.kt
│   │   ├── Money.kt
│   │   ├── Currency.kt
│   │   ├── PaymentStatus.kt
│   │   └── GatewayProfile.kt
│   └── usecase/
│       ├── GetPaymentsUseCase.kt
│       ├── CreatePaymentUseCase.kt
│       ├── CapturePaymentUseCase.kt
│       └── SyncOfflineQueueUseCase.kt
│
├── ui/
│   ├── navigation/
│   │   └── AppNavigation.kt            # Compose Navigation graph
│   ├── theme/
│   │   ├── Theme.kt                    # Material 3 theme
│   │   ├── Color.kt
│   │   └── Type.kt
│   ├── dashboard/
│   │   ├── DashboardScreen.kt
│   │   └── DashboardViewModel.kt
│   ├── payments/
│   │   ├── PaymentListScreen.kt
│   │   ├── PaymentDetailScreen.kt
│   │   ├── CreatePaymentScreen.kt
│   │   └── PaymentViewModel.kt
│   ├── reconciliation/
│   │   ├── ReconciliationScreen.kt
│   │   └── ReconciliationViewModel.kt
│   ├── assistant/
│   │   ├── AssistantScreen.kt
│   │   └── AssistantViewModel.kt
│   ├── settings/
│   │   ├── SettingsScreen.kt
│   │   └── SettingsViewModel.kt
│   └── components/                     # Shared Compose components
│       ├── StatusBadge.kt
│       ├── MoneyDisplay.kt
│       ├── DataTable.kt
│       └── LoadingState.kt
│
├── di/                                 # Hilt modules
│   ├── NetworkModule.kt
│   ├── DatabaseModule.kt
│   └── RepositoryModule.kt
│
└── util/
    ├── DateUtils.kt
    ├── CurrencyUtils.kt
    └── OfflineUtils.kt
```

---

## 3. MVVM + Clean Architecture Layers

```
┌─────────────────────────────────────┐
│            UI Layer                 │
│  (Compose Screens + ViewModels)     │
├─────────────────────────────────────┤
│         Domain Layer                │
│  (Use Cases + Domain Models)        │
├─────────────────────────────────────┤
│          Data Layer                 │
│  (Repositories + Room + Retrofit)   │
└─────────────────────────────────────┘
```

### Data Flow

```
User Action → ViewModel → UseCase → Repository → API/DB → UseCase → ViewModel → UI
                                        ↓
                              Offline: Room DB
                              Online: Retrofit API
                              Sync: Background WorkManager
```

### Dependency Injection (Hilt)

```kotlin
@Module
@InstallIn(SingletonComponent::class)
object NetworkModule {
    @Provides
    @Singleton
    fun provideRetrofit(okHttpClient: OkHttpClient, moshi: Moshi): Retrofit {
        return Retrofit.Builder()
            .baseUrl(BuildConfig.API_URL)
            .client(okHttpClient)
            .addConverterFactory(MoshiConverterFactory.create(moshi))
            .build()
    }

    @Provides
    @Singleton
    fun provideOkHttpClient(authInterceptor: AuthInterceptor): OkHttpClient {
        return OkHttpClient.Builder()
            .addInterceptor(authInterceptor)
            .addInterceptor(IdempotencyInterceptor())
            .addInterceptor(ErrorInterceptor())
            .certificatePinner(certificatePinner()) // Certificate pinning
            .build()
    }
}
```

---

## 4. Navigation (Compose Navigation)

```kotlin
@Composable
fun AppNavigation(navController: NavHostController) {
    NavHost(navController, startDestination = "dashboard") {
        // Auth routes
        composable("login") { LoginScreen() }
        composable("register") { RegisterScreen() }
        
        // Dashboard routes
        composable("dashboard") { DashboardScreen() }
        
        // Payment routes
        composable("payments") { PaymentListScreen() }
        composable("payments/{id}") { backStackEntry ->
            PaymentDetailScreen(paymentId = backStackEntry.arguments?.getString("id")!!)
        }
        composable("payments/create") { CreatePaymentScreen() }
        
        // Reconciliation routes
        composable("reconciliation") { ReconciliationScreen() }
        composable("reconciliation/exceptions") { ExceptionQueueScreen() }
        
        // AI Assistant
        composable("assistant") { AssistantScreen() }
        
        // Settings routes
        composable("settings") { SettingsScreen() }
        composable("settings/api-keys") { ApiKeysScreen() }
        composable("settings/users") { UsersScreen() }
        composable("settings/routing") { RoutingConfigScreen() }
        composable("settings/compliance") { ComplianceScreen() }
    }
}
```

---

## 5. Encrypted Storage

```kotlin
object EncryptedStorage {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val encryptedPrefs = EncryptedSharedPreferences.create(
        context,
        "secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    fun saveAuthToken(token: String) {
        encryptedPrefs.edit().putString("auth_token", token).apply()
    }

    fun getAuthToken(): String? {
        return encryptedPrefs.getString("auth_token", null)
    }

    fun clearAll() {
        encryptedPrefs.edit().clear().apply()
    }
}
```

---

## 6. Offline Support

```kotlin
// WorkManager for background sync
@HiltWorker
class OfflineSyncWorker @AssistedInject constructor(
    @Assisted context: Context,
    @Assisted workerParams: WorkerParameters,
    private val syncRepository: OfflineSyncRepository
) : CoroutineWorker(context, workerParams) {

    override suspend fun doWork(): Result {
        return try {
            syncRepository.syncPendingOperations()
            Result.success()
        } catch (e: Exception) {
            Result.retry()
        }
    }
}

// Schedule periodic sync
val syncRequest = PeriodicWorkRequestBuilder<OfflineSyncWorker>(
    15, TimeUnit.MINUTES
).setConstraints(
    Constraints.Builder()
        .setRequiredNetworkType(NetworkType.CONNECTED)
        .build()
).build()

WorkManager.getInstance(context).enqueueUniquePeriodicWork(
    "offline_sync",
    ExistingPeriodicWorkPolicy.KEEP,
    syncRequest
)
```

---

## 7. Push Notifications

```kotlin
// Firebase Cloud Messaging integration
@AndroidEntryPoint
class FCMService : FirebaseMessagingService() {

    @Inject lateinit var notificationHandler: NotificationHandler

    override fun onMessageReceived(message: RemoteMessage) {
        notificationHandler.handle(message)
    }

    override fun onNewToken(token: String) {
        // Register new FCM token with backend
        notificationHandler.registerToken(token)
    }
}

// Notification types
enum class NotificationType {
    PAYMENT_STATUS_CHANGED,
    SETTLEMENT_MATCHED,
    AML_ALERT,
    SUBSCRIPTION_RENEWAL_FAILED,
    API_KEY_EXPIRING,
    AI_ASSISTANT_RESPONSE
}
```
