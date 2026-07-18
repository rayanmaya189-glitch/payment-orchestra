# 02 — Payment Management

## 1. Create Payment Intent Modal

### Form Fields

| Field | Type | Validation | Required |
|-------|------|-----------|----------|
| Amount | NumberInput | > 0, max 1,000,000 | Yes |
| Currency | Select | AED, USD, EUR, SAR | Yes |
| Description | TextInput | max 255 chars | No |
| Purpose | RadioGroup | Payment / Card Verification | Yes |
| Payment Method Token | Select | From saved tokens | Yes (for Payment) |
| Metadata | KeyValueEditor | max 10 key-value pairs | No |

### API Call

```typescript
const createPaymentIntent = async (data: CreatePaymentIntentData) => {
  const response = await api.post('/v1/payment-intents', {
    amount: { amount_minor_units: data.amount * 100, currency_code: data.currency },
    idempotency_key: crypto.randomUUID(),
    purpose: data.purpose,
    metadata: data.metadata,
  });
  return response.data;
};
```

### 3DS Handling

```typescript
// When authorization returns Requires3DS
if (response.status === 'requires_3ds') {
  // Redirect to acquirer's 3DS page
  const threeDsUrl = response.three_ds_data.three_ds_url;
  window.location.href = threeDsUrl;
}
```

---

## 2. Capture Dialog

```tsx
<CaptureDialog
  paymentIntent={selectedPayment}
  onCapture={async (amount) => {
    await api.post(`/v1/payment-intents/${selectedPayment.id}/capture`, {
      amount: amount ? { amount_minor_units: amount * 100, currency_code: 'AED' } : null,
    });
    // null = full capture
  }}
/>
```

### Partial Capture

```typescript
// If connector supports partial capture
if (payment.supportsPartialCapture) {
  <CaptureDialog showAmountInput={true} maxAmount={payment.remainingAuthorized} />
} else {
  <CaptureDialog showAmountInput={false} /> // Full capture only
}
```

---

## 3. Refund Dialog

### Validation Rules

- Amount must be > 0
- Amount must be ≤ remaining refundable balance (captured - refunded)
- Cannot refund zero-amount authorizations
- Cannot refund if acquirer link is disabled

```tsx
<RefundDialog
  paymentIntent={selectedPayment}
  maxRefundable={selectedPayment.capturedAmount - selectedPayment.refundedAmount}
  onRefund={async (amount, reason) => {
    await api.post(`/v1/payment-intents/${selectedPayment.id}/refund`, {
      amount: { amount_minor_units: amount * 100, currency_code: 'AED' },
    });
  }}
/>
```

---

## 4. Void Confirmation

```tsx
<VoidConfirmDialog
  paymentIntent={selectedPayment}
  onConfirm={async () => {
    await api.post(`/v1/payment-intents/${selectedPayment.id}/void`);
  }}
/>
```

---

## 5. Payment Status Badges

```tsx
const statusConfig = {
  Created: { color: 'gray', label: 'Created' },
  Authorizing: { color: 'blue', label: 'Authorizing', pulse: true },
  Authorized: { color: 'green', label: 'Authorized' },
  Capturing: { color: 'blue', label: 'Capturing', pulse: true },
  Captured: { color: 'green', label: 'Captured' },
  Failed: { color: 'red', label: 'Failed' },
  FailedAllRoutes: { color: 'red', label: 'All Routes Failed' },
  Voided: { color: 'gray', label: 'Voided' },
  AuthorizationExpired: { color: 'yellow', label: 'Expired' },
  Refunded: { color: 'orange', label: 'Refunded' },
  PartiallyRefunded: { color: 'orange', label: 'Partially Refunded' },
};
```
