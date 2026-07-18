# 06 — Connector Configuration

## 1. Connector List (`/connectors`)

### Table Columns

| Column | Description |
|--------|-------------|
| Connector ID | Identifier |
| Name | Display name |
| Status | Connected/Untested, Active, Disabled |
| Card Schemes | Visa, Mastercard, Amex |
| Settlement Format | Webhook, Polling, SFTP |
| Connected At | Date |

### Status Badges

```tsx
const connectorStatusConfig = {
  connected_untested: { color: 'yellow', label: 'Connected (Untested)' },
  active: { color: 'green', label: 'Active' },
  disabled: { color: 'red', label: 'Disabled' },
};
```

---

## 2. Connect New Acquirer (`/connectors/new`)

### Step 1: Select Provider

```tsx
<ConnectorCatalog
  connectors={availableConnectors}
  onSelect={(connectorId) => setSelectedConnector(connectorId)}
/>

// Connector catalog shows:
// - Provider logo/name
// - Supported card schemes
// - Settlement format
// - Description
```

### Step 2: Configure Credentials

```tsx
// Dynamic form based on connector's OnboardingSchema
<DynamicForm
  schema={selectedConnector.onboardingSchema}
  onSubmit={async (credentials) => {
    const response = await api.post('/v1/connectors', {
      connector_id: selectedConnector.id,
      credentials: credentials,
      environment: 'sandbox', // Start with sandbox
    });
    return response.data;
  }}
/>
```

### Step 3: Test Connection

```tsx
<TestConnectionDialog
  connectorId={connector.id}
  onTest={async () => {
    const response = await api.post(`/v1/connectors/${connector.id}/test`);
    return response.data;
  }}
  onSuccess={() => navigateTo(`/connectors/${connector.id}`)}
/>
```

---

## 3. Connector Detail (`/connectors/[id]`)

### Sections

1. **Status Overview**: Connection status, last test result, health indicators
2. **Configuration**: Masked credentials (last 4 chars only), environment
3. **Capabilities**: Supported features (partial capture, idempotency, etc.)
4. **Circuit Breaker**: Current state, error rate, last failure
5. **Settlement**: Settlement format, last settlement received
6. **Actions**: Test Connection, Disable, Reconfigure

### Credential Display

```tsx
<CredentialDisplay>
  {credentials.map(cred => (
    <CredentialRow
      key={cred.name}
      name={cred.label}
      value={cred.maskedValue} // e.g., "sk_test_...xxxx"
      lastModified={cred.lastModified}
      onRotate={() => openRotateDialog(cred)}
    />
  ))}
</CredentialDisplay>
```

---

## 4. Routing Policy Configuration (`/settings/routing`)

### Visual Rule Builder

```tsx
<RoutingPolicyBuilder
  rules={policy.rules}
  acquirers={activeAcquirers}
  onUpdate={async (rules) => {
    await api.post('/v1/routing-policies', {
      rules: rules,
      failover_config: {
        retryable_decline_codes: ['InsufficientFunds', 'IssuerUnavailable'],
        max_hops: 3,
        latency_budget_ms: 10000,
      },
    });
  }}
/>
```

### Rule Editor

| Field | Type | Description |
|-------|------|-------------|
| Priority | Number | 1 = highest |
| Card Scheme | Multi-select | Visa, Mastercard, Amex |
| Currency | Multi-select | AED, USD, EUR |
| Amount Min | Number | Minimum transaction amount |
| Amount Max | Number | Maximum transaction amount |
| Target Acquirer | Select | Active acquirer link |
| Failover To | Select | Next acquirer in chain |

### Visual Routing Diagram

```tsx
<RoutingDiagram rules={policy.rules}>
  {policy.rules.map((rule, i) => (
    <RoutingNode
      key={i}
      priority={rule.priority}
      condition={rule.condition}
      target={rule.acquirerName}
      isLast={i === policy.rules.length - 1}
    />
  ))}
</RoutingDiagram>
```
