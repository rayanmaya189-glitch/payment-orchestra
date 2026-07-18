# 07 — Settings

## 1. API Keys (`/settings/api-keys`)

### Table Columns

| Column | Description |
|--------|-------------|
| Name | Key name |
| Key Prefix | First 8 characters |
| Scopes | Permission badges |
| Status | Active / Expired / Revoked |
| Created At | Date |
| Expires At | Date |
| Actions | Rotate / Revoke |

### Create API Key Modal

```tsx
<CreateApiKeyDialog
  onSubmit={async (data) => {
    const response = await api.post('/v1/api-keys', {
      name: data.name,
      scopes: data.scopes,
      acquirer_link_ids: data.acquirerLinkIds,
      expires_in_days: data.expiresInDays,
    });
    // Show secret ONCE — never again
    showApiKeySecret(response.data.api_key_secret);
  }}
/>
```

### Scopes

| Scope | Description |
|-------|-------------|
| `payments:read` | Read payment intents, transactions |
| `payments:write` | Create, capture, void, refund payments |
| `invoices:read` | Read invoices |
| `invoices:write` | Create, send, cancel invoices |
| `subscriptions:read` | Read subscriptions |
| `subscriptions:write` | Create, cancel subscriptions |
| `reconciliation:read` | Read reconciliation data |
| `analytics:read` | Read analytics/reports |
| `assistant:query` | Query AI Assistant |
| `settings:read` | Read settings |
| `settings:write` | Modify settings (routing, connectors) |

---

## 2. User Management (`/settings/users`)

### Table Columns

| Column | Description |
|--------|-------------|
| Name | User name |
| Email | Email address |
| Role | Badge (Admin, Finance, Developer, Read-Only) |
| MFA Status | Enrolled / Not Enrolled |
| Last Login | Date |
| Status | Active / Suspended |
| Actions | Edit Role / Suspend / Delete |

### Role Management

```tsx
<RoleAssignmentDialog
  user={selectedUser}
  roles={['Admin', 'Finance Operator', 'Developer', 'Read-Only']}
  onUpdate={async (roleId) => {
    await api.patch(`/v1/principals/${selectedUser.id}`, {
      role: roleId,
    });
  }}
/>
```

---

## 3. Compliance (`/settings/compliance`)

### Sections

1. **KYB Status**: Current KYB status, documents uploaded, reviewer
2. **AML Alerts**: Open alerts, severity, review actions
3. **Audit Log**: Searchable audit trail with filters

### KYB Status Card

```tsx
<KybStatusCard
  status={operator.kybStatus}
  documents={operator.kybDocuments}
  onUpload={handleDocumentUpload}
  onResubmit={handleResubmit}
/>
```

### AML Alert Queue

```tsx
<AmlAlertQueue
  alerts={amlAlerts}
  onReview={async (alertId, decision) => {
    await api.post(`/v1/aml-alerts/${alertId}/review`, {
      decision: decision, // 'dismissed' | 'escalated'
      note: note,
    });
  }}
/>
```

### Audit Log Search

```tsx
<AuditLogSearch
  onSearch={async (filters) => {
    const response = await api.get('/v1/audit-log', { params: filters });
    return response.data;
  }}
  filters={[
    { name: 'actor', type: 'text' },
    { name: 'action', type: 'select', options: ['login', 'payment', 'config_change'] },
    { name: 'date_range', type: 'daterange' },
  ]}
/>
```

---

## 4. Notification Preferences

```tsx
<NotificationPreferences
  preferences={notificationPrefs}
  onUpdate={async (prefs) => {
    await api.patch('/v1/settings/notifications', prefs);
  }}
/>
```

### Notification Types

| Type | Default | Description |
|------|---------|-------------|
| Payment Failed | Email | When payment fails on all routes |
| Settlement Exception | Email | Unmatched settlement record detected |
| Chargeback Received | Email + SMS | New chargeback |
| Subscription Renewal Failed | Email | Customer subscription payment failed |
| API Key Expiring | Email | API key expiring in X days |
| AML Alert | Email | Suspicious activity detected |
