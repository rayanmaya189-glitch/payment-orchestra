# Payment Orchestra — Quickstart Guide

Get started with Payment Orchestra in 5 minutes.

## 1. Get Your API Key

```bash
# Sign up at https://app.payment-orchestra.com
# Or use the CLI:
curl -X POST https://api.payment-orchestra.com/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "dev@example.com",
    "password": "secure_password",
    "company_name": "My Company"
  }'
```

## 2. Create Your First Connector

```bash
# Connect Stripe as your first payment provider
curl -X POST https://api.payment-orchestra.com/v1/gateway-profiles \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "connector_id": "stripe",
    "display_name": "Stripe Production",
    "environment": "production",
    "credentials": {
      "api_key": "sk_live_...",
      "secret_key": "sk_live_..."
    }
  }'
```

## 3. Create a Routing Policy

```bash
# Route all payments through Stripe by default
curl -X POST https://api.payment-orchestra.com/v1/routing-policies \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Default Routing",
    "rotation_strategy": "priority",
    "rules": [
      {
        "gateway_profile_id": "YOUR_GATEWAY_ID",
        "priority": 1,
        "condition": {}
      }
    ],
    "failover_config": {
      "max_hops": 3,
      "latency_budget_ms": 10000
    }
  }'
```

## 4. Process Your First Payment

```bash
# Create a payment intent
curl -X POST https://api.payment-orchestra.com/v1/payment-intents \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: order_12345" \
  -d '{
    "amount_minor_units": 5000,
    "currency": "USD",
    "purpose": "payment"
  }'
```

## 5. Authorize the Payment

```bash
# Authorize with card token
curl -X POST https://api.payment-orchestra.com/v1/payment-intents/{id}/authorize \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "payment_method_token": "tok_visa_4242",
    "card_scheme": "visa"
  }'
```

## 6. Capture the Payment

```bash
# Capture authorized amount
curl -X POST https://api.payment-orchestra.com/v1/payment-intents/{id}/capture \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "amount_minor_units": 5000
  }'
```

## SDKs

### JavaScript / TypeScript

```bash
npm install @payment-orchestra/sdk
```

```typescript
import { PaymentOrchestra } from '@payment-orchestra/sdk';

const client = new PaymentOrchestra({
  apiKey: 'YOUR_API_KEY',
  environment: 'production' // or 'sandbox'
});

// Create a payment intent
const intent = await client.paymentIntents.create({
  amount: 5000,
  currency: 'USD',
  purpose: 'payment',
  idempotencyKey: 'order_12345'
});

// Authorize
const authorized = await client.paymentIntents.authorize(intent.id, {
  paymentMethodToken: 'tok_visa_4242',
  cardScheme: 'visa'
});

// Capture
const captured = await client.paymentIntents.capture(intent.id, {
  amount: 5000
});
```

### Python

```bash
pip install payment-orchestra
```

```python
from payment_orchestra import PaymentOrchestra

client = PaymentOrchestra(api_key="YOUR_API_KEY")

# Create a payment intent
intent = client.payment_intents.create(
    amount=5000,
    currency="USD",
    purpose="payment",
    idempotency_key="order_12345"
)

# Authorize
authorized = client.payment_intents.authorize(
    intent.id,
    payment_method_token="tok_visa_4242",
    card_scheme="visa"
)

# Capture
captured = client.payment_intents.capture(
    intent.id,
    amount=5000
)
```

## Webhooks

Set up webhooks to receive payment events:

```bash
curl -X POST https://api.payment-orchestra.com/v1/webhooks \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://your-app.com/webhooks/payment",
    "events": ["payment.authorized", "payment.captured", "payment.failed"]
  }'
```

### Verify Webhook Signatures

```javascript
const crypto = require('crypto');

function verifyWebhookSignature(payload, signature, secret) {
  const expected = crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  return `sha256=${expected}` === signature;
}
```

## Sandbox Mode

Use test API keys and test card numbers:

```bash
# Use sandbox environment
curl -X POST https://sandbox.payment-orchestra.com/v1/payment-intents \
  -H "Authorization: Bearer pk_test_..." \
  ...
```

### Test Card Numbers

| Card | Number | Result |
|------|--------|--------|
| Visa Success | 4242424242424242 | Authorized |
| Visa Declined | 4000000000000002 | Declined |
| Mastercard | 5555555555554444 | Authorized |
| 3DS Required | 4000000000003220 | Requires 3DS |

## Next Steps

- [API Reference](/docs/api-reference)
- [Routing Guide](/docs/routing)
- [Connector Setup](/docs/connectors)
- [Webhook Events](/docs/webhooks)
- [Rate Limits](/docs/rate-limits)
