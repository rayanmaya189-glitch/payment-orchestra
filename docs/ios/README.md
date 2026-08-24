# iOS Development Documentation

## Overview

Native iOS client for the AI-Native Payment Orchestration Platform. The iOS app provides merchants with on-the-go access to payment operations, reconciliation, and AI assistant capabilities.

## Tech Stack

```
Language:       Swift 6.0+ (Strict Concurrency)
UI Framework:   SwiftUI (iOS 17+)
Architecture:   MVVM + Clean Architecture
DI:             Factory (or Swinject)
Networking:     Alamofire + Codable
Local Storage:  SwiftData + Keychain Services
Image Loading:  Kingfisher
Charts:         Swift Charts
Testing:        XCTest + Swift Testing + OHHTTPStubs
Min Target:     iOS 17.0
```

## File Index

| File | Description |
|------|-------------|
| `README.md` | This file |
| `00-architecture.md` | Tech stack, project structure, MVVM layers, dependency injection |
| `01-dashboard.md` | Main dashboard, stats cards, charts, real-time updates |
| `02-payment-management.md` | Transaction list, detail, create/capture/void/refund |
| `03-reconciliation.md` | Settlement matching, exception queue |
| `04-ai-assistant.md` | AI chat interface, speech recognition, citations |
| `05-invoice-subscription.md` | Invoice management, subscription lifecycle |
| `06-connector-config.md` | Gateway profile configuration, rotation strategy |
| `07-settings.md` | API keys, user management, compliance |
| `08-shared-components.md` | Design system, cards, tables, forms |
| `09-security.md` | Biometric auth, Keychain, certificate pinning |
| `10-offline-support.md` | SwiftData caching, sync queue, background tasks |

## Personas

| Persona | Role | Primary Features |
|---------|------|------------------|
| Fatima (Finance Ops) | Reconciliation, settlement | Dashboard, Reconciliation, AI Assistant |
| Rashid (Engineering) | API integration, webhooks | Settings (API Keys), Push Notifications |
| Omar (Compliance) | Audit, KYB, AML | Settings (Compliance), Audit Log |

## Design Principles

1. **Offline-first**: Critical operations work offline
2. **Push notifications**: Real-time alerts via APNs
3. **Biometric security**: Face ID / Touch ID for app access
4. **RTL support**: Full Arabic RTL layout from day one
5. **Native feel**: HIG-compliant, platform-native components
6. **Accessibility**: VoiceOver, Dynamic Type, Voice Control support
