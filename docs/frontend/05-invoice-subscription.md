# 05 — Invoice & Subscription

## 1. Invoice List (`/invoices`)

### Table Columns

| Column | Sortable | Format |
|--------|----------|--------|
| Invoice ID | Yes | Truncated UUID |
| Order Reference | Yes | String |
| Status | Yes | Badge (draft/sent/paid/overdue/cancelled) |
| Total Amount | Yes | Currency formatted |
| Paid Amount | Yes | Currency formatted |
| Due Date | Yes | Date formatted |
| Payment Status | No | Progress bar |
| Actions | No | View / Send / Cancel |

### Create Invoice Modal

```tsx
<CreateInvoiceDialog
  onSubmit={async (data) => {
    const response = await api.post('/v1/invoices', {
      order_reference: data.orderReference,
      line_items: data.lineItems.map(item => ({
        description: item.description,
        amount_minor_units: item.amount * 100,
      })),
      due_date: data.dueDate.toISOString(),
      recipient_email: data.recipientEmail,
    });
    return response.data;
  }}
/>
```

---

## 2. Payment Link Management

### Create Payment Link

```tsx
<CreatePaymentLinkDialog
  onSubmit={async (data) => {
    const response = await api.post('/v1/payment-links', {
      amount: { amount_minor_units: data.amount * 100, currency_code: data.currency },
      description: data.description,
      expires_in_days: data.expiresInDays,
    });
    // Return shareable URL
    return `${CHECKOUT_URL}/pay/${response.data.token}`;
  }}
/>
```

### Payment Link Table

| Column | Description |
|--------|-------------|
| Token | Truncated token |
| Status | Active / Expired / Used |
| Amount | Currency formatted |
| Description | Text |
| Expires At | Date |
| Actions | Copy Link / View / Deactivate |

---

## 3. Subscription Management (`/subscriptions`)

### Table Columns

| Column | Sortable | Format |
|--------|----------|--------|
| Subscription ID | Yes | Truncated UUID |
| Customer | Yes | Customer name/email |
| Plan | Yes | Plan name |
| Status | Yes | Badge (active/past_due/cancelled/paused) |
| Amount | Yes | Currency + interval |
| Current Period End | Yes | Date |
| Dunning Retries | No | Count |
| Actions | No | Pause / Resume / Cancel |

### Subscription Detail

- **Plan Info**: Price, interval, trial period
- **Billing History**: List of renewal PaymentIntents with status
- **Payment Method**: Last four, brand, expiry
- **Dunning Status**: Retry count, next retry date

---

## 4. Hosted Checkout Page (`/pay/[token]`)

### PCI-DSS Isolation

- **Separate domain**: `pay.platform.ae` (not `admin.platform.ae`)
- **No shared cookies**: Different cookie domain
- **No shared localStorage**: Isolated storage
- **Separate CSP**: Payment-page-specific Content Security Policy
- **No admin context**: Minimal checkout UI only

### Checkout Flow

```
1. Load page → validate token → show amount + description
2. Customer enters card details (via acquirer's tokenization SDK)
3. Submit → POST /v1/payment-intents + AuthorizePaymentIntent
4. If 3DS required → redirect to acquirer's 3DS page
5. On completion → redirect to success/cancel URL
```

### Checkout Page Component

```tsx
<CheckoutPage
  amount={link.amount}
  currency={link.currency}
  description={link.description}
  onPaymentComplete={async (paymentMethodToken) => {
    const intent = await api.post('/v1/payment-intents', {
      amount: link.amount,
      idempotency_key: crypto.randomUUID(),
      purpose: 'payment',
    });
    const auth = await api.post(`/v1/payment-intents/${intent.id}/authorize`, {
      payment_method_token: paymentMethodToken,
    });
    if (auth.status === 'Authorized') {
      window.location.href = successUrl;
    } else if (auth.status === 'requires_3ds') {
      // Handle 3DS redirect
    } else {
      // Show error
    }
  }}
/>
```
