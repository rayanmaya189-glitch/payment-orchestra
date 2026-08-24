# 06 — Connector Configuration & Gateway Profiles

## 1. Connector List (`/connectors`)

### Table Columns

| Column | Description |
|--------|-------------|
| Connector ID | Identifier |
| Name | Display name |
| Status | Connected/Untested, Active, Disabled |
| Card Schemes | Visa, Mastercard, Amex |
| Settlement Format | Webhook, Polling, SFTP |
| Daily Volume | Used / Limit |
| Success Rate | Percentage |
| Connected At | Date |

### Status Badges

```tsx
const connectorStatusConfig = {
  connected_untested: { color: 'yellow', label: 'Connected (Untested)' },
  active: { color: 'green', label: 'Active' },
  disabled: { color: 'red', label: 'Disabled' },
  maintenance: { color: 'orange', label: 'Maintenance' },
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
// - Default fee structure
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
      environment: 'sandbox',
    });
    return response.data;
  }}
/>
```

### Step 3: Set Gateway Profile

```tsx
<GatewayProfileForm
  connectorId={selectedConnector.id}
  defaults={selectedConnector.defaultProfile}
  onSubmit={async (profile) => {
    await api.post('/v1/gateway-profiles', {
      connector_id: selectedConnector.id,
      merchant_acquirer_link_id: link.id,
      ...profile,
    });
  }}
/>
```

### Step 4: Test Connection

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
2. **Gateway Profile**: Transaction limits, fee structure, rate limits
3. **Capabilities**: Supported features (partial capture, idempotency, etc.)
4. **Circuit Breaker**: Current state, error rate, last failure
5. **Settlement**: Settlement format, last settlement received
6. **Actions**: Test Connection, Disable, Reconfigure, Edit Profile

### Gateway Profile Display

```tsx
<GatewayProfileCard profile={gatewayProfile}>
  <ProfileSection title="Transaction Limits">
    <LimitRow label="Min Amount" value={formatMoney(profile.minAmount)} />
    <LimitRow label="Max Amount" value={formatMoney(profile.maxAmount)} />
    <LimitRow label="Daily Volume Limit" value={formatMoney(profile.dailyVolumeLimit)} />
    <LimitRow label="Monthly Volume Limit" value={formatMoney(profile.monthlyVolumeLimit)} />
    <LimitRow label="Max Refund Amount" value={formatMoney(profile.maxRefundAmount)} />
  </ProfileSection>

  <ProfileSection title="Fee Structure">
    <FeeRow label="Fixed Fee" value={formatMoney(profile.fixedFee)} />
    <FeeRow label="Percentage Fee" value={`${profile.percentageFeeBps / 100}%`} />
    <FeeRow label="Cross-Border Fee" value={`${profile.crossBorderFeeBps / 100}%`} />
    <FeeRow label="FX Conversion Fee" value={`${profile.currencyConversionFeeBps / 100}%`} />
  </ProfileSection>

  <ProfileSection title="Rate Limits">
    <LimitRow label="Per Second" value={profile.rateLimitPerSecond} />
    <LimitRow label="Per Day" value={profile.rateLimitPerDay.toLocaleString()} />
  </ProfileSection>

  <ProfileSection title="Monitoring">
    <ThresholdRow label="Success Rate Alert" value={`${profile.successRateThreshold * 100}%`} />
    <ThresholdRow label="Latency Alert" value={`${profile.latencyThresholdMs}ms`} />
    <ToggleRow label="Auto-Disable on Low Success" value={profile.autoDisableOnLowSuccess} />
  </ProfileSection>
</GatewayProfileCard>
```

### Volume Usage Display

```tsx
<VolumeUsageBar
  dailyUsed={profile.dailyVolumeUsed}
  dailyLimit={profile.dailyVolumeLimit}
  monthlyUsed={profile.monthlyVolumeUsed}
  monthlyLimit={profile.monthlyVolumeLimit}
/>
```

---

## 4. Gateway Profile Edit Modal

```tsx
<GatewayProfileEditModal
  profile={gatewayProfile}
  onSave={async (updates) => {
    await api.patch(`/v1/gateway-profiles/${gatewayProfile.id}`, updates);
  }}
/>
```

### Form Fields

| Section | Field | Type | Description |
|---------|-------|------|-------------|
| **Limits** | Min Transaction Amount | CurrencyInput | Minimum per transaction |
| | Max Transaction Amount | CurrencyInput | Maximum per transaction |
| | Daily Volume Limit | CurrencyInput | Max daily aggregate |
| | Monthly Volume Limit | CurrencyInput | Max monthly aggregate |
| | Max Refund Amount | CurrencyInput | Per-transaction refund cap |
| **Fees** | Fixed Fee | CurrencyInput | Per-transaction fixed fee |
| | Percentage Fee | NumberInput (bps) | Basis points (250 = 2.50%) |
| | Cross-Border Fee | NumberInput (bps) | Additional for cross-border |
| | FX Conversion Fee | NumberInput (bps) | Additional for currency conversion |
| **Card Schemes** | Enabled Schemes | CheckboxGroup | Visa, Mastercard, Amex, Mada |
| **Currencies** | Enabled Currencies | CheckboxGroup | AED, USD, EUR, SAR |
| **Rate Limits** | Per Second | NumberInput | Max API calls/sec |
| | Per Day | NumberInput | Max API calls/day |
| **Monitoring** | Success Rate Alert | NumberInput (%) | Alert threshold |
| | Latency Alert | NumberInput (ms) | P99 latency threshold |
| | Auto-Disable | Toggle | Disable if success rate drops |
| **Status** | Status | Select | Active / Disabled / Maintenance |

---

## 5. Fee Calculation Preview

```tsx
<FeeCalculationPreview
  profile={gatewayProfile}
  amount={transactionAmount}
  isCrossBorder={isCrossBorder}
  requiresFx={requiresFx}
/>

// Shows:
// - Base amount: 1,000.00 AED
// - Fixed fee: 1.00 AED
// - Percentage fee (2.50%): 25.00 AED
// - Cross-border fee (1.00%): 10.00 AED
// - Total fee: 36.00 AED
// - Net to merchant: 964.00 AED
```

---

## 6. Gateway Profile Comparison

```tsx
<GatewayProfileComparison
  profiles={allProfiles}
  onEdit={(profileId) => openEditModal(profileId)}
/>

// Side-by-side comparison table:
// | Metric | Network International | Checkout.com | Telr |
// |--------|----------------------|--------------|------|
// | Fixed Fee | 1.00 AED | 0.50 AED | 1.50 AED |
// | Percentage | 2.50% | 2.25% | 2.75% |
// | Max Amount | 500,000 | 1,000,000 | 250,000 |
// | Daily Volume | 50M | 100M | 25M |
// | Rate Limit/s | 100 | 200 | 50 |
// | Success Rate | 98.5% | 99.2% | 97.8% |
```

---

## 7. Gateway Rotation Strategy (`/settings/routing/rotation`)

### Rotation Strategy Selector

```tsx
<RotationStrategySelector
  currentStrategy={policy.rotationStrategy}
  onChange={async (strategy) => {
    await api.patch('/v1/routing-policies/rotation', {
      strategy: strategy.type,
      config: strategy.config,
    });
  }}
/>
```

### Strategy Options

| Strategy | Description | When to Use |
|----------|-------------|-------------|
| **Priority** | Fixed order — always try gateway 1 first | Default, simple |
| **Round Robin** | Distribute evenly across gateways | Load balancing |
| **Weighted Round Robin** | Distribute by weight (e.g., 60%/40%) | Cost optimization |
| **Cost Based** | Select cheapest gateway per transaction | Fee minimization |
| **Success Rate Based** | Select highest success rate gateway | Reliability optimization |
| **Volume Capped** | Rotate until one hits daily limit, then next | Volume management |

### Weighted Round Robin Configuration

```tsx
<WeightedRoundRobinConfig
  gateways={activeProfiles}
  weights={rotationConfig.weights}
  onUpdate={async (weights) => {
    await api.patch('/v1/routing-policies/rotation', {
      strategy: 'weighted_round_robin',
      weights: weights, // [{ gateway_id: 'xxx', weight: 60 }, { gateway_id: 'yyy', weight: 40 }]
    });
  }}
/>

// Visual weight display
<WeightDisplay>
  {gateways.map(gw => (
    <WeightBar
      key={gw.id}
      name={gw.name}
      weight={gw.weight}
      percentage={calculatePercentage(gw.weight, totalWeight)}
      color={gw.color}
    />
  ))}
</WeightDisplay>
```

### Rotation Status Dashboard

```tsx
<RotationStatusDashboard>
  <CurrentRotation
    strategy={policy.rotationStrategy}
    lastUsed={rotationState.lastUsedGateway}
    nextInLine={rotationState.nextGateway}
    dailyVolume={rotationState.dailyVolume}
  />

  <VolumeByGatewayChart data={rotationState.dailyVolumeByGateway} />

  <RotationHistoryTable
    history={rotationHistory}
    columns={['Time', 'Payment ID', 'Gateway', 'Strategy Used', 'Amount']}
  />
</RotationStatusDashboard>
```

---

## 8. Order-Gateway Profile Link (`/payments/[id]`)

### Gateway Profile Section in Transaction Detail

```tsx
<TransactionDetail>
  <GatewayProfileSection>
    <ProfileLink
      gatewayId={payment.gatewayProfileId}
      connectorName={payment.connectorName}
      onClick={() => navigateTo(`/connectors/${payment.gatewayProfileId}`)}
    />

    <ProfileSnapshot
      // Snapshot of profile at time of transaction (immutable)
      fees={payment.gatewayProfileSnapshot.fees}
      limits={payment.gatewayProfileSnapshot.limits}
      capturedAt={payment.createdAt}
    />

    <FeeBreakdown
      fixedFee={payment.fees.fixedFee}
      percentageFee={payment.fees.percentageFee}
      crossBorderFee={payment.fees.crossBorderFee}
      totalFee={payment.fees.totalFee}
      netAmount={payment.amount - payment.fees.totalFee}
    />
  </GatewayProfileSection>
</TransactionDetail>
```

### Multi-Hop Gateway Tracking

```tsx
// For failover transactions, show each gateway used
<RoutingTimeline attempts={payment.attempts}>
  {payment.attempts.map((attempt, i) => (
    <RoutingAttempt
      key={i}
      gatewayProfileId={attempt.gatewayProfileId}
      connectorName={attempt.connectorName}
      gatewaySnapshot={attempt.gatewaySnapshot} // snapshot at time of attempt
      status={attempt.approved ? 'success' : 'failed'}
      declineReason={attempt.declineReason}
      fee={attempt.fee}
      latencyMs={attempt.latencyMs}
    />
  ))}
</RoutingTimeline>
```

---

## 9. Gateway Profile Analytics (`/analytics/gateways`)

### Per-Gateway Metrics

```tsx
<GatewayAnalyticsDashboard>
  <GatewayMetricsGrid>
    {profiles.map(profile => (
      <GatewayMetricCard
        key={profile.id}
        name={profile.connectorName}
        metrics={{
          totalTransactions: profile.totalTransactions,
          successRate: profile.successRate,
          avgLatency: profile.avgLatency,
          totalVolume: profile.totalVolume,
          totalFees: profile.totalFees,
          dailyVolumeUsed: profile.dailyVolumeUsed,
          dailyVolumeLimit: profile.dailyVolumeLimit,
        }}
      />
    ))}
  </GatewayMetricsGrid>

  <FeeComparisonChart
    profiles={profiles}
    metric="totalFees"
    timeRange={selectedTimeRange}
  />

  <SuccessRateComparisonChart
    profiles={profiles}
    timeRange={selectedTimeRange}
  />

  <VolumeDistributionChart
    profiles={profiles}
    timeRange={selectedTimeRange}
  />
</GatewayAnalyticsDashboard>
```

### Fee Analysis Table

| Gateway | Transactions | Volume | Fees Paid | Avg Fee % | Net to Merchant |
|---------|-------------|--------|-----------|-----------|-----------------|
| Network International | 1,234 | 5.2M AED | 130,000 AED | 2.50% | 5,070,000 AED |
| Checkout.com | 890 | 3.8M AED | 85,500 AED | 2.25% | 3,714,500 AED |
| Telr | 456 | 1.2M AED | 33,000 AED | 2.75% | 1,167,000 AED |

### Gateway Performance Comparison

```tsx
<PerformanceComparison
  profiles={profiles}
  metrics={['successRate', 'avgLatency', 'errorRate', 'dailyVolume']}
  timeRange={selectedTimeRange}
/>

// Radar chart comparing gateways across all metrics
```

---

## 10. Gateway Profile Bulk Operations

### Bulk Limit Update

```tsx
<BulkLimitUpdateDialog
  profiles={selectedProfiles}
  onUpdate={async (limits) => {
    await api.post('/v1/gateway-profiles/bulk-update', {
      profile_ids: selectedProfiles.map(p => p.id),
      limits: limits,
    });
  }}
/>
```

### Bulk Enable/Disable

```tsx
<BulkStatusChangeDialog
  profiles={selectedProfiles}
  newStatus="disabled"
  onConfirm={async () => {
    await api.post('/v1/gateway-profiles/bulk-status', {
      profile_ids: selectedProfiles.map(p => p.id),
      status: 'disabled',
    });
  }}
/>
```

---

## 11. Routing Policy Configuration (`/settings/routing`)

### Visual Rule Builder

```tsx
<RoutingPolicyBuilder
  rules={policy.rules}
  acquirers={activeAcquirers}
  profiles={gatewayProfiles}
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
| Amount Min | CurrencyInput | Minimum transaction amount |
| Amount Max | CurrencyInput | Maximum transaction amount |
| Target Gateway | Select | Active gateway profile |
| Failover To | Select | Next gateway in chain |

### Gateway-Aware Routing

```tsx
// Rules reference gateway profiles, not just acquirer links
<RoutingRuleEditor
  rule={rule}
  availableGateways={activeProfiles.map(p => ({
    id: p.id,
    name: p.connectorName,
    limits: p.limits,
    fees: p.fees,
    successRate: p.successRate,
  }))}
/>
```

### Visual Routing Diagram with Gateway Limits

```tsx
<RoutingDiagram rules={policy.rules} profiles={gatewayProfiles}>
  {policy.rules.map((rule, i) => (
    <RoutingNode
      key={i}
      priority={rule.priority}
      condition={rule.condition}
      gateway={rule.gatewayProfile}
      limits={rule.gatewayProfile.limits}
      fees={rule.gatewayProfile.fees}
      isLast={i === policy.rules.length - 1}
    />
  ))}
</RoutingDiagram>
```

---

## 8. Gateway Health Dashboard

```tsx
<GatewayHealthDashboard profiles={gatewayProfiles}>
  {profiles.map(profile => (
    <GatewayHealthCard
      key={profile.id}
      profile={profile}
      metrics={{
        successRate: profile.successRate,
        avgLatency: profile.avgLatency,
        dailyVolume: profile.dailyVolumeUsed,
        errorRate: profile.errorRate,
        circuitBreakerState: profile.circuitBreakerState,
      }}
    />
  ))}
</GatewayHealthDashboard>
```

### Health Metrics

| Metric | Green | Yellow | Red |
|--------|-------|--------|-----|
| Success Rate | > 99% | 95-99% | < 95% |
| Avg Latency | < 2s | 2-5s | > 5s |
| Daily Volume | < 80% limit | 80-95% limit | > 95% limit |
| Error Rate | < 1% | 1-5% | > 5% |
| Circuit Breaker | Closed | Half-Open | Open |
