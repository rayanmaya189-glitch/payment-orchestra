import { useState } from 'react';
import { Plus, Copy, Trash2, Eye, EyeOff, Key, Shield } from 'lucide-react';
import { Card, CardHeader } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { copyToClipboard, truncate, formatRelativeTime } from '@/utils/format';
import toast from 'react-hot-toast';

const mockApiKeys = [
  {
    id: 'key_001',
    name: 'Production API Key',
    key_prefix: 'sk_live_',
    scopes: ['payments:read', 'payments:write'],
    environment: 'production',
    last_used_at: new Date(Date.now() - 3600000),
    created_at: new Date(Date.now() - 30 * 24 * 3600000),
  },
  {
    id: 'key_002',
    name: 'Sandbox Testing',
    key_prefix: 'sk_test_',
    scopes: ['payments:read', 'payments:write', 'gateways:read'],
    environment: 'sandbox',
    last_used_at: new Date(Date.now() - 1800000),
    created_at: new Date(Date.now() - 15 * 24 * 3600000),
  },
  {
    id: 'key_003',
    name: 'Webhook Integration',
    key_prefix: 'whsec_',
    scopes: ['webhooks:manage'],
    environment: 'sandbox',
    last_used_at: null,
    created_at: new Date(Date.now() - 7 * 24 * 3600000),
  },
];

export function ApiKeysPage() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(new Set());

  const handleCopyKey = async (keyId: string) => {
    await copyToClipboard(`sk_test_example_key_${keyId}`);
    toast.success('API key copied to clipboard');
  };

  const toggleKeyVisibility = (keyId: string) => {
    setVisibleKeys((prev) => {
      const next = new Set(prev);
      if (next.has(keyId)) {
        next.delete(keyId);
      } else {
        next.add(keyId);
      }
      return next;
    });
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">API Keys</h1>
          <p className="text-gray-500">Manage your API keys for authentication</p>
        </div>
        <button className="btn-primary" onClick={() => setShowCreateModal(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Create API Key
        </button>
      </div>

      {/* Security Notice */}
      <Card className="border-warning-200 bg-warning-50">
        <div className="flex items-start gap-3">
          <Shield className="w-5 h-5 text-warning-600 mt-0.5" />
          <div>
            <h3 className="font-medium text-warning-800">Keep your API keys secure</h3>
            <p className="text-sm text-warning-700 mt-1">
              Never share your API keys in public repositories, client-side code, or chat messages.
              Use environment variables to store them securely.
            </p>
          </div>
        </div>
      </Card>

      {/* API Keys List */}
      <Card>
        <div className="space-y-4">
          {mockApiKeys.map((apiKey) => (
            <div
              key={apiKey.id}
              className="flex items-center justify-between p-4 bg-gray-50 rounded-lg"
            >
              <div className="flex items-center gap-4">
                <div className="p-2 bg-white rounded-lg border border-gray-200">
                  <Key className="w-5 h-5 text-gray-600" />
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="font-medium text-gray-900">{apiKey.name}</h3>
                    <StatusBadge status={apiKey.environment} />
                  </div>
                  <div className="flex items-center gap-2 mt-1">
                    <code className="text-sm font-mono text-gray-600 bg-white px-2 py-0.5 rounded border border-gray-200">
                      {visibleKeys.has(apiKey.id)
                        ? `${apiKey.key_prefix}sk_live_xxxxxxxxxxxx`
                        : `${apiKey.key_prefix}${'•'.repeat(16)}`}
                    </code>
                    <button
                      onClick={() => toggleKeyVisibility(apiKey.id)}
                      className="text-gray-400 hover:text-gray-600"
                    >
                      {visibleKeys.has(apiKey.id) ? (
                        <EyeOff className="w-4 h-4" />
                      ) : (
                        <Eye className="w-4 h-4" />
                      )}
                    </button>
                    <button
                      onClick={() => handleCopyKey(apiKey.id)}
                      className="text-gray-400 hover:text-gray-600"
                    >
                      <Copy className="w-4 h-4" />
                    </button>
                  </div>
                  <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
                    <span>
                      Scopes: {apiKey.scopes.join(', ')}
                    </span>
                    {apiKey.last_used_at && (
                      <span>Last used: {formatRelativeTime(apiKey.last_used_at)}</span>
                    )}
                  </div>
                </div>
              </div>
              <button className="p-2 text-gray-400 hover:text-danger-600 hover:bg-danger-50 rounded-lg transition-colors">
                <Trash2 className="w-5 h-5" />
              </button>
            </div>
          ))}
        </div>
      </Card>

      {/* Create API Key Modal Placeholder */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <Card className="w-full max-w-md">
            <CardHeader title="Create API Key" />
            <div className="space-y-4">
              <div>
                <label className="label">Key Name</label>
                <input type="text" className="input" placeholder="e.g., Production Server" />
              </div>
              <div>
                <label className="label">Environment</label>
                <select className="input">
                  <option value="sandbox">Sandbox</option>
                  <option value="production">Production</option>
                </select>
              </div>
              <div>
                <label className="label">Permissions</label>
                <div className="space-y-2">
                  {['payments:read', 'payments:write', 'gateways:read', 'webhooks:manage'].map(
                    (scope) => (
                      <label key={scope} className="flex items-center gap-2">
                        <input type="checkbox" className="rounded border-gray-300 text-primary-600" />
                        <span className="text-sm text-gray-700">{scope}</span>
                      </label>
                    )
                  )}
                </div>
              </div>
              <div className="flex gap-3 mt-6">
                <button
                  className="btn-secondary flex-1"
                  onClick={() => setShowCreateModal(false)}
                >
                  Cancel
                </button>
                <button className="btn-primary flex-1">Create Key</button>
              </div>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
