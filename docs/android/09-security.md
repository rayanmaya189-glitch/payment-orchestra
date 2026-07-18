# 09 — Security

## 1. Biometric Authentication

```kotlin
class BiometricAuthManager(private val context: Context) {
    
    fun authenticate(
        onSuccess: () -> Unit,
        onError: (String) -> Unit,
        onFallback: () -> Unit
    ) {
        val biometricPrompt = BiometricPrompt(
            FragmentActivity(context),
            ContextCompat.getMainExecutor(context),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    onSuccess()
                }
                
                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    when (errorCode) {
                        BiometricPrompt.ERROR_NEGATIVE_USER_BUTTON,
                        BiometricPrompt.ERROR_CANCELED -> onFallback()
                        else -> onError(errString.toString())
                    }
                }
                
                override fun onAuthenticationFailed() {
                    // Biometric not recognized — no action needed
                }
            }
        )

        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(context.getString(R.string.biometric_title))
            .setSubtitle(context.getString(R.string.biometric_subtitle))
            .setNegativeButtonText(context.getString(R.string.use_password))
            .build()

        biometricPrompt.authenticate(promptInfo)
    }
}
```

---

## 2. Certificate Pinning

```kotlin
// OkHttp certificate pinning
fun certificatePinner(): CertificatePinner {
    return CertificatePinner.Builder()
        .add(
            "api.platform.ae",
            "sha256/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" // Production cert
        )
        .add(
            "api.platform.ae",
            "sha256/BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=" // Backup cert
        )
        .add(
            "ai.platform.ae",
            "sha256/CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC=" // AI service cert
        )
        .build()
}
```

---

## 3. Secure Network Config

```xml
<!-- network_security_config.xml -->
<network-security-config>
    <!-- Production: certificate pinning -->
    <domain-config>
        <domain includeSubdomains="true">api.platform.ae</domain>
        <pin-set expiration="2027-01-01">
            <pin digest="SHA-256">AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=</pin>
            <pin digest="SHA-256">BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=</pin>
        </pin-set>
    </domain-config>

    <!-- Development: trust user-added CAs -->
    <domain-config cleartextTrafficPermitted="false">
        <domain includeSubdomains="true">localhost</domain>
        <trust-anchors>
            <certificates src="system" />
            <certificates src="user" />
        </trust-anchors>
    </domain-config>
</network-security-config>
```

---

## 4. Encrypted Local Storage

```kotlin
object SecureStorage {
    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val encryptedPrefs = EncryptedSharedPreferences.create(
        context,
        "secure_storage",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )

    // Store auth tokens
    fun saveAuthTokens(accessToken: String, refreshToken: String) {
        encryptedPrefs.edit()
            .putString("access_token", accessToken)
            .putString("refresh_token", refreshToken)
            .apply()
    }

    fun getAccessToken(): String? = encryptedPrefs.getString("access_token", null)
    fun getRefreshToken(): String? = encryptedPrefs.getString("refresh_token", null)

    // Store API key
    fun saveApiKey(keyId: String, keyHash: String) {
        encryptedPrefs.edit()
            .putString("api_key_$keyId", keyHash)
            .apply()
    }

    // Clear all secure data
    fun clearAll() {
        encryptedPrefs.edit().clear().apply()
    }
}
```

---

## 5. Root/Jailbreak Detection

```kotlin
object RootDetection {
    fun isDeviceRooted(context: Context): Boolean {
        // Check for su binary
        val suPaths = listOf(
            "/system/app/Superuser.apk",
            "/system/bin/su",
            "/system/xbin/su",
            "/sbin/su"
        )
        if (suPaths.any { File(it).exists() }) return true

        // Check for Magisk
        if (File("/sbin/.magisk").exists()) return true

        // Check for test-keys build
        if (Build.TAGS?.contains("test-keys") == true) return true

        // Check for dangerous props
        if (Build.TYPE == "userdebug") return true

        return false
    }

    fun isEmulator(): Boolean {
        return Build.FINGERPRINT.startsWith("generic")
            || Build.FINGERPRINT.startsWith("unknown")
            || Build.MODEL.contains("Emulator")
            || Build.MODEL.contains("Android SDK")
    }
}

// In Application class
class App : Application() {
    override fun onCreate() {
        super.onCreate()
        
        if (RootDetection.isDeviceRooted(this) || RootDetection.isEmulator()) {
            // Block app or show warning
            // For bank-grade: block rooted devices entirely
            throw SecurityException("Rooted/emulator device detected")
        }
    }
}
```

---

## 6. ProGuard/R8 Rules

```proguard
# Keep Moshi models
-keep class com.platform.payment.data.remote.model.** { *; }

# Keep Retrofit interfaces
-keep class com.platform.payment.data.remote.api.** { *; }

# Keep Room entities
-keep class com.platform.payment.data.local.entity.** { *; }

# Obfuscation
-repackageclasses ''
-allowaccessmodification
-optimizations !code/simplification/arithmetic

# Don't warn about missing classes
-dontwarn javax.annotation.**
```

---

## 7. Security Checklist

| Control | Implementation |
|---------|---------------|
| Biometric auth | `BiometricPrompt` API with fallback to PIN |
| Encrypted storage | `EncryptedSharedPreferences` (AES-256-GCM) |
| Certificate pinning | OkHttp `CertificatePinner` with backup pins |
| Root detection | Check for su binary, Magisk, test-keys |
| ProGuard obfuscation | R8 with aggressive optimizations |
| Network security | `network_security_config.xml` |
| Secure random | `SecureRandom` for all crypto operations |
| No sensitive data in logs | ProGuard strips `Log.d`/`Log.v` in release |
| Session timeout | 30min idle for Admin/Finance, 60min for others |
| Auto-logout | On app background >5min (configurable) |
