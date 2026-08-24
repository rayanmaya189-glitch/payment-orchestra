# Frontend Development Documentation

## Overview

This directory contains implementation-ready frontend specifications for the AI-Native Payment Orchestration Platform. The frontend is a **reference client** — API-first design means every capability exists as a versioned REST/gRPC API; the dashboard is a consumer of those APIs, not the product boundary.

## Tech Stack

```
React 19 + React Router DOM 7 + Vite 8
TypeScript 5.x (strict) + Tailwind CSS 4
TanStack React Query v5 + Zustand v5
Axios v1.x + Zod + react-i18next
Recharts + TanStack Table v9 + Lucide React
```

## File Index

| File | Description |
|------|-------------|
| `README.md` | This file |
| `00-architecture.md` | Tech stack, project structure, routing, Axios config, Zustand stores, i18n |
| `01-dashboard.md` | Main dashboard pages, stats cards, charts, real-time updates |
| `02-payment-management.md` | Payment intents, capture, void, refund, 3DS handling |
| `03-reconciliation.md` | Settlement matching, exceptions, ledger view |
| `04-ai-assistant.md` | AI chat interface, citations, file upload, streaming |
| `05-invoice-subscription.md` | Invoices, payment links, subscriptions |
| `06-connector-config.md` | Acquirer connection setup, routing policy builder |
| `07-settings.md` | API keys, users, KYB, AML alerts, audit log |
| `08-hosted-checkout.md` | Public payment page (PCI-DSS isolated) |
| `09-shared-components.md` | Design system, tables, forms, modals |
| `10-security.md` | Auth flow, CSRF, session storage encryption, XSS prevention |
| `11-api-client.md` | Axios config, React Query hooks, error handling, WebSocket |

## Personas

| Persona | Role | Primary Pages |
|---------|------|---------------|
| Fatima (Finance Ops) | Reconciliation, settlement, monthly close | Dashboard, Reconciliation, Analytics |
| Rashid (Engineering) | API integration, webhooks, sandbox | Settings (API Keys), Hosted Checkout |
| Omar (Compliance) | Audit, KYB, AML monitoring | Settings (Compliance), AI Assistant |

## Design Principles

1. **API-first**: Every dashboard action calls a REST/gRPC API — no direct DB access
2. **Real-time updates**: WebSocket/SSE for payment status, reconciliation updates
3. **Progressive disclosure**: Complex data behind drill-downs, not overwhelming landing pages
4. **Mobile-responsive**: Dashboard works on tablet for finance operators on the go
5. **Accessibility**: WCAG 2.1 AA compliance for all interactive elements
6. **Security-first**: No sensitive data in URL params, CSP enforced, XSS prevention, encrypted session storage
7. **i18n**: Arabic + English support from day one (UAE market requirement)

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| No SSR | Vite SPA | Admin dashboard doesn't need SEO; SPA is simpler and faster to develop |
| Session Storage | Encrypted | Survives page refresh, cleared on tab close, lower XSS blast radius than localStorage |
| Axios over fetch | Interceptors | Better error handling, automatic token refresh, request/response transformation |
| Zustand over Redux | Simplicity | Less boilerplate for UI state; React Query handles server state |
| Zod over Yup | Performance | Zod is faster, tree-shakeable, and has better TypeScript inference |
| react-i18next | RTL support | Mature Arabic RTL support, lazy loading of translation bundles |
