import { useState } from 'react';
import { Plus, Play, Pause, ChevronRight, GitBranch, Zap, Shield } from 'lucide-react';
import { Card, CardHeader } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { clsx } from 'clsx';

const mockPolicies = [
  {
    id: 'rp_001',
    name: 'US Card Primary',
    status: 'active' as const,
    strategy: 'success_rate_based',
    rules_count: 3,
    success_rate: 98.2,
    created_at: '2024-01-15',
  },
  {
    id: 'rp_002',
    name: 'UAE Cards Fallback',
    status: 'active' as const,
    strategy: 'priority',
    rules_count: 2,
    success_rate: 96.5,
    created_at: '2024-01-20',
  },
  {
    id: 'rp_003',
    name: 'High-Value Transactions',
    status: 'inactive' as const,
    strategy: 'cost_based',
    rules_count: 4,
    success_rate: 0,
    created_at: '2024-02-01',
  },
];

const strategyLabels: Record<string, string> = {
  priority: 'Priority',
  round_robin: 'Round Robin',
  weighted_round_robin: 'Weighted Round Robin',
  cost_based: 'Cost-Based',
  success_rate_based: 'Success Rate',
  volume_capped: 'Volume Capped',
};

const strategyIcons: Record<string, typeof GitBranch> = {
  priority: Shield,
  round_robin: GitBranch,
  weighted_round_robin: GitBranch,
  cost_based: Zap,
  success_rate_based: Zap,
  volume_capped: Zap,
};

export function RoutingPage() {
  const [selectedPolicy, setSelectedPolicy] = useState<string | null>(null);

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Routing Policies</h1>
          <p className="text-gray-500">Configure how payments are routed across your gateways</p>
        </div>
        <button className="btn-primary">
          <Plus className="w-4 h-4 mr-2" />
          Create Policy
        </button>
      </div>

      {/* Policy List */}
      <div className="space-y-4">
        {mockPolicies.map((policy) => {
          const StrategyIcon = strategyIcons[policy.strategy] || GitBranch;
          return (
            <Card
              key={policy.id}
              className={clsx(
                'cursor-pointer transition-all hover:shadow-md',
                selectedPolicy === policy.id && 'ring-2 ring-primary-500'
              )}
              onClick={() => setSelectedPolicy(policy.id)}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="p-3 bg-primary-50 rounded-lg">
                    <StrategyIcon className="w-6 h-6 text-primary-600" />
                  </div>
                  <div>
                    <div className="flex items-center gap-3">
                      <h3 className="text-lg font-semibold text-gray-900">
                        {policy.name}
                      </h3>
                      <StatusBadge status={policy.status} />
                    </div>
                    <div className="flex items-center gap-4 mt-1 text-sm text-gray-500">
                      <span>{strategyLabels[policy.strategy]}</span>
                      <span>•</span>
                      <span>{policy.rules_count} rules</span>
                      {policy.success_rate > 0 && (
                        <>
                          <span>•</span>
                          <span className="text-success-600">
                            {policy.success_rate}% success rate
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <button
                    className={clsx(
                      'p-2 rounded-lg transition-colors',
                      policy.status === 'active'
                        ? 'text-success-600 hover:bg-success-50'
                        : 'text-gray-400 hover:bg-gray-100'
                    )}
                    title={policy.status === 'active' ? 'Deactivate' : 'Activate'}
                  >
                    {policy.status === 'active' ? (
                      <Pause className="w-5 h-5" />
                    ) : (
                      <Play className="w-5 h-5" />
                    )}
                  </button>
                  <ChevronRight className="w-5 h-5 text-gray-400" />
                </div>
              </div>
            </Card>
          );
        })}
      </div>

      {/* Create New Policy Dialog Placeholder */}
      <Card className="border-dashed border-2 border-gray-300 hover:border-primary-400 transition-colors cursor-pointer">
        <div className="text-center py-8">
          <Plus className="w-12 h-12 text-gray-400 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900">Create New Policy</h3>
          <p className="text-gray-500 mt-1">
            Define routing rules to optimize your payment success rates
          </p>
        </div>
      </Card>
    </div>
  );
}
