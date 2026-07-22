# Load Tests

K6-based load testing scripts for the Payment Orchestra platform.

## Prerequisites
- [k6](https://k6.io/docs/getting-started/installation/)

## Running

```bash
k6 run checkout.js
k6 run failover.js
```

## Test Scenarios

- `checkout.js` — Simulates the complete checkout flow: create payment intent → authorize → capture
- `failover.js` — Simulates acquirer failure scenarios and verifies automatic failover
