import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, Copy, Trash2, Eye, EyeOff, Key, Shield, AlertCircle, RefreshCw, Loader2, Check, RotateCw } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { api, ApiError } from '@/services/api';
import { copyToClipboard, formatRelativeTime } from '@/utils/format';
import toast from 'react-hot-toast';

export function ApiKeysPage() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [visibleKeys, setVisibleKeys] = useState<Set<string>>(new Set());
  const [newKeyName, setNewKeyName] = useState('');
  const [newKeyEnvironment, setNewKeyEnvironment] = useState<'sandbox' | 'production'>('sandbox');
  const [newKeyScopes, setNewKeyScopes] = useState<string[]>(['payments:read']);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  
  const queryClient = useQueryClient();

  // Fetch API keys
  const {
    data: apiKeys,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['api-keys'],
    queryFn: () => api.listApiKeys(),
    retry: 2,
  });

  // Create API key mutation
  const createMutation = useMutation({
    mutationFn: api.createApiKey,
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      setCreatedKey(data.key);
      toast.success('API key created successfully');
      // Reset form
      setNewKeyName('');
      setNewKeyEnvironment('sandbox');
      setNewKeyScopes(['payments:read']);
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to create API key');
    },
  });

  // Revoke API key mutation
  const revokeMutation = useMutation({
    mutationFn: api.revokeApiKey,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      toast.success('API key revoked');
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to revoke API key');
    },
  });

  // Rotate API key mutation
  const rotateMutation = useMutation({
    mutationFn: api.rotateApiKey,
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['api-keys'] });
      setCreatedKey(data.key);
      toast.success('API key rotated successfully');
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to rotate API key');
    },
  });

  const handleCopyKey = async (key: string) => {
    await copyToClipboard(key);
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

  const handleCreateKey = () => {
    if (!newKeyName.trim()) {
      toast.error('Please enter a key name');
      return;
    }
    createMutation.mutate({
      name: newKeyName.trim(),
      scopes: newKeyScopes,
      environment: newKeyEnvironment,
    });
  };

  const handleRevokeKey = (id: string) => {
    if (window.confirm('Are you sure you want to revoke this API key? This action cannot be undone.')) {
      revokeMutation.mutate(id);
    }
  };

  const handleRotateKey = (id: string) => {
    if (window.confirm('Are you sure you want to rotate this API key? The old key will stop working immediately.')) {
      rotateMutation.mutate(id);
    }
  };

  const toggleScope = (scope: string) => {
    setNewKeyScopes((prev) => {
      if (prev.includes(scope)) {
        return prev.filter((s) => s !== scope);
      } else {
        return [...prev, scope];
      }
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
        <button
          className="btn-primary flex items-center gap-2"
          onClick={() => {
            setCreatedKey(null);
            setShowCreateModal(true);
          }}
        >
          <Plus className="w-4 h-4" />
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

      {/* Error State */}
      {error && (
        <Card className="border-danger-200 bg-danger-50">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-danger-600" />
            <div className="flex-1">
              <p className="text-sm font-medium text-danger-800">Error loading API keys</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load API keys'}
              </p>
            </div>
            <button
              onClick={() => refetch()}
              className="p-2 text-danger-600 hover:bg-danger-100 rounded-lg transition-colors"
            >
              <RefreshCw className="w-4 h-4" />
            </button>
          </div>
        </Card>
      )}

      {/* API Keys List */}
      <Card>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
          </div>
        ) : apiKeys && apiKeys.length > 0 ? (
          <div className="space-y-4">
            {apiKeys.map((apiKey) => (
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
                      {apiKey.expires_at && new Date(apiKey.expires_at) < new Date() && (
                        <span className="px-2 py-0.5 text-xs bg-warning-100 text-warning-700 rounded">
                          Expired
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-2 mt-1">
                      <code className="text-sm font-mono text-gray-600 bg-white px-2 py-0.5 rounded border border-gray-200">
                        {apiKey.key_prefix}{visibleKeys.has(apiKey.id) ? 'sk_live_xxxxxxxxxxxx' : '•'.repeat(16)}
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
                        onClick={() => handleCopyKey(`${apiKey.key_prefix}sk_live_xxxxxxxxxxxx`)}
                        className="text-gray-400 hover:text-gray-600"
                      >
                        <Copy className="w-4 h-4" />
                      </button>
                    </div>
                    <div className="flex items-center gap-4 mt-2 text-xs text-gray-500">
                      <span>Scopes: {apiKey.scopes.join(', ')}</span>
                      {apiKey.last_used_at && (
                        <span>Last used: {formatRelativeTime(apiKey.last_used_at)}</span>
                      )}
                      <span>Created: {formatRelativeTime(apiKey.created_at)}</span>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleRotateKey(apiKey.id)}
                    disabled={rotateMutation.isPending}
                    className="p-2 text-gray-400 hover:text-warning-600 hover:bg-warning-50 rounded-lg transition-colors"
                    title="Rotate key"
                  >
                    {rotateMutation.isPending ? (
                      <Loader2 className="w-5 h-5 animate-spin" />
                    ) : (
                      <RotateCw className="w-5 h-5" />
                    )}
                  </button>
                  <button
                    onClick={() => handleRevokeKey(apiKey.id)}
                    disabled={revokeMutation.isPending}
                    className="p-2 text-gray-400 hover:text-danger-600 hover:bg-danger-50 rounded-lg transition-colors"
                    title="Revoke key"
                  >
                    <Trash2 className="w-5 h-5" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12">
            <Key className="w-12 h-12 text-gray-300 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-gray-900">No API keys yet</h3>
            <p className="text-gray-500 mt-1">Create your first API key to start integrating</p>
            <button
              className="btn-primary mt-4"
              onClick={() => setShowCreateModal(true)}
            >
              <Plus className="w-4 h-4 mr-2" />
              Create API Key
            </button>
          </div>
        )}
      </Card>

      {/* Create API Key Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <Card className="w-full max-w-md">
            <div className="p-6">
              <h2 className="text-xl font-semibold text-gray-900 mb-4">
                {createdKey ? 'API Key Created' : 'Create API Key'}
              </h2>
              
              {createdKey ? (
                // Show created key
                <div className="space-y-4">
                  <div className="p-4 bg-success-50 border border-success-200 rounded-lg">
                    <p className="text-sm text-success-700 mb-2">
                      Your API key has been created. Copy it now - you won't be able to see it again.
                    </p>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 p-2 bg-white border border-success-200 rounded font-mono text-sm break-all">
                        {createdKey}
                      </code>
                      <button
                        onClick={() => handleCopyKey(createdKey)}
                        className="p-2 text-success-600 hover:bg-success-100 rounded"
                      >
                        <Copy className="w-5 h-5" />
                      </button>
                    </div>
                  </div>
                  <button
                    className="btn-primary w-full"
                    onClick={() => {
                      setShowCreateModal(false);
                      setCreatedKey(null);
                    }}
                  >
                    Done
                  </button>
                </div>
              ) : (
                // Create form
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Key Name
                    </label>
                    <input
                      type="text"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      placeholder="e.g., Production Server"
                      value={newKeyName}
                      onChange={(e) => setNewKeyName(e.target.value)}
                    />
                  </div>
                  
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Environment
                    </label>
                    <select
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      value={newKeyEnvironment}
                      onChange={(e) => setNewKeyEnvironment(e.target.value as 'sandbox' | 'production')}
                    >
                      <option value="sandbox">Sandbox</option>
                      <option value="production">Production</option>
                    </select>
                  </div>
                  
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      Permissions
                    </label>
                    <div className="space-y-2">
                      {[
                        { id: 'payments:read', label: 'Read payments', description: 'View payment details and status' },
                        { id: 'payments:write', label: 'Create payments', description: 'Create and manage payments' },
                        { id: 'gateways:read', label: 'Read gateways', description: 'View gateway configurations' },
                        { id: 'gateways:write', label: 'Manage gateways', description: 'Create and configure gateways' },
                        { id: 'webhooks:manage', label: 'Manage webhooks', description: 'Create and manage webhook endpoints' },
                      ].map((scope) => (
                        <label
                          key={scope.id}
                          className="flex items-start gap-3 p-2 rounded-lg hover:bg-gray-50 cursor-pointer"
                        >
                          <input
                            type="checkbox"
                            className="mt-1 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                            checked={newKeyScopes.includes(scope.id)}
                            onChange={() => toggleScope(scope.id)}
                          />
                          <div>
                            <span className="text-sm font-medium text-gray-700">{scope.label}</span>
                            <p className="text-xs text-gray-500">{scope.description}</p>
                          </div>
                        </label>
                      ))}
                    </div>
                  </div>
                  
                  <div className="flex gap-3 mt-6">
                    <button
                      className="btn-secondary flex-1"
                      onClick={() => setShowCreateModal(false)}
                    >
                      Cancel
                    </button>
                    <button
                      className="btn-primary flex-1 flex items-center justify-center gap-2"
                      onClick={handleCreateKey}
                      disabled={createMutation.isPending || !newKeyName.trim()}
                    >
                      {createMutation.isPending ? (
                        <>
                          <Loader2 className="w-4 h-4 animate-spin" />
                          Creating...
                        </>
                      ) : (
                        <>
                          <Check className="w-4 h-4" />
                          Create Key
                        </>
                      )}
                    </button>
                  </div>
                </div>
              )}
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
