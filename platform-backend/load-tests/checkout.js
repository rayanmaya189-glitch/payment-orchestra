// k6 load test: Checkout Hot Path
// SRS Part 11 §7: 500 TPS, p99 < 2000ms, error < 1%

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const checkoutSuccess = new Counter('checkout_success');
const checkoutFailure = new Counter('checkout_failure');
const checkoutLatency = new Trend('checkout_latency');

export const options = {
  stages: [
    { duration: '30s', target: 100 },  // Ramp up
    { duration: '60s', target: 500 },  // Sustained load (500 TPS target)
    { duration: '30s', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<2000'],  // p99 < 2000ms
    http_req_failed: ['rate<0.01'],     // Error rate < 1%
    checkout_success: ['count>10000'],   // At least 10K successful checkouts
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const API_KEY = __ENV.API_KEY || 'pk_test_123';

export default function () {
  const headers = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${API_KEY}`,
  };

  // Step 1: Create Payment Intent
  const createPayload = JSON.stringify({
    amount: {
      amount_minor_units: Math.floor(Math.random() * 100000) + 100,
      currency_code: 'AED',
    },
    idempotency_key: `load_${__VU}_${__ITER}`,
    purpose: 'payment',
  });

  const createRes = http.post(`${BASE_URL}/v1/payment-intents`, createPayload, { headers });

  check(createRes, {
    'create intent status 201': (r) => r.status === 201,
  });

  if (createRes.status !== 201) {
    checkoutFailure.add(1);
    return;
  }

  const intentId = JSON.parse(createRes.body).payment_intent_id;

  // Step 2: Authorize
  const authPayload = JSON.stringify({
    payment_method_token_id: 'tok_test_123',
  });

  const authRes = http.post(`${BASE_URL}/v1/payment-intents/${intentId}/authorize`, authPayload, { headers });

  check(authRes, {
    'authorize status 200': (r) => r.status === 200,
  });

  if (authRes.status !== 200) {
    checkoutFailure.add(1);
    return;
  }

  // Step 3: Capture
  const captureRes = http.post(`${BASE_URL}/v1/payment-intents/${intentId}/capture`, '{}', { headers });

  check(captureRes, {
    'capture status 200': (r) => r.status === 200,
  });

  if (captureRes.status === 200) {
    checkoutSuccess.add(1);
    checkoutLatency.add(createRes.timings.duration + authRes.timings.duration + captureRes.timings.duration);
  } else {
    checkoutFailure.add(1);
  }

  sleep(0.1); // Small delay between iterations
}

export function handleSummary(data) {
  return {
    stdout: JSON.stringify({
      total_requests: data.metrics.http_reqs.values.count,
      success_rate: data.metrics.checkout_success.values.count,
      failure_rate: data.metrics.checkout_failure.values.count,
      p99_latency: data.metrics.http_req_duration.values['p(99)'],
      p95_latency: data.metrics.http_req_duration.values['p(95)'],
      avg_latency: data.metrics.http_req_duration.values.avg,
    }, null, 2),
  };
}
