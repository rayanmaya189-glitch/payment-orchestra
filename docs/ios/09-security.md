# 09 — Security

## 1. Biometric Authentication

```swift
import LocalAuthentication

class BiometricAuthManager {
    func authenticate() async throws -> Bool {
        let context = LAContext()
        var error: NSError?
        
        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
            throw BiometricError.notAvailable(error?.localizedDescription ?? "Biometrics not available")
        }
        
        return try await withCheckedThrowingContinuation { continuation in
            context.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: "Authenticate to access Payment Platform") { success, error in
                if success {
                    continuation.resume(returning: true)
                } else if let error = error {
                    continuation.resume(throwing: BiometricError.authenticationFailed(error.localizedDescription))
                } else {
                    continuation.resume(returning: false)
                }
            }
        }
    }
    
    enum BiometricError: Error {
        case notAvailable(String)
        case authenticationFailed(String)
    }
}
```

---

## 2. Keychain Storage

```swift
class KeychainManager {
    private let service = "com.platform.payment"
    
    func save(key: String, value: Data) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecValueData as String: value,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]
        
        SecItemDelete(query as CFDictionary)
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status)
        }
    }
    
    func load(key: String) throws -> Data {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data else {
            throw KeychainError.loadFailed(status)
        }
        return data
    }
    
    func delete(key: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key
        ]
        SecItemDelete(query as CFDictionary)
    }
    
    enum KeychainError: Error {
        case saveFailed(OSStatus)
        case loadFailed(OSStatus)
    }
}
```

---

## 3. Certificate Pinning

```swift
import Alamofire

class CertificatePinningTrustManager: ServerTrustEvaluating {
    let certificates: [SecCertificate]
    
    init(certificates: [SecCertificate]) {
        self.certificates = certificates
    }
    
    func evaluateTrust(_ trust: SecTrust, forHost host: String) throws {
        // Evaluate certificate chain
        var error: CFError?
        guard SecTrustEvaluateWithError(trust, &error) else {
            throw AFError.serverTrustEvaluationFailed(reason: .trustEvaluationFailed(error: error))
        }
        
        // Check if server certificate matches pinned certificates
        guard let serverCertificate = SecTrustGetCertificateAtIndex(trust, 0) else {
            throw AFError.serverTrustEvaluationFailed(reason: .noPublicKeys)
        }
        
        let serverCertData = SecCertificateCopyData(serverCertificate) as Data
        let pinnedCertData = certificates.map { SecCertificateCopyData($0) as Data }
        
        guard pinnedCertData.contains(serverCertData) else {
            throw AFError.serverTrustEvaluationFailed(reason: .publicKeyPinningFailed(
                host: host,
                trust: trust,
                pinnedPublicKeys: certificates.compactMap { SecCertificateCopyKey($0) },
                serverPublicKey: SecCertificateCopyKey(serverCertificate)
            ))
        }
    }
}

// Usage
let session = Session(
    serverTrustManager: ServerTrustManager(
        allHostsTrustEvaluators: [
            "api.platform.ae": CertificatePinningTrustManager(certificates: [/* pinned certs */]),
            "ai.platform.ae": CertificatePinningTrustManager(certificates: [/* pinned certs */])
        ]
    )
)
```

---

## 4. Device Security Checks

```swift
class DeviceSecurityChecker {
    static func isDeviceCompromised() -> Bool {
        // Check for jailbreak indicators
        let paths = [
            "/Applications/Cydia.app",
            "/Library/MobileSubstrate/MobileSubstrate.dylib",
            "/bin/bash",
            "/usr/sbin/sshd",
            "/etc/apt"
        ]
        
        for path in paths {
            if FileManager.default.fileExists(atPath: path) {
                return true
            }
        }
        
        // Check for suspicious URLs
        if let url = URL(string: "cydia://package/test"), 
           UIApplication.shared.canOpenURL(url) {
            return true
        }
        
        // Check build type
        #if DEBUG
        return false // Allow in debug
        #else
        return !isProductionBuild()
        #endif
    }
    
    static func isProductionBuild() -> Bool {
        #if DEBUG
        return false
        #else
        return true
        #endif
    }
}

// In SceneDelegate or App
class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        if DeviceSecurityChecker.isDeviceCompromised() {
            // Show warning or block app access
            fatalError("Compromised device detected")
        }
        return true
    }
}
```

---

## 5. Security Checklist

| Control | Implementation |
|---------|---------------|
| Biometric auth | `LAContext` with Face ID / Touch ID |
| Keychain storage | `SecItem` API with `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` |
| Certificate pinning | Alamofire `ServerTrustManager` with pinned certs |
| Device integrity | Jailbreak detection via file system checks |
| No sensitive data in logs | `os_log` with `.private` flag for sensitive data |
| Secure random | `SecRandomCopyBytes` for all crypto operations |
| ProGuard equivalent | Swift compiler optimizations + bitcode stripping |
| Session timeout | 30min idle for Admin/Finance, 60min for others |
| Auto-logout | On app background >5min (configurable) |
