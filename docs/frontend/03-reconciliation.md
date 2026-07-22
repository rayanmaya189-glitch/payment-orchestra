# 03 — Reconciliation

## 1. Reconciliation Dashboard (`/reconciliation`)

### Stats Cards

| Card | Data Source |
|------|-------------|
| Total Matched | `POST /v1/analytics/settlement-status/search` |
| Total Unmatched | `POST /v1/analytics/settlement-status/search` |
| Total Amount Matched | Calculated from matched records |
| Match Rate | Calculated percentage |
| Exceptions Pending Review | `POST /v1/reconciliation/exceptions/search` |

### Charts

- **Match Rate Trend**: Line chart showing daily match rate over 30 days
- **Settlement by Acquirer**: Stacked bar chart showing matched/unmatched per acquirer
- **Exception Types**: Pie chart showing breakdown (no_reference, amount_mismatch, duplicate, orphan)

---

## 2. Exception Queue (`/reconciliation/exceptions`)

### Filters

| Filter | Type | Options |
|--------|------|---------|
| Classification | Multi-select | No Reference Match, Amount Mismatch, Duplicate Reference, Orphan Record |
| Status | Select | Unmatched, Resolved, Flagged Discrepancy |
| Acquirer | Select | All connected acquirers |
| Date Range | DateRangePicker | Custom range |
| Amount Range | Min/Max | AED |

### Table Columns

| Column | Description |
|--------|-------------|
| Exception ID | Truncated UUID |
| Classification | Badge with color |
| Acquirer Reference | From settlement record |
| Amount | Settlement amount vs. expected amount |
| Status | Unmatched / Resolved / Flagged |
| Detected At | Relative time |
| Actions | Match / Flag / AI Suggest |

### Action: AI-Assisted Matching

```typescript
const suggestMatch = async (exceptionId: string) => {
  const response = await api.post('/v1/assistant/query', {
    question: `Suggest a match for settlement exception ${exceptionId}: amount ${exception.amount} ${exception.currency} on ${exception.detectedAt}`,
  });
  // Returns AI suggestion with cited sources
  return response.data;
};
```

### Action: Manual Matching

```tsx
<MatchDialog
  exception={selectedException}
  onMatch={async (paymentIntentId) => {
    await api.post(`/v1/reconciliation/exceptions/${selectedException.id}/resolve`, {
      resolution: 'linked',
      payment_intent_id: paymentIntentId,
    });
  }}
/>
```

---

## 3. Settlement Batch View

### Batch List

| Column | Description |
|--------|-------------|
| Batch ID | Truncated UUID |
| Acquirer | Name |
| File Format | Webhook / SFTP / API |
| Status | Processing / Processed / Quarantined |
| Records | Total / Matched / Unmatched |
| Ingested At | Timestamp |

### Batch Detail

- **File Info**: Checksum, format, size
- **Record Summary**: Matched, unmatched, amount mismatch counts
- **Record List**: Expandable table of individual settlement records
- **Unmatched Records**: Highlighted, with action buttons

---

## 4. Ledger View

### Ledger Balance Table

| Column | Description |
|--------|-------------|
| Acquirer | Name |
| Currency | AED, USD, etc. |
| Date | Balance date |
| Net Balance | Credit - Debit |
| Entry Count | Total entries |
| Unreconciled Count | Entries pending reconciliation |

### Balance Verification

```typescript
// Trigger ledger balance verification
const verifyLedger = async () => {
  await api.post('/v1/reconciliation/ledger/verify');
  // Runs LEDGER-VERIFY-001 background job
};
```
