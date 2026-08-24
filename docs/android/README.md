# Android Development Documentation

## Overview

Native Android client for the AI-Native Payment Orchestration Platform. The Android app provides merchants with on-the-go access to payment operations, reconciliation, and AI assistant capabilities.

## Tech Stack

```
Language:       Kotlin 2.0+ (Coroutines + Flow)
UI Framework:   Jetpack Compose (Material 3)
Architecture:   MVVM + Clean Architecture
DI:             Hilt (Dagger)
Networking:     Retrofit + OkHttp + Moshi
Local Storage:  Room + DataStore (Encrypted)
Image Loading:  Coil
Navigation:     Compose Navigation
Testing:        JUnit 5 + Mockk + Turbine (Flow testing)
Min SDK:        26 (Android 8.0)
Target SDK:     35 (Android 15)
```

## File Index

| File | Description |
|------|-------------|
| `README.md` | This file |
| `00-architecture.md` | Tech stack, project structure, MVVM layers, dependency injection |
| `01-dashboard.md` | Main dashboard, stats cards, charts, real-time updates |
| `02-payment-management.md` | Transaction list, detail, create/capture/void/refund |
| `03-reconciliation.md` | Settlement matching, exception queue |
| `04-ai-assistant.md` | AI chat interface, voice input, citations |
| `05-invoice-subscription.md` | Invoice management, subscription lifecycle |
| `06-connector-config.md` | Gateway profile configuration, rotation strategy |
| `07-settings.md` | API keys, user management, compliance |
| `08-shared-components.md` | Design system, cards, tables, forms |
| `09-security.md` | Biometric auth, encrypted storage, certificate pinning |
| `10-offline-support.md` | Offline queue, sync, conflict resolution |

## Personas

| Persona | Role | Primary Features |
|---------|------|------------------|
| Fatima (Finance Ops) | Reconciliation, settlement | Dashboard, Reconciliation, AI Assistant |
| Rashid (Engineering) | API integration, webhooks | Settings (API Keys), Push Notifications |
| Omar (Compliance) | Audit, KYB, AML | Settings (Compliance), Audit Log |

## Design Principles

1. **Offline-first**: Critical operations (view transactions, check status) work offline
2. **Push notifications**: Real-time alerts for payments, settlements, AML
3. **Biometric security**: Fingerprint/Face ID for app access and sensitive operations
4. **RTL support**: Full Arabic RTL layout from day one
5. **Material 3**: Google's latest design system with dynamic color
6. **Offline-first**: Queue operations when offline, sync when online
