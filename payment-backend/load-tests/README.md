# Load Testing for Payment Orchestra

This directory contains load testing scripts for validating the Payment Orchestra platform performance.

## Prerequisites

- [k6](https://k6.io/) load testing tool
- Node.js 18+ (for SDK-based tests)

## Running Tests

### Basic Load Test

```bash
k6 run payment-flow.js
```

### Stress Test

```bash
k6 run --vus 100 --duration 5m payment-flow.js
```

### Spike Test

```bash
k6 run --vus 1000 --duration 10s payment-flow.js
```

## Test Scenarios

### payment-flow.js

Tests the complete payment lifecycle:
1. Create payment intent
2. Authorize payment
3. Capture payment
4. Verify status

### connector-failover.js

Tests connector failover behavior:
1. Create payment with primary gateway
2. Simulate gateway failure
3. Verify failover to secondary gateway
4. Verify payment completion

### concurrent-transactions.js

Tests concurrent transaction handling:
1. Simulate 1000 concurrent payment creations
2. Verify no data corruption
3. Verify idempotency
4. Measure throughput

## Performance Targets

| Metric | Target | Description |
|--------|--------|-------------|
| P50 Latency | < 200ms | Median response time |
| P95 Latency | < 500ms | 95th percentile latency |
| P99 Latency | < 1000ms | 99th percentile latency |
| Throughput | > 10,000 TPS | Transactions per second |
| Error Rate | < 0.1% | Failed requests |
| Availability | > 99.9% | Uptime during test |

## Environment Variables

```bash
export API_BASE_URL="https://sandbox.api.paymentorchestra.com"
export API_KEY="pk_test_your_key"
export SECRET_KEY="sk_test_your_secret"
```

## Interpreting Results

k6 outputs metrics including:
- `http_req_duration`: Request latency
- `http_reqs`: Total requests
- `http_req_failed`: Failed requests
- `iterations`: Test iterations completed

## Best Practices

1. Run load tests against sandbox environment only
2. Start with low VUs and increase gradually
3. Monitor system metrics (CPU, memory, connections) during tests
4. Compare results across releases
5. Document baseline performance metrics
