// Checkout flow load test
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 10 },  // Ramp up
    { duration: '1m', target: 50 },   // Steady state
    { duration: '30s', target: 0 },   // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(95)<3000'],  // 95% of requests under 3s
  },
};

export default function () {
  const url = 'http://localhost:9000/v1/payment-intents';
  
  const payload = new Uint8Array([/* protobuf-encoded CreatePaymentIntentRequest */]);
  
  const params = {
    headers: {
      'Content-Type': 'application/protobuf',
      'Authorization': 'Bearer test-api-key',
    },
  };

  const res = http.post(url, payload, params);
  
  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 3s': (r) => r.timings.duration < 3000,
  });

  sleep(1);
}
