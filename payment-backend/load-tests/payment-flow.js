import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

// Custom metrics
const paymentCreateDuration = new Trend('payment_create_duration');
const paymentAuthorizeDuration = new Trend('payment_authorize_duration');
const paymentCaptureDuration = new Trend('payment_capture_duration');
const paymentSuccessRate = new Rate('payment_success_rate');
const totalPayments = new Counter('total_payments');

// Configuration
const API_BASE_URL = __ENV.API_BASE_URL || 'https://sandbox.api.paymentorchestra.com';
const API_KEY = __ENV.API_KEY || 'pk_test_your_key';
const SECRET_KEY = __ENV.API_KEY || 'sk_test_your_secret';

export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Ramp up
    { duration: '1m', target: 50 },    // Stay at 50 VUs
    { duration: '30s', target: 100 },  // Ramp to 100 VUs
    { duration: '2m', target: 100 },   // Stay at 100 VUs
    { duration: '30s', target: 0 },    // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.01'],
    payment_success_rate: ['rate>0.99'],
  },
};

const headers = {
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${API_KEY}`,
  'X-Secret-Key': SECRET_KEY,
};

function generateIdempotencyKey() {
  return `load_test_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
}

export default function () {
  const idempotencyKey = generateIdempotencyKey();
  
  // Step 1: Create Payment Intent
  const createPayload = JSON.stringify({
    amount: Math.floor(Math.random() * 100000) + 100, // 1-1000 in minor units
    currency: 'USD',
    order_id: `LOAD-${idempotencyKey}`,
    metadata: {
      test: 'load_test',
      vu: __VU,
      iteration: __ITER,
    },
  });

  const createStart = Date.now();
  const createRes = http.post(`${API_BASE_URL}/v1/payment-intents`, createPayload, { headers });
  paymentCreateDuration.add(Date.now() - createStart);

  check(createRes, {
    'payment created': (r) => r.status === 201,
    'has payment id': (r) => {
      const body = JSON.parse(r.body);
      return body.id && body.id.startsWith('pi_');
    },
  });

  if (createRes.status !== 201) {
    paymentSuccessRate.add(0);
    totalPayments.add(1);
    return;
  }

  const paymentIntent = JSON.parse(createRes.body);
  sleep(0.1); // Small delay between operations

  // Step 2: Authorize Payment
  const authorizeStart = Date.now();
  const authorizeRes = http.post(
    `${API_BASE_URL}/v1/payment-intents/${paymentIntent.id}/authorize`,
    {},
    { headers }
  );
  paymentAuthorizeDuration.add(Date.now() - authorizeStart);

  check(authorizeRes, {
    'payment authorized': (r) => r.status === 200,
  });

  if (authorizeRes.status !== 200) {
    paymentSuccessRate.add(0);
    totalPayments.add(1);
    return;
  }

  sleep(0.1);

  // Step 3: Capture Payment
  const captureStart = Date.now();
  const captureRes = http.post(
    `${API_BASE_URL}/v1/payment-intents/${paymentIntent.id}/capture`,
    JSON.stringify({ amount: createPayload.amount }),
    { headers }
  );
  paymentCaptureDuration.add(Date.now() - captureStart);

  check(captureRes, {
    'payment captured': (r) => r.status === 200,
    'payment status captured': (r) => {
      const body = JSON.parse(r.body);
      return body.status === 'captured';
    },
  });

  const success = captureRes.status === 200;
  paymentSuccessRate.add(success ? 1 : 0);
  totalPayments.add(1);

  // Small delay before next iteration
  sleep(0.5);
}

export function handleSummary(data) {
  return {
    'load-tests/results/summary.json': JSON.stringify(data, null, 2),
    stdout: textSummary(data, { indent: ' ', enableColors: true }),
  };
}

function textSummary(data, options) {
  // Simplified summary output
  const metrics = data.metrics;
  const lines = [];
  
  lines.push('\n=== Load Test Summary ===\n');
  lines.push(`Total Requests: ${metrics.http_reqs?.values?.count || 0}`);
  lines.push(`Failed Requests: ${metrics.http_req_failed?.values?.rate || 0}`);
  lines.push(`P50 Latency: ${metrics.http_req_duration?.values?.p50 || 0}ms`);
  lines.push(`P95 Latency: ${metrics.http_req_duration?.values?.p95 || 0}ms`);
  lines.push(`P99 Latency: ${metrics.http_req_duration?.values?.p99 || 0}ms`);
  lines.push(`Payment Success Rate: ${metrics.payment_success_rate?.values?.rate || 0}`);
  lines.push(`Total Payments: ${metrics.total_payments?.values?.count || 0}`);
  
  return lines.join('\n');
}
