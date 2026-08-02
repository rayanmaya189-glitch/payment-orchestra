import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Plus, Settings, TestTube, CheckCircle, XCircle, AlertCircle, RefreshCw, Loader2 } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { clsx } from 'clsx';

const statusConfig = {
  active: { icon: CheckCircle, color: 'text-success-600', bg: 'bg-success-50', label: 'Active' },
  inactive: { icon: XCircle, color: 'text-gray-400', bg: 'bg-gray-50', label: 'Inactive' },
  error: { icon: XCircle, color: 'text-danger-600', bg: 'bg-danger-50', label: 'Error' },
  testing: { icon: Loader2, color: 'text-warning-600', bg: 'bg-warning-50', label: 'Testing' },
};

export function ConnectorsPage() {
  const [showAddModal, setShowAddModal] = useState(false);
  const [testingId, setTestingId] = useState<string | null>(null);

  // Fetch gateway profiles
  const {
    data: gateways,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['gateway-profiles'],
    queryFn: () => api.listGatewayProfiles(),
    retry: 2,
  });

  // Fetch available connectors
  const {
    data: connectors,
    isLoading: connectorsLoading,
  } = useQuery({
    queryKey: ['connectors'],
    queryFn: () => api.listConnectors(),
    retry: 2,
  });

  const handleTestConnection = async (id: string) => {
    setTestingId(id);
    try {
      const result = await api.testGatewayConnection(id);
      if (result.success) {
        // Show success toast
        console.log('Connection test successful:', result.message);
      } else {
        // Show error toast
        console.error('Connection test failed:', result.message);
      }
    } catch (error) {
      console.error('Test connection error:', error);
    } finally {
      setTestingId(null);
    }
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Connectors</h1>
          <p className="text-gray-500">Manage your payment gateway connections</p>
        </div>
        <button
          onClick={() => setShowAddModal(true)}
          className="btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          Add Connector
        </button>
      </div>

      {/* Error State */}
      {error && (
        <Card className="border-danger-200 bg-danger-50">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-danger-600" />
            <div className="flex-1">
              <p className="text-sm font-medium text-danger-800">Error loading connectors</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load connectors'}
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

      {/* Gateway Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {isLoading ? (
          // Loading skeletons
          Array.from({ length: 3 }).map((_, i) => (
            <Card key={i} className="animate-pulse">
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div className="w-12 h-12 bg-gray-200 rounded-lg"></div>
                  <div>
                    <div className="h-4 bg-gray-200 rounded w-24 mb-2"></div>
                    <div className="h-3 bg-gray-200 rounded w-16"></div>
                  </div>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-4 mb-4">
                <div className="h-16 bg-gray-200 rounded-lg"></div>
                <div className="h-16 bg-gray-200 rounded-lg"></div>
              </div>
            </Card>
          ))
        ) : gateways && gateways.length > 0 ? (
          gateways.map((gateway) => {
            const config = statusConfig[gateway.status as keyof typeof statusConfig] || statusConfig.inactive;
            const StatusIcon = config.icon;
            const isTesting = testingId === gateway.id;
            
            return (
              <Card key={gateway.id} className="hover:shadow-md transition-shadow">
                <div className="flex items-start justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <div className="w-12 h-12 bg-gray-100 rounded-lg flex items-center justify-center">
                      <span className="text-xl font-bold text-gray-700">
                        {gateway.display_name.charAt(0)}
                      </span>
                    </div>
                    <div>
                      <h3 className="font-semibold text-gray-900">{gateway.display_name}</h3>
                      <p className="text-sm text-gray-500 capitalize">
                        {gateway.connector_id} • {gateway.environment}
                      </p>
                    </div>
                  </div>
                  <div className={clsx('p-2 rounded-lg', config.bg)}>
                    <StatusIcon className={clsx('w-5 h-5', config.color, isTesting && 'animate-spin')} />
                  </div>
                </div>

                {/* Metrics */}
                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div className="text-center p-3 bg-gray-50 rounded-lg">
                    <p className="text-2xl font-bold text-gray-900">
                      {gateway.success_rate !== undefined ? `${gateway.success_rate}%` : '-'}
                    </p>
                    <p className="text-xs text-gray-500">Success Rate</p>
                  </div>
                  <div className="text-center p-3 bg-gray-50 rounded-lg">
                    <p className="text-2xl font-bold text-gray-900">
                      {gateway.avg_latency_ms !== undefined ? `${gateway.avg_latency_ms}ms` : '-'}
                    </p>
                    <p className="text-xs text-gray-500">Avg Latency</p>
                  </div>
                </div>

                {/* Currencies */}
                <div className="flex flex-wrap gap-1 mb-4">
                  {gateway.supported_currencies.map((currency) => (
                    <span
                      key={currency}
                      className="px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-600 rounded"
                    >
                      {currency}
                    </span>
                  ))}
                </div>

                {/* Last Health Check */}
                {gateway.last_health_check && (
                  <p className="text-xs text-gray-400 mb-4">
                    Last health check: {new Date(gateway.last_health_check).toLocaleString()}
                  </p>
                )}

                {/* Actions */}
                <div className="flex gap-2">
                  <button
                    onClick={() => handleTestConnection(gateway.id)}
                    disabled={isTesting}
                    className="flex-1 btn-secondary text-sm flex items-center justify-center gap-1"
                  >
                    {isTesting ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <TestTube className="w-4 h-4" />
                    )}
                    Test
                  </button>
                  <button className="flex-1 btn-secondary text-sm flex items-center justify-center gap-1">
                    <Settings className="w-4 h-4" />
                    Configure
                  </button>
                </div>
              </Card>
            );
          })
        ) : (
          // Empty state
          <>
            <button
              onClick={() => setShowAddModal(true)}
              className="border-2 border-dashed border-gray-300 rounded-xl p-6 hover:border-primary-400 hover:bg-primary-50 transition-all flex flex-col items-center justify-center min-h-[280px]"
            >
              <Plus className="w-12 h-12 text-gray-400 mb-4" />
              <span className="text-lg font-medium text-gray-700">Add Connector</span>
              <span className="text-sm text-gray-500 mt-1">
                Connect a new payment gateway
              </span>
            </button>
          </>
        )}

        {/* Add New Connector Card (if we have gateways) */}
        {gateways && gateways.length > 0 && (
          <button
            onClick={() => setShowAddModal(true)}
            className="border-2 border-dashed border-gray-300 rounded-xl p-6 hover:border-primary-400 hover:bg-primary-50 transition-all flex flex-col items-center justify-center min-h-[280px]"
          >
            <Plus className="w-12 h-12 text-gray-400 mb-4" />
            <span className="text-lg font-medium text-gray-700">Add Connector</span>
            <span className="text-sm text-gray-500 mt-1">
              Connect a new payment gateway
            </span>
          </button>
        )}
      </div>

      {/* Available Connectors Summary */}
      {connectors && connectors.length > 0 && (
        <Card>
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Available Integrations</h3>
          <p className="text-sm text-gray-500 mb-4">
            We support {connectors.length} payment providers. Connect one to get started.
          </p>
          <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-3">
            {connectors.slice(0, 12).map((connector) => (
              <div
                key={connector.id}
                className="p-3 border border-gray-200 rounded-lg hover:border-primary-300 hover:bg-primary-50 transition-colors cursor-pointer"
              >
                <div className="font-medium text-sm text-gray-900">{connector.name}</div>
                <div className="text-xs text-gray-500 mt-1">
                  {connector.supported_currencies.length} currencies
                </div>
              </div>
            ))}
            {connectors.length > 12 && (
              <div className="p-3 border border-gray-200 rounded-lg flex items-center justify-center">
                <span className="text-sm text-gray-500">+{connectors.length - 12} more</span>
              </div>
            )}
          </div>
        </Card>
      )}

      {/* Add Connector Modal Placeholder */}
      {showAddModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <Card className="w-full max-w-2xl max-h-[90vh] overflow-y-auto">
            <div className="p-6">
              <div className="flex items-center justify-between mb-6">
                <h2 className="text-xl font-semibold text-gray-900">Add Connector</h2>
                <button
                  onClick={() => setShowAddModal(false)}
                  className="text-gray-400 hover:text-gray-600"
                >
                  <XCircle className="w-6 h-6" />
                </button>
              </div>
              
              <div className="space-y-4">
                <p className="text-gray-600">
                  Select a payment provider to connect. You'll need your API credentials from the provider.
                </p>
                
                {connectorsLoading ? (
                  <div className="flex items-center justify-center py-8">
                    <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
                  </div>
                ) : connectors ? (
                  <div className="grid grid-cols-2 gap-3">
                    {connectors.map((connector) => (
                      <button
                        key={connector.id}
                        onClick={() => {
                          // TODO: Open credential form for selected connector
                          console.log('Selected connector:', connector.id);
                        }}
                        className="p-4 border border-gray-200 rounded-lg hover:border-primary-400 hover:bg-primary-50 transition-colors text-left"
                      >
                        <div className="font-medium text-gray-900">{connector.name}</div>
                        <div className="text-sm text-gray-500 mt-1">{connector.description}</div>
                        <div className="flex flex-wrap gap-1 mt-2">
                          {connector.supported_currencies.slice(0, 3).map((c) => (
                            <span key={c} className="text-xs bg-gray-100 px-2 py-0.5 rounded">
                              {c}
                            </span>
                          ))}
                          {connector.supported_currencies.length > 3 && (
                            <span className="text-xs bg-gray-100 px-2 py-0.5 rounded">
                              +{connector.supported_currencies.length - 3}
                            </span>
                          )}
                        </div>
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
