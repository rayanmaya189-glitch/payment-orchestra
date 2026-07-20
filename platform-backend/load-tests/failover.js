// k6 load test: Failover Under Load
// SRS Part 11 §7: 200 TPS, p99 < 5000ms, failover success > 95%

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

const failoverSuccess = new Counter('failover_success');
const failoverFailure = new Counter('failover_failure');

export const options = {
  stages: [
    { duration: '30s', target: 50 },   // Ramp up
    { duration: '120s', target: 200 }, // Sustained load (200 TPS)
    { duration: '30s', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<5000'],  // p99 < 5000ms (failover is slower)
    http_req_failed: ['rate<0.05'],     // Error rate < 5%
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'pk_test_123';

export default function () {
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${API_KEY}`,
  };

  // Create a payment intent
  const createPayload = JSON.stringify({
    amount: {
      amount_minor_units: Math.floor(Math.random() * 50000) + 100,
      currency_code: 'AED',
    },
    idempotency_key: `failover_${__VU}_${__ITER}_${Date.now()}`,
    purpose: 'payment',
  });

  const createRes = http.post(`${BASE_URL}/v1/payment-intents`, createPayload, { headers });

  if (createRes.status !== 201) {
    failoverFailure.add(1);
    return;
  }

  const intentId = JSON.parse(createRes.body).payment_intent_id;

  // Authorize with potential failover
  const authPayload = JSON.stringify({
    payment_method_token_id: 'tok_test_123',
  });

  const authRes = http.post(`${BASE_URL}/v1/payment-intents/${intentId}/authorize`, authPayload, { headers });

  check(authRes, {
    'authorize returns 200 or 409 (idempotent)': (r) => r.status === 200 || r.status === 409,
  });

  if (authRes.status === 200) {
    failoverSuccess.add(1);
  } else {
    failoverFailure.add(1);
  }

  sleep(0.05);
}

export function handleSummary(data) {
  return {
    stdout: JSON.stringify({
      total_requests: data.metrics.http_reqs.values.count,
      failover_success: data.metrics.failover_success.values.count,
      failover_failure: data.metrics.failover_failure.values.count,
      p99_latency: data.metrics.http_req_duration.values['p(99)'],
    }, null, 2),
  };
}
