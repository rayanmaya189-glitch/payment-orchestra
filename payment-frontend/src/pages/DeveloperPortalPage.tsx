import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Code,
  Book,
  Key,
  Webhook,
  TestTube,
  Copy,
  Check,
  ExternalLink,
  Terminal,
  FileCode,
  AlertCircle,
  RefreshCw,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { clsx } from 'clsx';

type Tab = 'overview' | 'api-keys' | 'sdks' | 'webhooks' | 'explorer';

const sdks = [
  { name: 'JavaScript / TypeScript', install: 'npm install @payment-orchestra/sdk', icon: '📦', docs: '#', github: '#' },
  { name: 'Python', install: 'pip install payment-orchestra', icon: '🐍', docs: '#', github: '#' },
  { name: 'Ruby', install: 'gem install payment_orchestra', icon: '💎', docs: '#', github: '#' },
  { name: 'Go', install: 'go get github.com/payment-orchestra/sdk-go', icon: '🔵', docs: '#', github: '#' },
  { name: 'PHP', install: 'composer require payment-orchestra/sdk', icon: '🐘', docs: '#', github: '#' },
];

const apiEndpoints = [
  { method: 'POST', path: '/v1/payment-intents', description: 'Create a payment intent' },
  { method: 'POST', path: '/v1/payment-intents/:id/authorize', description: 'Authorize a payment' },
  { method: 'POST', path: '/v1/payment-intents/:id/capture', description: 'Capture a payment' },
  { method: 'POST', path: '/v1/payment-intents/:id/refund', description: 'Refund a payment' },
  { method: 'GET', path: '/v1/payment-intents/:id', description: 'Get payment intent' },
  { method: 'GET', path: '/v1/payment-intents', description: 'List payment intents' },
  { method: 'POST', path: '/v1/routing-policies', description: 'Create routing policy' },
  { method: 'GET', path: '/v1/routing-policies', description: 'List routing policies' },
  { method: 'POST', path: '/v1/gateway-profiles', description: 'Create gateway profile' },
  { method: 'GET', path: '/v1/gateway-profiles', description: 'List gateway profiles' },
];

export function DeveloperPortalPage() {
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const [copiedText, setCopiedText] = useState<string | null>(null);
  const [expandedEndpoint, setExpandedEndpoint] = useState<string | null>(null);

  const { data: apiKeys, isLoading: keysLoading } = useQuery({
    queryKey: ['api-keys'],
    queryFn: () => api.listApiKeys(),
    retry: 2,
  });

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedText(text);
    setTimeout(() => setCopiedText(null), 2000);
  };

  const tabs = [
    { id: 'overview', label: 'Overview', icon: Book },
    { id: 'api-keys', label: 'API Keys', icon: Key },
    { id: 'sdks', label: 'SDKs', icon: Code },
    { id: 'webhooks', label: 'Webhooks', icon: Webhook },
    { id: 'explorer', label: 'API Explorer', icon: Terminal },
  ] as const;

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Developer Portal</h1>
        <p className="text-gray-500">
          API documentation, SDKs, and tools for integrating with Payment Orchestra
        </p>
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200">
        <nav className="flex space-x-8">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={clsx(
                  'flex items-center gap-2 py-4 px-1 border-b-2 font-medium text-sm transition-colors',
                  activeTab === tab.id
                    ? 'border-primary-500 text-primary-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                )}
              >
                <Icon className="w-4 h-4" />
                {tab.label}
              </button>
            );
          })}
        </nav>
      </div>

      {/* Tab Content */}
      {activeTab === 'overview' && (
        <div className="space-y-6">
          {/* Quick Start */}
          <Card>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">Quick Start</h2>
            <div className="space-y-4">
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center">
                  <span className="text-sm font-bold text-primary-600">1</span>
                </div>
                <div>
                  <h3 className="font-medium text-gray-900">Get your API key</h3>
                  <p className="text-sm text-gray-500">Create an API key in the API Keys tab</p>
                </div>
              </div>
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center">
                  <span className="text-sm font-bold text-primary-600">2</span>
                </div>
                <div>
                  <h3 className="font-medium text-gray-900">Install the SDK</h3>
                  <p className="text-sm text-gray-500">Choose your language and install the SDK</p>
                </div>
              </div>
              <div className="flex items-start gap-3">
                <div className="flex-shrink-0 w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center">
                  <span className="text-sm font-bold text-primary-600">3</span>
                </div>
                <div>
                  <h3 className="font-medium text-gray-900">Process your first payment</h3>
                  <p className="text-sm text-gray-500">Create a payment intent and authorize it</p>
                </div>
              </div>
            </div>

            {/* Code Example */}
            <div className="mt-6 bg-gray-900 rounded-lg p-4 overflow-x-auto">
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs text-gray-400">JavaScript</span>
                <button
                  onClick={() => copyToClipboard(`import { PaymentOrchestra } from '@payment-orchestra/sdk';\n\nconst client = new PaymentOrchestra({\n  apiKey: 'pk_live_...'\n});\n\nconst intent = await client.paymentIntents.create({\n  amount: 5000,\n  currency: 'USD',\n  purpose: 'payment'\n});`)}
                  className="text-gray-400 hover:text-white transition-colors"
                >
                  {copiedText ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                </button>
              </div>
              <pre className="text-sm text-gray-300 overflow-x-auto">
                <code>{`import { PaymentOrchestra } from '@payment-orchestra/sdk';

const client = new PaymentOrchestra({
  apiKey: 'pk_live_...'
});

const intent = await client.paymentIntents.create({
  amount: 5000,
  currency: 'USD',
  purpose: 'payment'
});`}</code>
              </pre>
            </div>
          </Card>

          {/* API Reference */}
          <Card>
            <h2 className="text-lg font-semibold text-gray-900 mb-4">API Reference</h2>
            <p className="text-sm text-gray-500 mb-4">
              Base URL: <code className="bg-gray-100 px-2 py-1 rounded">https://api.payment-orchestra.com</code>
            </p>
            <div className="space-y-2">
              {apiEndpoints.map((endpoint) => (
                <div
                  key={`${endpoint.method}-${endpoint.path}`}
                  className="flex items-center gap-3 p-3 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors cursor-pointer"
                  onClick={() => setExpandedEndpoint(
                    expandedEndpoint === endpoint.path ? null : endpoint.path
                  )}
                >
                  <span
                    className={clsx(
                      'px-2 py-0.5 text-xs font-bold rounded',
                      endpoint.method === 'GET'
                        ? 'bg-success-100 text-success-700'
                        : 'bg-primary-100 text-primary-700'
                    )}
                  >
                    {endpoint.method}
                  </span>
                  <span className="font-mono text-sm text-gray-900">{endpoint.path}</span>
                  <span className="text-sm text-gray-500 ml-auto">{endpoint.description}</span>
                  {expandedEndpoint === endpoint.path ? (
                    <ChevronDown className="w-4 h-4 text-gray-400" />
                  ) : (
                    <ChevronRight className="w-4 h-4 text-gray-400" />
                  )}
                </div>
              ))}
            </div>
          </Card>
        </div>
      )}

      {activeTab === 'api-keys' && (
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">API Keys</h2>
            <button className="btn-primary">
              <Key className="w-4 h-4 mr-2" />
              Create API Key
            </button>
          </div>
          {keysLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
            </div>
          ) : (
            <div className="space-y-3">
              {apiKeys && apiKeys.length > 0 ? (
                apiKeys.map((key: any) => (
                  <div key={key.id} className="flex items-center justify-between p-4 bg-gray-50 rounded-lg">
                    <div>
                      <p className="font-medium text-gray-900">{key.name}</p>
                      <p className="text-sm text-gray-500 font-mono">{key.prefix}...</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={clsx(
                        'px-2 py-1 text-xs font-medium rounded',
                        key.status === 'active' ? 'bg-success-100 text-success-700' : 'bg-gray-100 text-gray-700'
                      )}>
                        {key.status}
                      </span>
                      <button className="text-sm text-primary-600 hover:text-primary-700">Rotate</button>
                      <button className="text-sm text-danger-600 hover:text-danger-700">Revoke</button>
                    </div>
                  </div>
                ))
              ) : (
                <div className="text-center py-12 text-gray-500">
                  <Key className="w-12 h-12 mx-auto mb-4 text-gray-300" />
                  <p className="font-medium">No API keys yet</p>
                  <p className="text-sm mt-1">Create your first API key to get started</p>
                </div>
              )}
            </div>
          )}
        </Card>
      )}

      {activeTab === 'sdks' && (
        <div className="space-y-4">
          {sdks.map((sdk) => (
            <Card key={sdk.name} className="hover:shadow-md transition-shadow">
              <div className="flex items-start gap-4">
                <div className="text-3xl">{sdk.icon}</div>
                <div className="flex-1">
                  <h3 className="font-semibold text-gray-900">{sdk.name}</h3>
                  <div className="mt-2 flex items-center gap-2">
                    <code className="bg-gray-100 px-3 py-1 rounded text-sm">{sdk.install}</code>
                    <button
                      onClick={() => copyToClipboard(sdk.install)}
                      className="text-gray-400 hover:text-gray-600"
                    >
                      {copiedText === sdk.install ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                    </button>
                  </div>
                  <div className="mt-3 flex gap-4">
                    <a href={sdk.docs} className="text-sm text-primary-600 hover:text-primary-700 flex items-center gap-1">
                      <Book className="w-4 h-4" />
                      Documentation
                    </a>
                    <a href={sdk.github} className="text-sm text-primary-600 hover:text-primary-700 flex items-center gap-1">
                      <FileCode className="w-4 h-4" />
                      GitHub
                    </a>
                  </div>
                </div>
              </div>
            </Card>
          ))}
        </div>
      )}

      {activeTab === 'explorer' && (
        <Card>
          <h2 className="text-lg font-semibold text-gray-900 mb-4">API Explorer</h2>
          <p className="text-sm text-gray-500 mb-6">
            Test API endpoints directly from the browser. Use your sandbox API key for testing.
          </p>
          <div className="bg-gray-900 rounded-lg p-4 text-center">
            <Terminal className="w-12 h-12 text-gray-500 mx-auto mb-4" />
            <p className="text-gray-400">Interactive API explorer coming soon</p>
            <p className="text-sm text-gray-500 mt-2">
              In the meantime, use the{' '}
              <a href="#" className="text-primary-400 hover:text-primary-300">
                cURL examples
              </a>{' '}
              below
            </p>
          </div>

          {/* cURL Examples */}
          <div className="mt-6 space-y-4">
            <h3 className="font-medium text-gray-900">cURL Examples</h3>
            <div className="bg-gray-900 rounded-lg p-4 overflow-x-auto">
              <pre className="text-sm text-gray-300">
{`# Create a payment intent
curl -X POST https://sandbox.payment-orchestra.com/v1/payment-intents \\
  -H "Authorization: Bearer pk_test_..." \\
  -H "Content-Type: application/json" \\
  -H "Idempotency-Key: order_123" \\
  -d '{
    "amount_minor_units": 5000,
    "currency": "USD",
    "purpose": "payment"
  }'`}
              </pre>
            </div>
          </div>
        </Card>
      )}

      {activeTab === 'webhooks' && (
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Webhook Endpoints</h2>
            <button className="btn-primary">
              <Webhook className="w-4 h-4 mr-2" />
              Add Endpoint
            </button>
          </div>
          <p className="text-sm text-gray-500 mb-6">
            Receive real-time notifications when events occur in your account.
          </p>
          <div className="bg-gray-50 rounded-lg p-4">
            <h3 className="font-medium text-gray-900 mb-2">Available Events</h3>
            <div className="grid grid-cols-2 gap-2">
              {[
                'payment.created',
                'payment.authorized',
                'payment.captured',
                'payment.failed',
                'payment.refunded',
                'gateway.health_changed',
                'webhook.delivery_failed',
              ].map((event) => (
                <div key={event} className="flex items-center gap-2 text-sm">
                  <div className="w-2 h-2 bg-success-500 rounded-full" />
                  <code className="text-gray-700">{event}</code>
                </div>
              ))}
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}
