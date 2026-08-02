import { useState } from 'react';
import { Plus, Settings, TestTube, CheckCircle, XCircle, AlertCircle } from 'lucide-react';
import { Card, CardHeader } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { clsx } from 'clsx';

const mockGateways = [
  {
    id: 'gw_stripe',
    connector_id: 'stripe',
    name: 'Stripe',
    environment: 'sandbox',
    status: 'active',
    success_rate: 98.5,
    avg_latency: 120,
    last_health_check: '2 min ago',
    supported_currencies: ['USD', 'EUR', 'GBP'],
  },
  {
    id: 'gw_checkout',
    connector_id: 'checkout_com',
    name: 'Checkout.com',
    environment: 'sandbox',
    status: 'active',
    success_rate: 97.8,
    avg_latency: 145,
    last_health_check: '5 min ago',
    supported_currencies: ['USD', 'AED', 'EUR'],
  },
  {
    id: 'gw_network',
    connector_id: 'network_intl',
    name: 'Network International',
    environment: 'sandbox',
    status: 'active',
    success_rate: 96.2,
    avg_latency: 180,
    last_health_check: '3 min ago',
    supported_currencies: ['AED', 'USD'],
  },
  {
    id: 'gw_razorpay',
    connector_id: 'razorpay',
    name: 'Razorpay',
    environment: 'sandbox',
    status: 'error',
    success_rate: 95.8,
    avg_latency: 210,
    last_health_check: '10 min ago',
    supported_currencies: ['INR'],
  },
];

const statusConfig = {
  active: { icon: CheckCircle, color: 'text-success-600', bg: 'bg-success-50' },
  error: { icon: XCircle, color: 'text-danger-600', bg: 'bg-danger-50' },
  testing: { icon: AlertCircle, color: 'text-warning-600', bg: 'bg-warning-50' },
};

export function ConnectorsPage() {
  const [showAddModal, setShowAddModal] = useState(false);

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Connectors</h1>
          <p className="text-gray-500">Manage your payment gateway connections</p>
        </div>
        <button className="btn-primary" onClick={() => setShowAddModal(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Add Connector
        </button>
      </div>

      {/* Gateway Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {mockGateways.map((gateway) => {
          const config = statusConfig[gateway.status as keyof typeof statusConfig];
          const StatusIcon = config.icon;
          
          return (
            <Card key={gateway.id} className="hover:shadow-md transition-shadow">
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-center gap-3">
                  <div className="w-12 h-12 bg-gray-100 rounded-lg flex items-center justify-center">
                    <span className="text-xl font-bold text-gray-700">
                      {gateway.name.charAt(0)}
                    </span>
                  </div>
                  <div>
                    <h3 className="font-semibold text-gray-900">{gateway.name}</h3>
                    <p className="text-sm text-gray-500 capitalize">
                      {gateway.environment}
                    </p>
                  </div>
                </div>
                <div className={clsx('p-2 rounded-lg', config.bg)}>
                  <StatusIcon className={clsx('w-5 h-5', config.color)} />
                </div>
              </div>

              {/* Metrics */}
              <div className="grid grid-cols-2 gap-4 mb-4">
                <div className="text-center p-3 bg-gray-50 rounded-lg">
                  <p className="text-2xl font-bold text-gray-900">
                    {gateway.success_rate}%
                  </p>
                  <p className="text-xs text-gray-500">Success Rate</p>
                </div>
                <div className="text-center p-3 bg-gray-50 rounded-lg">
                  <p className="text-2xl font-bold text-gray-900">
                    {gateway.avg_latency}ms
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

              {/* Last Check */}
              <p className="text-xs text-gray-400 mb-4">
                Last health check: {gateway.last_health_check}
              </p>

              {/* Actions */}
              <div className="flex gap-2">
                <button className="flex-1 btn-secondary text-sm">
                  <TestTube className="w-4 h-4 mr-1" />
                  Test
                </button>
                <button className="flex-1 btn-secondary text-sm">
                  <Settings className="w-4 h-4 mr-1" />
                  Configure
                </button>
              </div>
            </Card>
          );
        })}

        {/* Add New Connector Card */}
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
      </div>
    </div>
  );
}
