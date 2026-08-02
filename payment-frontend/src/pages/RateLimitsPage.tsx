import { useState } from 'react';
import { 
  RefreshCw, 
  Activity,
  Clock,
  Zap,
  Shield,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { formatNumber } from '@/utils/format';

// Mock rate limit data for demonstration
const mockRateLimitData = {
  global: {
    requests_per_second: 1000,
    current_usage: 450,
    usage_percentage: 45,
  },
  endpoints: [
    {
      path: '/v1/payment-intents',
      method: 'POST',
      limit: 100,
      current: 78,
      window: '1s',
      blocked: 2,
    },
    {
      path: '/v1/gateway-profiles',
      method: 'GET',
      limit: 500,
      current: 120,
      window: '1s',
      blocked: 0,
    },
    {
      path: '/v1/routing-policies',
      method: 'POST',
      limit: 50,
      current: 12,
      window: '1s',
      blocked: 0,
    },
    {
      path: '/v1/webhooks',
      method: 'POST',
      limit: 20,
      current: 8,
      window: '1s',
      blocked: 1,
    },
  ],
  top_consumers: [
    { api_key: 'sk_live_abc...xyz', requests: 12500, blocked: 5 },
    { api_key: 'sk_live_def...uvw', requests: 8200, blocked: 0 },
    { api_key: 'sk_test_ghi...rst', requests: 3400, blocked: 2 },
    { api_key: 'sk_live_jkl...opq', requests: 1800, blocked: 0 },
  ],
};

export function RateLimitsPage() {
  const [timeRange, setTimeRange] = useState('1h');

  // In production, this would fetch from the API
  const rateLimitData = mockRateLimitData;

  const methodColor: Record<string, string> = {
    GET: 'bg-success-100 text-success-700',
    POST: 'bg-primary-100 text-primary-700',
    PUT: 'bg-warning-100 text-warning-700',
    DELETE: 'bg-danger-100 text-danger-700',
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Rate Limits</h1>
          <p className="text-gray-500">Monitor API rate limiting and usage</p>
        </div>
        <div className="flex items-center gap-3">
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(e.target.value)}
            className="px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value="5m">Last 5 minutes</option>
            <option value="15m">Last 15 minutes</option>
            <option value="1h">Last 1 hour</option>
            <option value="24h">Last 24 hours</option>
          </select>
          <button className="btn-secondary flex items-center gap-2">
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>
      </div>

      {/* Global Rate Limit */}
      <Card>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Global Rate Limit</h2>
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-500">Limit:</span>
            <span className="font-medium">{formatNumber(rateLimitData.global.requests_per_second)} req/s</span>
          </div>
        </div>
        
        <div className="space-y-4">
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-gray-600">Current Usage</span>
              <span className="text-sm font-medium">{rateLimitData.global.current_usage} / {rateLimitData.global.requests_per_second}</span>
            </div>
            <div className="w-full bg-gray-100 rounded-full h-3">
              <div
                className={`h-3 rounded-full transition-all duration-500 ${
                  rateLimitData.global.usage_percentage > 80
                    ? 'bg-danger-500'
                    : rateLimitData.global.usage_percentage > 60
                    ? 'bg-warning-500'
                    : 'bg-success-500'
                }`}
                style={{ width: `${Math.min(rateLimitData.global.usage_percentage, 100)}%` }}
              />
            </div>
            <div className="flex items-center justify-between mt-1">
              <span className="text-xs text-gray-400">0%</span>
              <span className={`text-xs font-medium ${
                rateLimitData.global.usage_percentage > 80
                  ? 'text-danger-600'
                  : rateLimitData.global.usage_percentage > 60
                  ? 'text-warning-600'
                  : 'text-success-600'
              }`}>
                {rateLimitData.global.usage_percentage}%
              </span>
              <span className="text-xs text-gray-400">100%</span>
            </div>
          </div>
        </div>
      </Card>

      {/* Endpoint Rate Limits */}
      <Card>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Endpoint Rate Limits</h2>
        </div>
        
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="text-left text-sm text-gray-500 border-b">
                <th className="pb-3 font-medium">Endpoint</th>
                <th className="pb-3 font-medium">Method</th>
                <th className="pb-3 font-medium">Limit</th>
                <th className="pb-3 font-medium">Current</th>
                <th className="pb-3 font-medium">Usage</th>
                <th className="pb-3 font-medium">Blocked</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {rateLimitData.endpoints.map((endpoint, index) => (
                <tr key={index} className="hover:bg-gray-50">
                  <td className="py-4">
                    <code className="text-sm font-mono text-gray-900">{endpoint.path}</code>
                  </td>
                  <td className="py-4">
                    <span className={`px-2 py-0.5 text-xs font-medium rounded ${methodColor[endpoint.method]}`}>
                      {endpoint.method}
                    </span>
                  </td>
                  <td className="py-4 text-sm text-gray-600">{endpoint.limit}/s</td>
                  <td className="py-4 text-sm text-gray-600">{endpoint.current}</td>
                  <td className="py-4">
                    <div className="w-24 bg-gray-100 rounded-full h-2">
                      <div
                        className={`h-2 rounded-full ${
                          (endpoint.current / endpoint.limit) > 0.8
                            ? 'bg-danger-500'
                            : (endpoint.current / endpoint.limit) > 0.6
                            ? 'bg-warning-500'
                            : 'bg-success-500'
                        }`}
                        style={{ width: `${Math.min((endpoint.current / endpoint.limit) * 100, 100)}%` }}
                      />
                    </div>
                    <span className="text-xs text-gray-500 mt-1">
                      {Math.round((endpoint.current / endpoint.limit) * 100)}%
                    </span>
                  </td>
                  <td className="py-4">
                    {endpoint.blocked > 0 ? (
                      <span className="px-2 py-0.5 text-xs font-medium bg-danger-100 text-danger-700 rounded">
                        {endpoint.blocked}
                      </span>
                    ) : (
                      <span className="text-sm text-gray-400">0</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      {/* Top Consumers */}
      <Card>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Top API Key Consumers</h2>
        </div>
        
        <div className="space-y-4">
          {rateLimitData.top_consumers.map((consumer, index) => (
            <div key={index} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-gray-500">#{index + 1}</span>
                <code className="text-sm font-mono text-gray-900">{consumer.api_key}</code>
              </div>
              <div className="flex items-center gap-4">
                <div className="text-right">
                  <p className="text-sm font-medium text-gray-900">{formatNumber(consumer.requests)}</p>
                  <p className="text-xs text-gray-500">requests</p>
                </div>
                {consumer.blocked > 0 && (
                  <span className="px-2 py-0.5 text-xs font-medium bg-danger-100 text-danger-700 rounded">
                    {consumer.blocked} blocked
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      </Card>

      {/* Rate Limit Configuration */}
      <Card>
        <div className="flex items-center gap-3 mb-4">
          <Shield className="w-5 h-5 text-gray-600" />
          <h2 className="text-lg font-semibold text-gray-900">Configuration</h2>
        </div>
        
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 bg-gray-50 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <Zap className="w-4 h-4 text-primary-600" />
              <span className="text-sm font-medium text-gray-700">Algorithm</span>
            </div>
            <p className="text-lg font-semibold text-gray-900">Sliding Window</p>
            <p className="text-xs text-gray-500 mt-1">Per-second sliding window counter</p>
          </div>
          
          <div className="p-4 bg-gray-50 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <Clock className="w-4 h-4 text-primary-600" />
              <span className="text-sm font-medium text-gray-700">Window Size</span>
            </div>
            <p className="text-lg font-semibold text-gray-900">1 second</p>
            <p className="text-xs text-gray-500 mt-1">Rolling window interval</p>
          </div>
          
          <div className="p-4 bg-gray-50 rounded-lg">
            <div className="flex items-center gap-2 mb-2">
              <Activity className="w-4 h-4 text-primary-600" />
              <span className="text-sm font-medium text-gray-700">Backend</span>
            </div>
            <p className="text-lg font-semibold text-gray-900">Redis</p>
            <p className="text-xs text-gray-500 mt-1">Distributed rate limiting</p>
          </div>
        </div>
      </Card>
    </div>
  );
}
