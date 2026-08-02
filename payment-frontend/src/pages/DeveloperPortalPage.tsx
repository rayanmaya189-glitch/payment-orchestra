import { useState } from 'react';
import { 
  Book, 
  Code, 
  Webhook, 
  Shield, 
  Copy, 
  Check, 
  ExternalLink,
  Terminal,
  FileCode,
  Zap,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import toast from 'react-hot-toast';

const apiEndpoints = [
  {
    method: 'POST',
    path: '/v1/payment-intents',
    description: 'Create a new payment intent',
    category: 'Payments',
  },
  {
    method: 'GET',
    path: '/v1/payment-intents/:id',
    description: 'Get a payment intent by ID',
    category: 'Payments',
  },
  {
    method: 'POST',
    path: '/v1/payment-intents/:id/capture',
    description: 'Capture an authorized payment',
    category: 'Payments',
  },
  {
    method: 'POST',
    path: '/v1/payment-intents/:id/void',
    description: 'Void an authorized payment',
    category: 'Payments',
  },
  {
    method: 'POST',
    path: '/v1/payment-intents/:id/refund',
    description: 'Refund a captured payment',
    category: 'Payments',
  },
  {
    method: 'GET',
    path: '/v1/gateway-profiles',
    description: 'List all gateway profiles',
    category: 'Gateways',
  },
  {
    method: 'POST',
    path: '/v1/gateway-profiles',
    description: 'Create a new gateway profile',
    category: 'Gateways',
  },
  {
    method: 'POST',
    path: '/v1/gateway-profiles/:id/test',
    description: 'Test gateway connection',
    category: 'Gateways',
  },
  {
    method: 'GET',
    path: '/v1/routing-policies',
    description: 'List all routing policies',
    category: 'Routing',
  },
  {
    method: 'POST',
    path: '/v1/routing-policies',
    description: 'Create a new routing policy',
    category: 'Routing',
  },
  {
    method: 'POST',
    path: '/v1/routing-policies/:id/activate',
    description: 'Activate a routing policy',
    category: 'Routing',
  },
  {
    method: 'GET',
    path: '/v1/webhooks',
    description: 'List webhook endpoints',
    category: 'Webhooks',
  },
  {
    method: 'POST',
    path: '/v1/webhooks',
    description: 'Create a webhook endpoint',
    category: 'Webhooks',
  },
];

const codeExamples = [
  {
    title: 'Create a Payment Intent',
    language: 'curl',
    code: `curl -X POST https://api.paymentorchestra.com/v1/payment-intents \\
  -H "Authorization: Bearer sk_test_your_api_key" \\
  -H "Content-Type: application/json" \\
  -d '{
    "amount": 1000,
    "currency": "USD",
    "order_id": "ORD-12345",
    "metadata": {
      "customer_id": "cust_123",
      "description": "Order #12345"
    }
  }'`,
  },
  {
    title: 'Create a Payment Intent (Node.js)',
    language: 'javascript',
    code: `import PaymentOrchestra from '@paymentorchestra/sdk';

const client = new PaymentOrchestra({
  apiKey: 'sk_test_your_api_key',
});

const paymentIntent = await client.paymentIntents.create({
  amount: 1000,
  currency: 'USD',
  orderId: 'ORD-12345',
  metadata: {
    customerId: 'cust_123',
    description: 'Order #12345',
  },
});

console.log(paymentIntent.id);`,
  },
  {
    title: 'Create a Payment Intent (Python)',
    language: 'python',
    code: `from payment_orchestra import PaymentOrchestra

client = PaymentOrchestra(api_key="sk_test_your_api_key")

payment_intent = client.payment_intents.create(
    amount=1000,
    currency="USD",
    order_id="ORD-12345",
    metadata={
        "customer_id": "cust_123",
        "description": "Order #12345"
    }
)

print(payment_intent.id)`,
  },
  {
    title: 'Create a Payment Intent (Go)',
    language: 'go',
    code: `package main

import (
    "fmt"
    "github.com/paymentorchestra/paymentorchestra-go"
)

func main() {
    client := paymentorchestra.NewClient("sk_test_your_api_key")

    paymentIntent, err := client.PaymentIntents.Create(
        &paymentorchestra.CreatePaymentIntentParams{
            Amount:   paymentorchestra.Int64(1000),
            Currency: paymentorchestra.String("USD"),
            OrderID:  paymentorchestra.String("ORD-12345"),
        },
    )
    if err != nil {
        panic(err)
    }

    fmt.Println(paymentIntent.ID)
}`,
  },
];

const webhookEvents = [
  'payment_intent.created',
  'payment_intent.authorized',
  'payment_intent.captured',
  'payment_intent.failed',
  'payment_intent.refunded',
  'gateway.health_changed',
  'routing.policy.activated',
  'routing.policy.deactivated',
];

export function DeveloperPortalPage() {
  const [activeTab, setActiveTab] = useState<'overview' | 'api' | 'sdks' | 'webhooks' | 'testing'>('overview');
  const [copiedCode, setCopiedCode] = useState<number | null>(null);
  const [selectedEndpoint, setSelectedEndpoint] = useState<number | null>(null);

  const handleCopyCode = async (code: string, index: number) => {
    await navigator.clipboard.writeText(code);
    setCopiedCode(index);
    toast.success('Code copied to clipboard');
    setTimeout(() => setCopiedCode(null), 2000);
  };

  const methodColors: Record<string, string> = {
    GET: 'bg-success-100 text-success-700',
    POST: 'bg-primary-100 text-primary-700',
    PUT: 'bg-warning-100 text-warning-700',
    PATCH: 'bg-warning-100 text-warning-700',
    DELETE: 'bg-danger-100 text-danger-700',
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Developer Portal</h1>
        <p className="text-gray-500">API documentation, SDKs, and integration guides</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-1 bg-gray-100 p-1 rounded-lg">
        {[
          { id: 'overview', label: 'Overview', icon: Book },
          { id: 'api', label: 'API Reference', icon: Code },
          { id: 'sdks', label: 'SDKs', icon: FileCode },
          { id: 'webhooks', label: 'Webhooks', icon: Webhook },
          { id: 'testing', label: 'Testing', icon: Terminal },
        ].map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id as any)}
            className={`flex items-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-colors ${
              activeTab === tab.id
                ? 'bg-white text-gray-900 shadow-sm'
                : 'text-gray-600 hover:text-gray-900'
            }`}
          >
            <tab.icon className="w-4 h-4" />
            {tab.label}
          </button>
        ))}
      </div>

      {/* Overview Tab */}
      {activeTab === 'overview' && (
        <div className="space-y-6">
          {/* Quick Start */}
          <Card>
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2 bg-primary-100 rounded-lg">
                <Zap className="w-5 h-5 text-primary-600" />
              </div>
              <h2 className="text-lg font-semibold text-gray-900">Quick Start</h2>
            </div>
            <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="p-4 bg-gray-50 rounded-lg">
                  <div className="text-2xl font-bold text-primary-600 mb-2">1</div>
                  <h3 className="font-medium text-gray-900">Get API Keys</h3>
                  <p className="text-sm text-gray-500">Create a sandbox API key from the Settings page</p>
                </div>
                <div className="p-4 bg-gray-50 rounded-lg">
                  <div className="text-2xl font-bold text-primary-600 mb-2">2</div>
                  <h3 className="font-medium text-gray-900">Connect Gateway</h3>
                  <p className="text-sm text-gray-500">Add your payment gateway credentials</p>
                </div>
                <div className="p-4 bg-gray-50 rounded-lg">
                  <div className="text-2xl font-bold text-primary-600 mb-2">3</div>
                  <h3 className="font-medium text-gray-900">Make Your First Payment</h3>
                  <p className="text-sm text-gray-500">Create a payment intent and start processing</p>
                </div>
              </div>
            </div>
          </Card>

          {/* Base URL */}
          <Card>
            <h3 className="font-medium text-gray-900 mb-2">Base URL</h3>
            <div className="flex items-center gap-2">
              <code className="flex-1 p-3 bg-gray-100 rounded-lg font-mono text-sm">
                https://api.paymentorchestra.com
              </code>
              <button
                onClick={() => handleCopyCode('https://api.paymentorchestra.com', 0)}
                className="p-2 text-gray-500 hover:text-gray-700"
              >
                {copiedCode === 0 ? <Check className="w-5 h-5 text-success-500" /> : <Copy className="w-5 h-5" />}
              </button>
            </div>
          </Card>

          {/* Authentication */}
          <Card>
            <div className="flex items-center gap-3 mb-4">
              <Shield className="w-5 h-5 text-gray-600" />
              <h3 className="font-medium text-gray-900">Authentication</h3>
            </div>
            <p className="text-sm text-gray-600 mb-4">
              All API requests must include an Authorization header with your API key:
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 p-3 bg-gray-100 rounded-lg font-mono text-sm">
                Authorization: Bearer sk_test_your_api_key
              </code>
            </div>
          </Card>

          {/* Quick Example */}
          <Card>
            <h3 className="font-medium text-gray-900 mb-4">Quick Example</h3>
            <div className="relative">
              <pre className="p-4 bg-gray-900 text-gray-100 rounded-lg overflow-x-auto text-sm">
                {codeExamples[0].code}
              </pre>
              <button
                onClick={() => handleCopyCode(codeExamples[0].code, 1)}
                className="absolute top-2 right-2 p-2 text-gray-400 hover:text-white"
              >
                {copiedCode === 1 ? <Check className="w-4 h-4 text-success-400" /> : <Copy className="w-4 h-4" />}
              </button>
            </div>
          </Card>
        </div>
      )}

      {/* API Reference Tab */}
      {activeTab === 'api' && (
        <div className="space-y-6">
          <Card>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">API Endpoints</h2>
            <p className="text-sm text-gray-500 mb-6">
              All endpoints use POST for mutations and return JSON responses.
            </p>

            <div className="space-y-4">
              {apiEndpoints.map((endpoint, index) => (
                <div
                  key={index}
                  className="border border-gray-200 rounded-lg p-4 hover:border-primary-300 transition-colors cursor-pointer"
                  onClick={() => setSelectedEndpoint(selectedEndpoint === index ? null : index)}
                >
                  <div className="flex items-center gap-3">
                    <span className={`px-2 py-1 text-xs font-medium rounded ${methodColors[endpoint.method]}`}>
                      {endpoint.method}
                    </span>
                    <code className="font-mono text-sm text-gray-900">{endpoint.path}</code>
                    <span className="text-sm text-gray-500 ml-auto">{endpoint.category}</span>
                  </div>
                  <p className="text-sm text-gray-600 mt-2">{endpoint.description}</p>
                  
                  {selectedEndpoint === index && (
                    <div className="mt-4 pt-4 border-t border-gray-100">
                      <h4 className="font-medium text-gray-900 mb-2">Request Body</h4>
                      <pre className="p-3 bg-gray-100 rounded-lg text-sm font-mono">
{`{
  "amount": 1000,
  "currency": "USD",
  "order_id": "ORD-12345"
}`}
                      </pre>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

      {/* SDKs Tab */}
      {activeTab === 'sdks' && (
        <div className="space-y-6">
          <Card>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Official SDKs</h2>
            <p className="text-sm text-gray-500 mb-6">
              Use our official SDKs to integrate quickly with your favorite language.
            </p>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {[
                { name: 'Node.js', package: '@paymentorchestra/sdk', install: 'npm install @paymentorchestra/sdk' },
                { name: 'Python', package: 'payment-orchestra', install: 'pip install payment-orchestra' },
                { name: 'Go', package: 'github.com/paymentorchestra/paymentorchestra-go', install: 'go get github.com/paymentorchestra/paymentorchestra-go' },
                { name: 'PHP', package: 'paymentorchestra/paymentorchestra-php', install: 'composer require paymentorchestra/paymentorchestra-php' },
                { name: 'React Native', package: 'react-native-payment-orchestra', install: 'npm install react-native-payment-orchestra' },
              ].map((sdk) => (
                <div key={sdk.name} className="p-4 border border-gray-200 rounded-lg">
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="font-medium text-gray-900">{sdk.name}</h3>
                    <a href="#" className="text-primary-600 hover:text-primary-700">
                      <ExternalLink className="w-4 h-4" />
                    </a>
                  </div>
                  <code className="text-sm text-gray-600 block mb-2">{sdk.package}</code>
                  <div className="flex items-center gap-2">
                    <code className="flex-1 p-2 bg-gray-100 rounded text-xs font-mono">
                      {sdk.install}
                    </code>
                    <button
                      onClick={() => handleCopyCode(sdk.install, 10)}
                      className="p-1 text-gray-400 hover:text-gray-600"
                    >
                      {copiedCode === 10 ? <Check className="w-4 h-4 text-success-500" /> : <Copy className="w-4 h-4" />}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </Card>

          {/* Code Examples */}
          <Card>
            <h3 className="font-medium text-gray-900 mb-4">Code Examples</h3>
            <div className="space-y-4">
              {codeExamples.map((example, index) => (
                <div key={index}>
                  <div className="flex items-center justify-between mb-2">
                    <h4 className="font-medium text-gray-900">{example.title}</h4>
                    <span className="text-xs text-gray-500 uppercase">{example.language}</span>
                  </div>
                  <div className="relative">
                    <pre className="p-4 bg-gray-900 text-gray-100 rounded-lg overflow-x-auto text-sm">
                      {example.code}
                    </pre>
                    <button
                      onClick={() => handleCopyCode(example.code, 20 + index)}
                      className="absolute top-2 right-2 p-2 text-gray-400 hover:text-white"
                    >
                      {copiedCode === 20 + index ? <Check className="w-4 h-4 text-success-400" /> : <Copy className="w-4 h-4" />}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

      {/* Webhooks Tab */}
      {activeTab === 'webhooks' && (
        <div className="space-y-6">
          <Card>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Webhook Events</h2>
            <p className="text-sm text-gray-500 mb-6">
              Subscribe to events to receive notifications when something happens.
            </p>

            <div className="space-y-3">
              {webhookEvents.map((event) => (
                <div key={event} className="flex items-center gap-3 p-3 bg-gray-50 rounded-lg">
                  <div className="w-2 h-2 bg-success-500 rounded-full"></div>
                  <code className="font-mono text-sm text-gray-900">{event}</code>
                </div>
              ))}
            </div>
          </Card>

          {/* Webhook Payload */}
          <Card>
            <h3 className="font-medium text-gray-900 mb-4">Webhook Payload Example</h3>
            <pre className="p-4 bg-gray-900 text-gray-100 rounded-lg overflow-x-auto text-sm">
{`{
  "id": "evt_1234567890",
  "type": "payment_intent.captured",
  "created_at": "2024-01-15T10:30:00Z",
  "data": {
    "object": {
      "id": "pi_abc123",
      "amount": 1000,
      "currency": "USD",
      "status": "captured"
    }
  }
}`}
            </pre>
          </Card>

          {/* Webhook Security */}
          <Card>
            <div className="flex items-center gap-3 mb-4">
              <Shield className="w-5 h-5 text-gray-600" />
              <h3 className="font-medium text-gray-900">Webhook Signature Verification</h3>
            </div>
            <p className="text-sm text-gray-600 mb-4">
              All webhook payloads are signed with HMAC-SHA256. Verify the signature using the <code>X-Webhook-Signature</code> header.
            </p>
            <pre className="p-4 bg-gray-900 text-gray-100 rounded-lg overflow-x-auto text-sm">
{`const crypto = require('crypto');

function verifyWebhookSignature(payload, signature, secret) {
  const expectedSignature = crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expectedSignature)
  );
}`}
            </pre>
          </Card>
        </div>
      )}

      {/* Testing Tab */}
      {activeTab === 'testing' && (
        <div className="space-y-6">
          <Card>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Test Mode</h2>
            <p className="text-sm text-gray-500 mb-6">
              Use sandbox mode to test your integration without affecting real data.
            </p>

            <div className="space-y-4">
              <div className="p-4 bg-warning-50 border border-warning-200 rounded-lg">
                <h3 className="font-medium text-warning-800 mb-2">Sandbox Environment</h3>
                <p className="text-sm text-warning-700">
                  Use test API keys (sk_test_*) and sandbox mode to test your integration.
                  No real money will be processed.
                </p>
              </div>

              <div>
                <h3 className="font-medium text-gray-900 mb-2">Test API Keys</h3>
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <code className="flex-1 p-2 bg-gray-100 rounded text-sm font-mono">
                      sk_test_sandbox_key_for_testing
                    </code>
                    <button className="p-1 text-gray-400 hover:text-gray-600">
                      <Copy className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </div>

              <div>
                <h3 className="font-medium text-gray-900 mb-2">Test Cards</h3>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="text-left text-gray-500 border-b">
                        <th className="pb-2 font-medium">Card Number</th>
                        <th className="pb-2 font-medium">Result</th>
                        <th className="pb-2 font-medium">Description</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y">
                      <tr>
                        <td className="py-2 font-mono">4242424242424242</td>
                        <td className="py-2 text-success-600">Success</td>
                        <td className="py-2 text-gray-600">Always succeeds</td>
                      </tr>
                      <tr>
                        <td className="py-2 font-mono">4000000000000002</td>
                        <td className="py-2 text-danger-600">Declined</td>
                        <td className="py-2 text-gray-600">Generic decline</td>
                      </tr>
                      <tr>
                        <td className="py-2 font-mono">4000000000009995</td>
                        <td className="py-2 text-danger-600">Declined</td>
                        <td className="py-2 text-gray-600">Insufficient funds</td>
                      </tr>
                      <tr>
                        <td className="py-2 font-mono">4000000000009987</td>
                        <td className="py-2 text-danger-600">Declined</td>
                        <td className="py-2 text-gray-600">Lost card</td>
                      </tr>
                      <tr>
                        <td className="py-2 font-mono">4000000000000069</td>
                        <td className="py-2 text-warning-600">Error</td>
                        <td className="py-2 text-gray-600">Expired card</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </Card>

          {/* Webhook Simulator */}
          <Card>
            <h3 className="font-medium text-gray-900 mb-4">Webhook Simulator</h3>
            <p className="text-sm text-gray-600 mb-4">
              Test your webhook endpoint by sending a test event.
            </p>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Webhook URL
                </label>
                <input
                  type="url"
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                  placeholder="https://your-app.com/webhooks"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  Event Type
                </label>
                <select className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500">
                  {webhookEvents.map((event) => (
                    <option key={event} value={event}>{event}</option>
                  ))}
                </select>
              </div>
              <button className="btn-primary">
                Send Test Event
              </button>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
