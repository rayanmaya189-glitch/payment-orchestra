import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Plus, Play, Pause, ChevronRight, GitBranch, Zap, Shield, AlertCircle, RefreshCw, Loader2 } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { api, ApiError } from '@/services/api';
import { clsx } from 'clsx';
import type { RoutingPolicy } from '@/types';

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
  const queryClient = useQueryClient();

  // Fetch routing policies
  const {
    data: policies,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['routing-policies'],
    queryFn: () => api.listRoutingPolicies(),
    retry: 2,
  });

  // Toggle policy status mutation
  const toggleMutation = useMutation({
    mutationFn: async (policy: RoutingPolicy) => {
      if (policy.status === 'active') {
        return api.deactivateRoutingPolicy(policy.id);
      } else {
        return api.activateRoutingPolicy(policy.id);
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['routing-policies'] });
    },
    onError: (error) => {
      console.error('Failed to toggle policy:', error);
    },
  });

  const handleTogglePolicy = (policy: RoutingPolicy) => {
    toggleMutation.mutate(policy);
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Routing Policies</h1>
          <p className="text-gray-500">Configure how payments are routed across your gateways</p>
        </div>
        <button className="btn-primary flex items-center gap-2">
          <Plus className="w-4 h-4" />
          Create Policy
        </button>
      </div>

      {/* Error State */}
      {error && (
        <Card className="border-danger-200 bg-danger-50">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-danger-600" />
            <div className="flex-1">
              <p className="text-sm font-medium text-danger-800">Error loading routing policies</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load policies'}
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

      {/* Policy List */}
      <div className="space-y-4">
        {isLoading ? (
          // Loading skeletons
          Array.from({ length: 3 }).map((_, i) => (
            <Card key={i} className="animate-pulse">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-gray-200 rounded-lg"></div>
                  <div>
                    <div className="h-5 bg-gray-200 rounded w-32 mb-2"></div>
                    <div className="h-4 bg-gray-200 rounded w-48"></div>
                  </div>
                </div>
              </div>
            </Card>
          ))
        ) : policies && policies.length > 0 ? (
          policies.map((policy) => {
            const StrategyIcon = strategyIcons[policy.rotation_strategy] || GitBranch;
            const isToggling = toggleMutation.isPending && toggleMutation.variables?.id === policy.id;
            
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
                        <span>{strategyLabels[policy.rotation_strategy] || policy.rotation_strategy}</span>
                        <span>•</span>
                        <span>{policy.rules.length} rules</span>
                        {policy.activated_at && (
                          <>
                            <span>•</span>
                            <span>Activated {new Date(policy.activated_at).toLocaleDateString()}</span>
                          </>
                        )}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-3">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleTogglePolicy(policy);
                      }}
                      disabled={isToggling}
                      className={clsx(
                        'p-2 rounded-lg transition-colors',
                        isToggling && 'opacity-50 cursor-not-allowed',
                        policy.status === 'active'
                          ? 'text-success-600 hover:bg-success-50'
                          : 'text-gray-400 hover:bg-gray-100'
                      )}
                      title={policy.status === 'active' ? 'Deactivate' : 'Activate'}
                    >
                      {isToggling ? (
                        <Loader2 className="w-5 h-5 animate-spin" />
                      ) : policy.status === 'active' ? (
                        <Pause className="w-5 h-5" />
                      ) : (
                        <Play className="w-5 h-5" />
                      )}
                    </button>
                    <ChevronRight className="w-5 h-5 text-gray-400" />
                  </div>
                </div>

                {/* Policy Details (expanded view) */}
                {selectedPolicy === policy.id && (
                  <div className="mt-4 pt-4 border-t border-gray-100">
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                      <div>
                        <p className="text-sm text-gray-500">Strategy</p>
                        <p className="font-medium">{strategyLabels[policy.rotation_strategy]}</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-500">Max Hops</p>
                        <p className="font-medium">{policy.failover_config.max_hops}</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-500">Latency Budget</p>
                        <p className="font-medium">{policy.failover_config.latency_budget_ms}ms</p>
                      </div>
                      <div>
                        <p className="text-sm text-gray-500">Created</p>
                        <p className="font-medium">{new Date(policy.created_at).toLocaleDateString()}</p>
                      </div>
                    </div>
                    
                    {/* Rules Summary */}
                    {policy.rules.length > 0 && (
                      <div className="mt-4">
                        <h4 className="text-sm font-medium text-gray-700 mb-2">Routing Rules</h4>
                        <div className="space-y-2">
                          {policy.rules.map((rule, idx) => (
                            <div
                              key={rule.id}
                              className="flex items-center gap-3 p-2 bg-gray-50 rounded-lg text-sm"
                            >
                              <span className="text-gray-400">#{idx + 1}</span>
                              <span className="font-mono text-gray-600">{rule.gateway_profile_id}</span>
                              <span className="text-gray-400">Priority: {rule.priority}</span>
                              {rule.condition.card_schemes && (
                                <span className="text-gray-500">
                                  Cards: {rule.condition.card_schemes.join(', ')}
                                </span>
                              )}
                              {rule.condition.currencies && (
                                <span className="text-gray-500">
                                  Currencies: {rule.condition.currencies.join(', ')}
                                </span>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </Card>
            );
          })
        ) : (
          // Empty state
          <Card className="border-dashed border-2 border-gray-300 hover:border-primary-400 transition-colors cursor-pointer">
            <div className="text-center py-8">
              <Plus className="w-12 h-12 text-gray-400 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-gray-900">Create Your First Routing Policy</h3>
              <p className="text-gray-500 mt-1">
                Define routing rules to optimize your payment success rates
              </p>
              <button className="btn-primary mt-4">
                <Plus className="w-4 h-4 mr-2" />
                Create Policy
              </button>
            </div>
          </Card>
        )}
      </div>

      {/* Quick Stats */}
      {policies && policies.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Card>
            <div className="text-center">
              <p className="text-3xl font-bold text-gray-900">{policies.length}</p>
              <p className="text-sm text-gray-500">Total Policies</p>
            </div>
          </Card>
          <Card>
            <div className="text-center">
              <p className="text-3xl font-bold text-success-600">
                {policies.filter(p => p.status === 'active').length}
              </p>
              <p className="text-sm text-gray-500">Active Policies</p>
            </div>
          </Card>
          <Card>
            <div className="text-center">
              <p className="text-3xl font-bold text-gray-600">
                {policies.reduce((sum, p) => sum + p.rules.length, 0)}
              </p>
              <p className="text-sm text-gray-500">Total Rules</p>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
