# 01 — Dashboard Pages

## 1. Main Dashboard (`/dashboard`)

### Overview Cards

| Card | Data Source | Refresh |
|------|-------------|---------|
| Total Transactions (today) | `POST /v1/analytics/authorization-rates/search` | 30s |
| Authorization Rate | `POST /v1/analytics/authorization-rates/search` | 30s |
| Revenue Recovered (failover) | `POST /v1/analytics/revenue-recovery/search` | 60s |
| Pending Settlements | `POST /v1/analytics/settlement-status/search` | 60s |
| Open Exceptions | `POST /v1/reconciliation/exceptions/search` | 30s |
| Active Subscriptions | `POST /v1/subscriptions/search` | 5min |

### Components

```tsx
// Main dashboard layout
<DashboardLayout>
  <DashboardHeader>
    <DateRangePicker onChange={handleDateRange} />
    <ExportButton onClick={handleExport} />
  </DashboardHeader>

  <StatsGrid>
    <StatCard title="Transactions Today" value={stats.totalTx} trend={stats.txTrend} />
    <StatCard title="Auth Rate" value={stats.authRate} suffix="%" trend={stats.authTrend} />
    <StatCard title="Revenue Recovered" value={stats.recoveredAmount} currency="AED" />
    <StatCard title="Pending Settlements" value={stats.pendingSettlements} />
  </StatsGrid>

  <ChartsGrid>
    <AuthRateChart data={hourlyRates} />           // Line chart: auth rate over time
    <DeclineReasonPie data={declineBreakdown} />    // Pie chart: decline reasons
    <SettlementProgress data={settlementStatus} />  // Stacked bar: matched/unmatched
    <RevenueRecoveryBar data={recoveryByAcquirer} /> // Bar chart: recovery by acquirer
  </ChartsGrid>

  <RecentActivityTable data={recentTransactions} /> // Table: last 10 transactions
</DashboardLayout>
```

### Real-time Updates

```typescript
// WebSocket connection for live payment status
const usePaymentUpdates = () => {
  const queryClient = useQueryClient();

  useEffect(() => {
    const ws = new WebSocket(process.env.NEXT_PUBLIC_WS_URL);
    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === 'payment_status_changed') {
        queryClient.invalidateQueries(['dashboard-stats']);
        queryClient.invalidateQueries(['recent-transactions']);
      }
    };
    return () => ws.close();
  }, []);
};
```

---

## 2. Payments List (`/payments`)

### Filters

| Filter | Type | Options |
|--------|------|---------|
| Status | Multi-select | Created, Authorizing, Authorized, Captured, Failed, Refunded |
| Date Range | DateRangePicker | Last 24h, 7d, 30d, Custom |
| Acquirer | Select | All, Network International, Checkout.com, Telr |
| Card Scheme | Select | All, Visa, Mastercard, Amex |
| Amount Range | Min/Max inputs | AED |

### Table Columns

| Column | Sortable | Format |
|--------|----------|--------|
| Transaction ID | Yes | Truncated UUID |
| Status | Yes | Badge (color-coded) |
| Amount | Yes | Currency formatted |
| Acquirer | Yes | Name |
| Card Scheme | No | Icon + label |
| Decline Reason | No | Text (if failed) |
| Date | Yes | Relative time |
| Actions | No | View / Void / Refund |

### Row Actions

```typescript
// Status-dependent actions
const getActions = (payment: PaymentIntent) => {
  switch (payment.status) {
    case 'Authorized':
      return [{ label: 'Capture', action: 'capture' }, { label: 'Void', action: 'void' }];
    case 'Captured':
      return [{ label: 'Refund', action: 'refund' }, { label: 'View', action: 'view' }];
    case 'Failed':
      return [{ label: 'View Details', action: 'view' }];
    default:
      return [{ label: 'View', action: 'view' }];
  }
};
```

---

## 3. Transaction Detail (`/payments/[id]`)

### Sections

1. **Header**: Payment Intent ID, Status badge, Created/Updated timestamps
2. **Amount Summary**: Requested, Authorized, Captured, Refunded amounts with visual progress bar
3. **Routing Timeline**: Visual timeline showing each acquirer hop with status, latency, decline reason
4. **Event Timeline**: Chronological list of all domain events for this payment
5. **Metadata**: Custom metadata key-value pairs
6. **Actions**: Capture, Void, Refund (status-dependent)

### Routing Timeline Component

```tsx
<RoutingTimeline attempts={payment.attempts}>
  {payment.attempts.map((attempt, i) => (
    <RoutingAttempt
      key={i}
      acquirer={attempt.acquirerName}
      status={attempt.approved ? 'success' : 'failed'}
      declineReason={attempt.declineReason}
      latencyMs={attempt.latencyMs}
      timestamp={attempt.timestamp}
    />
  ))}
</RoutingTimeline>
```
