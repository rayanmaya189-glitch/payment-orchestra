# Load Testing

## Prerequisites

Install k6:
```bash
# macOS
brew install k6

# Linux
sudo snap install k6
```

## Running Tests

### Checkout Hot Path (500 TPS)
```bash
k6 run load-tests/checkout.js \
  --env BASE_URL=http://localhost:8080 \
  --env API_KEY=pk_test_123
```

### Failover Under Load (200 TPS)
```bash
k6 run load-tests/failover.js \
  --env BASE_URL=http://localhost:8080 \
  --env API_KEY=pk_test_123
```

### With Docker Compose
```bash
# Start infrastructure
docker-compose up -d

# Run services (build first)
cargo run --release --bin orchestration-service

# Run load test
k6 run load-tests/checkout.js
```

## SRS Requirements (Part 11 §7)

| Test | TPS | p99 Latency | Error Rate |
|------|-----|-------------|------------|
| Single-hop authorization | 500 | < 2000ms | < 1% |
| Failover under load | 200 | < 5000ms | < 5% |
| Concurrent same-card | 100 | - | Double auth = 0 |
| Settlement ingestion burst | 50 | - | Match rate > 99% |

## Metrics

- `checkout_success` — Successful checkout completions
- `checkout_failure` — Failed checkout attempts
- `checkout_latency` — Total checkout latency (create + authorize + capture)
- `failover_success` — Successful failover attempts
- `failover_failure` — Failed failover attempts
