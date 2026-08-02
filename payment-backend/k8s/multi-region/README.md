# Multi-Region Deployment Strategy

## Overview

Payment Orchestra supports multi-region deployment for:
- **Latency optimization**: Route to nearest region
- **Data residency**: Comply with local data protection laws (India DPDP, UAE PDPL)
- **High availability**: Cross-region failover
- **Disaster recovery**: RPO < 1 minute, RTO < 5 minutes

## Supported Regions

| Region | Code | Data Center | Use Case |
|--------|------|-------------|----------|
| India - Mumbai | ap-south-1 | AWS Mumbai | Primary for INR payments |
| India - Delhi | ap-south-2 | AWS Delhi | DR for India |
| UAE - Dubai | me-south-1 | AWS Bahrain | Primary for AED payments |
| UAE - Abu Dhabi | me-central-1 | AWS UAE | DR for UAE |
| EU - Frankfurt | eu-central-1 | AWS Frankfurt | International cards |
| US - Virginia | us-east-1 | AWS Virginia | Global fallback |

## Architecture

```
                    ┌─────────────────────────────────────┐
                    │      Global Load Balancer (GLB)     │
                    │   (Route53 / Cloudflare Workers)    │
                    └──────────────┬──────────────────────┘
                                   │
           ┌───────────────────────┼───────────────────────┐
           │                       │                       │
           ▼                       ▼                       ▼
   ┌───────────────┐      ┌───────────────┐      ┌───────────────┐
   │   ap-south-1  │      │   me-south-1  │      │   eu-central-1│
   │    (Mumbai)   │      │    (Dubai)    │      │  (Frankfurt)  │
   └───────┬───────┘      └───────┬───────┘      └───────┬───────┘
           │                       │                       │
           ▼                       ▼                       ▼
   ┌───────────────┐      ┌───────────────┐      ┌───────────────┐
   │  PostgreSQL   │      │  PostgreSQL   │      │  PostgreSQL   │
   │   Primary +   │      │   Primary +   │      │   Primary +   │
   │   Read Replicas│     │   Read Replicas│     │   Read Replicas│
   └───────────────┘      └───────────────┘      └───────────────┘
```

## Region Selection Logic

```rust
// PaymentIntent includes region_hint based on:
// 1. Merchant's primary operating region
// 2. Card BIN country detection
// 3. Currency (INR → India, AED → UAE)
// 4. Customer IP geolocation

pub enum Region {
    ApSouth1,    // India - Mumbai
    ApSouth2,    // India - Delhi (DR)
    MeSouth1,    // UAE - Dubai
    MeCentral1,  // UAE - Abu Dhabi (DR)
    EuCentral1,  // EU - Frankfurt
    UsEast1,     // US - Virginia (Global fallback)
}

impl Region {
    pub fn from_currency(currency: &Currency) -> Self {
        match currency {
            Currency::INR => Region::ApSouth1,
            Currency::AED => Region::MeSouth1,
            Currency::EUR => Region::EuCentral1,
            _ => Region::UsEast1,
        }
    }
}
```

## Data Residency Compliance

### India (DPDP Act 2023)
- Payment data for Indian users stored in India regions only
- Cross-border transfer requires explicit consent
- Data localization for "critical personal financial data"

### UAE (PDPL)
- Personal data must be processed within UAE or approved jurisdictions
- Financial data subject to Central Bank regulations
- Cross-border transfer requires DPO approval

### Implementation

```rust
pub struct DataResidencyConfig {
    pub allowed_regions: Vec<Region>,
    pub require_local_storage: bool,
    pub cross_border_consent: bool,
    pub retention_days: u32,
}

impl DataResidencyConfig {
    pub fn india() -> Self {
        Self {
            allowed_regions: vec![Region::ApSouth1, Region::ApSouth2],
            require_local_storage: true,
            cross_border_consent: true,
            retention_days: 1825, // 5 years
        }
    }

    pub fn uae() -> Self {
        Self {
            allowed_regions: vec![Region::MeSouth1, Region::MeCentral1],
            require_local_storage: true,
            cross_border_consent: true,
            retention_days: 1825, // 5 years
        }
    }
}
```

## Cross-Region Replication

### Event Replication (NATS JetStream)
```nats
# Stream configuration for cross-region
STREAM: payment-events
REPLICAS: 3
SOURCES: []
MAX_MESSAGES: -1
MAX_BYTES: -1
STORAGE: file
RETENTION: limits
MAX_AGE: 86400000000000  # 24 hours
MAX_MSG_SIZE: 1048576
```

### Database Replication
- **Async replication**: For read replicas (eventual consistency OK)
- **Sync replication**: For critical payment data (RPO = 0)
- **Conflict resolution**: Last-writer-wins with vector clocks

## Failover Strategy

### Automatic Failover
```yaml
health_check:
  interval: 10s
  timeout: 5s
  unhealthy_threshold: 3
  healthy_threshold: 2

failover:
  mode: automatic
  primary_region: ap-south-1
  failover_regions:
    - ap-south-2
    - me-south-1
  max_failover_latency_ms: 100
```

### Manual Failover
```bash
# Trigger manual failover
curl -X POST https://api.paymentorchestra.com/admin/failover \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"from_region": "ap-south-1", "to_region": "ap-south-2", "reason": "maintenance"}'
```

## Deployment Process

### Blue-Green Deployment
1. Deploy new version to green environment
2. Run smoke tests
3. Switch traffic (gradual: 10% → 50% → 100%)
4. Monitor for errors
5. Decommission blue environment

### Canary Deployment
```yaml
canary:
  initial_weight: 5
  interval: 5m
  steps:
    - weight: 5
      pause: 5m
    - weight: 25
      pause: 10m
    - weight: 50
      pause: 10m
    - weight: 100
  analysis:
    metrics:
      - name: error_rate
        threshold: 0.01
      - name: latency_p99
        threshold: 500ms
```

## Monitoring

### Per-Region Metrics
- Request latency (p50, p95, p99)
- Error rate by type
- Transaction success rate
- Connector availability
- Database replication lag

### Cross-Region Metrics
- Global failover count
- Data residency violations
- Cross-region latency
- Replication lag
