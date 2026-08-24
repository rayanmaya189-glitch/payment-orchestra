# Payment Orchestration Platform - Developer Portal

Welcome to the Payment Orchestration Platform developer documentation. This portal provides everything you need to integrate with our platform.

## Quick Start

### 1. Get Your API Keys

```bash
# Sign up at https://dashboard.paymentorchestra.com
# Get your API keys from Settings > API Keys

# Sandbox Environment
API_KEY=pk_test_your_sandbox_key
SECRET_KEY=sk_test_your_sandbox_secret

# Production Environment
API_KEY=pk_live_your_production_key
SECRET_KEY=sk_live_your_production_secret
```

### 2. Install SDK

```bash
# Node.js
npm install @payment-orchestra/sdk

# Python
pip install payment-orchestra

# Go
go get github.com/payment-orchestra/sdk-go
```

### 3. Make Your First Payment

```javascript
const { PaymentOrchestra } = require('@payment-orchestra/sdk');

const client = new PaymentOrchestra({
  apiKey: 'pk_test_your_key',
  secretKey: 'sk_test_your_secret',
  environment: 'sandbox'
});

// Create a payment intent
const paymentIntent = await client.paymentIntents.create({
  amount: 10000, // $100.00
  currency: 'USD',
  orderId: 'order_12345',
  metadata: {
    customerId: 'cust_123'
  }
});

console.log('Payment Intent:', paymentIntent.id);
```

## API Reference

### Base URLs

| Environment | URL |
|-------------|-----|
| Sandbox | `https://sandbox.api.paymentorchestra.com` |
| Production | `https://api.paymentorchestra.com` |

### Authentication

All API requests require authentication via API keys:

```bash
curl -X POST https://api.paymentorchestra.com/v1/payment_intents \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer pk_live_your_key" \
  -d '{
    "amount": 10000,
    "currency": "USD"
  }'
```

### Core Resources

#### Payment Intents

Create and manage payment intents for processing transactions.

**Create Payment Intent**
```http
POST /v1/payment_intents
```

```json
{
  "amount": 10000,
  "currency": "USD",
  "order_id": "order_12345",
  "customer_id": "cust_123",
  "payment_method": "pm_card_visa",
  "capture_method": "automatic",
  "metadata": {
    "description": "Premium subscription"
  }
}
```

**Response:**
```json
{
  "id": "pi_abc123",
  "object": "payment_intent",
  "amount": 10000,
  "currency": "usd",
  "status": "requires_capture",
  "client_secret": "pi_abc123_secret_xyz",
  "created": 1690000000
}
```

**Authorize Payment Intent**
```http
POST /v1/payment_intents/{id}/authorize
```

**Capture Payment Intent**
```http
POST /v1/payment_intents/{id}/capture
```

**Cancel Payment Intent**
```http
POST /v1/payment_intents/{id}/cancel
```

**Refund Payment Intent**
```http
POST /v1/payment_intents/{id}/refund
```

#### Gateway Profiles

Configure and manage payment gateway connections.

**Create Gateway Profile**
```http
POST /v1/gateway_profiles
```

```json
{
  "connector_id": "stripe",
  "merchant_id": "merchant_123",
  "api_key": "sk_test_stripe_key",
  "environment": "sandbox",
  "supported_currencies": ["USD", "EUR", "GBP"],
  "supported_card_schemes": ["visa", "mastercard"]
}
```

#### Routing Policies

Configure payment routing rules.

**Create Routing Policy**
```http
POST /v1/routing_policies
```

```json
{
  "name": "US Card Routing",
  "rules": [
    {
      "condition": {
        "card_schemes": ["visa", "mastercard"],
        "currencies": ["USD"],
        "min_amount": 100,
        "max_amount": 1000000
      },
      "gateway_profile_id": "gw_stripe",
      "priority": 1
    }
  ],
  "rotation_strategy": "success_rate",
  "failover_config": {
    "max_hops": 3,
    "latency_budget_ms": 10000
  }
}
```

### Webhooks

Configure webhooks to receive real-time notifications.

**Webhook Event Types:**
- `payment_intent.created`
- `payment_intent.authorized`
- `payment_intent.captured`
- `payment_intent.failed`
- `payment_intent.refunded`
- `gateway.health_changed`

**Webhook Payload Example:**
```json
{
  "id": "evt_abc123",
  "type": "payment_intent.authorized",
  "created": 1690000000,
  "data": {
    "object": {
      "id": "pi_abc123",
      "amount": 10000,
      "currency": "usd",
      "status": "authorized"
    }
  }
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `invalid_request` | Invalid request parameters |
| `authentication_error` | Invalid API key |
| `rate_limit_error` | Too many requests |
| `payment_intent_not_found` | Payment intent not found |
| `gateway_unavailable` | Gateway is unavailable |
| `insufficient_funds` | Insufficient funds |
| `card_declined` | Card was declined |

### Rate Limits

| Plan | Requests per second |
|------|---------------------|
| Starter | 100 |
| Growth | 500 |
| Enterprise | 5000 |

### Test Cards

| Card Number | Result |
|-------------|--------|
| 4242424242424242 | Success |
| 4000000000000002 | Declined |
| 4000000000009995 | Insufficient funds |
| 4000000000009987 | Lost card |
| 4000000000009979 | Stolen card |

## SDKs

### Node.js SDK

```bash
npm install @payment-orchestra/sdk
```

```javascript
const { PaymentOrchestra } = require('@payment-orchestra/sdk');

const client = new PaymentOrchestra({
  apiKey: process.env.PAYMENT_API_KEY,
  secretKey: process.env.PAYMENT_SECRET_KEY
});

// Create payment
const payment = await client.payments.create({
  amount: 10000,
  currency: 'USD'
});
```

### Python SDK

```bash
pip install payment-orchestra
```

```python
from payment_orchestra import PaymentOrchestra

client = PaymentOrchestra(
    api_key="pk_test_your_key",
    secret_key="sk_test_your_secret"
)

# Create payment
payment = client.payments.create(
    amount=10000,
    currency="USD"
)
```

### Go SDK

```bash
go get github.com/payment-orchestra/sdk-go
```

```go
package main

import (
    "fmt"
    "github.com/payment-orchestra/sdk-go"
)

func main() {
    client := paymentorchestra.NewClient(
        "pk_test_your_key",
        "sk_test_your_secret",
    )

    payment, err := client.Payments.Create(&paymentorchestra.CreatePaymentParams{
        Amount:   10000,
        Currency: "USD",
    })
    if err != nil {
        panic(err)
    }

    fmt.Printf("Payment created: %s\n", payment.ID)
}
```

## Support

- **Documentation**: https://docs.paymentorchestra.com
- **API Status**: https://status.paymentorchestra.com
- **Support**: support@paymentorchestra.com
- **Discord**: https://discord.gg/paymentorchestra

## Changelog

### v1.0.0 (2024-01-01)
- Initial release
- Payment Intent API
- Gateway Profile management
- Routing Policy configuration
- Webhook support
- Sandbox environment
