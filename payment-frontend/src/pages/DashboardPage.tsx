import { useQuery } from '@tanstack/react-query';
import {
  TrendingUp,
  CreditCard,
  Activity,
  Plug,
  DollarSign,
  ArrowUpRight,
  AlertCircle,
  RefreshCw,
} from 'lucide-react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  AreaChart,
  Area,
} from 'recharts';
import { api, ApiError } from '@/services/api';
import { formatCurrency, formatNumber, formatPercentage } from '@/utils/format';
import { Card } from '@/components/ui/Card';
import { MetricCard } from '@/components/ui/MetricCard';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { useAppStore } from '@/store';

// Loading skeleton component
function LoadingSkeleton() {
  return (
    <div className="animate-pulse">
      <div className="h-8 bg-gray-200 rounded w-1/4 mb-4"></div>
      <div className="h-4 bg-gray-200 rounded w-1/2"></div>
    </div>
  );
}

// Error component
function ErrorDisplay({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <Card className="border-danger-200 bg-danger-50">
      <div className="flex items-center gap-3">
        <AlertCircle className="w-5 h-5 text-danger-600" />
        <div className="flex-1">
          <p className="text-sm font-medium text-danger-800">Error loading data</p>
          <p className="text-sm text-danger-600">{message}</p>
        </div>
        {onRetry && (
          <button
            onClick={onRetry}
            className="p-2 text-danger-600 hover:bg-danger-100 rounded-lg transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        )}
      </div>
    </Card>
  );
}

export function DashboardPage() {
  const { organization } = useAppStore();
  
  // Fetch dashboard metrics
  const {
    data: metrics,
    isLoading: metricsLoading,
    error: metricsError,
    refetch: refetchMetrics,
  } = useQuery({
    queryKey: ['dashboard-metrics'],
    queryFn: () => api.getDashboardMetrics(),
    retry: 2,
    staleTime: 30000, // 30 seconds
  });

  // Fetch daily metrics for chart (last 7 days)
  const {
    data: dailyMetrics,
    isLoading: dailyLoading,
    error: dailyError,
    refetch: refetchDaily,
  } = useQuery({
    queryKey: ['daily-metrics', '7d'],
    queryFn: () => {
      const end = new Date().toISOString().split('T')[0];
      const start = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0];
      return api.getDailyMetrics(start, end);
    },
    retry: 2,
    staleTime: 60000, // 1 minute
  });

  // Fetch gateway performance
  const {
    data: gatewayPerformance,
    isLoading: gatewayLoading,
    error: gatewayError,
    refetch: refetchGateway,
  } = useQuery({
    queryKey: ['gateway-performance'],
    queryFn: () => api.getGatewayPerformance(),
    retry: 2,
    staleTime: 60000,
  });

  // Fetch recent transactions
  const {
    data: recentTransactions,
    isLoading: transactionsLoading,
    error: transactionsError,
    refetch: refetchTransactions,
  } = useQuery({
    queryKey: ['recent-transactions'],
    queryFn: () => api.getRecentTransactions(5),
    retry: 2,
    staleTime: 15000, // 15 seconds for real-time feel
  });

  // Fetch transaction summary
  const {
    data: transactionSummary,
  } = useQuery({
    queryKey: ['transaction-summary', '7d'],
    queryFn: () => api.getTransactionSummary('7d'),
    retry: 2,
    staleTime: 60000,
  });

  // Loading state
  const isLoading = metricsLoading || dailyLoading || gatewayLoading || transactionsLoading;
  
  // Combined error state
  const error = metricsError || dailyError || gatewayError || transactionsError;
  const errorMessage = error instanceof ApiError 
    ? error.message 
    : error instanceof Error 
      ? error.message 
      : 'An unexpected error occurred';

  // Retry all queries
  const handleRetry = () => {
    refetchMetrics();
    refetchDaily();
    refetchGateway();
    refetchTransactions();
  };

  // Format chart data
  const chartData = dailyMetrics?.map((m) => ({
    date: new Date(m.date).toLocaleDateString('en-US', { weekday: 'short' }),
    amount: m.amount / 100, // Convert from cents
    count: m.count,
    success_rate: m.success_rate,
  })) || [];

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>
          <p className="text-gray-500">
            Welcome back! Here's what's happening with your payments.
            {organization && (
              <span className="ml-2 text-sm text-gray-400">
                ({organization.name})
              </span>
            )}
          </p>
        </div>
        <button
          onClick={handleRetry}
          disabled={isLoading}
          className="btn-secondary flex items-center gap-2"
        >
          <RefreshCw className={`w-4 h-4 ${isLoading ? 'animate-spin' : ''}`} />
          Refresh
        </button>
      </div>

      {/* Error State */}
      {error && !isLoading && (
        <ErrorDisplay message={errorMessage} onRetry={handleRetry} />
      )}

      {/* Metric Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {metricsLoading ? (
          // Loading skeletons
          Array.from({ length: 4 }).map((_, i) => (
            <Card key={i}>
              <LoadingSkeleton />
            </Card>
          ))
        ) : metrics ? (
          <>
            <MetricCard
              title="Total Transactions"
              value={formatNumber(metrics.total_transactions)}
              change={transactionSummary?.total_count ? 
                ((metrics.total_transactions - transactionSummary.total_count) / transactionSummary.total_count * 100) : 0}
              icon={CreditCard}
              color="primary"
            />
            <MetricCard
              title="Success Rate"
              value={formatPercentage(metrics.success_rate)}
              change={0.3} // Would compare to previous period
              icon={Activity}
              color="success"
            />
            <MetricCard
              title="Total Volume"
              value={formatCurrency(metrics.total_volume, 'USD')}
              change={8.2}
              icon={DollarSign}
              color="primary"
            />
            <MetricCard
              title="Active Gateways"
              value={String(metrics.active_gateways)}
              change={0}
              icon={Plug}
              color="info"
            />
          </>
        ) : null}
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Transaction Volume Chart */}
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Transaction Volume</h3>
            <select className="text-sm border border-gray-200 rounded-lg px-3 py-1.5 focus:outline-none focus:ring-2 focus:ring-primary-500">
              <option>Last 7 days</option>
              <option>Last 30 days</option>
              <option>Last 90 days</option>
            </select>
          </div>
          <div className="h-64">
            {dailyLoading ? (
              <div className="flex items-center justify-center h-full">
                <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
              </div>
            ) : (
              <ResponsiveContainer width="100%" height="100%">
                <AreaChart data={chartData}>
                  <defs>
                    <linearGradient id="colorVolume" x1="0" y1="0" x2="0" y2="1">
                      <stop offset="5%" stopColor="#0ea5e9" stopOpacity={0.1} />
                      <stop offset="95%" stopColor="#0ea5e9" stopOpacity={0} />
                    </linearGradient>
                  </defs>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                  <XAxis dataKey="date" stroke="#9ca3af" fontSize={12} />
                  <YAxis stroke="#9ca3af" fontSize={12} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#fff',
                      border: '1px solid #e5e7eb',
                      borderRadius: '8px',
                      boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.1)',
                    }}
                    formatter={(value: number) => [`$${value.toLocaleString()}`, 'Volume']}
                  />
                  <Area
                    type="monotone"
                    dataKey="amount"
                    stroke="#0ea5e9"
                    strokeWidth={2}
                    fillOpacity={1}
                    fill="url(#colorVolume)"
                  />
                </AreaChart>
              </ResponsiveContainer>
            )}
          </div>
        </Card>

        {/* Success Rate Chart */}
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Success Rate</h3>
            {metrics && (
              <span className="text-sm text-success-600 font-medium flex items-center gap-1">
                <TrendingUp className="w-4 h-4" />
                {metrics.success_rate >= 97 ? '+' : ''}{(metrics.success_rate - 97).toFixed(1)}%
              </span>
            )}
          </div>
          <div className="h-64">
            {dailyLoading ? (
              <div className="flex items-center justify-center h-full">
                <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
              </div>
            ) : (
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                  <XAxis dataKey="date" stroke="#9ca3af" fontSize={12} />
                  <YAxis
                    stroke="#9ca3af"
                    fontSize={12}
                    domain={[95, 100]}
                    tickFormatter={(value) => `${value}%`}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#fff',
                      border: '1px solid #e5e7eb',
                      borderRadius: '8px',
                      boxShadow: '0 4px 6px -1px rgb(0 0 0 / 0.1)',
                    }}
                    formatter={(value: number) => [`${value.toFixed(1)}%`, 'Success Rate']}
                  />
                  <Line
                    type="monotone"
                    dataKey="success_rate"
                    stroke="#22c55e"
                    strokeWidth={2}
                    dot={{ fill: '#22c55e', strokeWidth: 2 }}
                    activeDot={{ r: 6 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            )}
          </div>
        </Card>
      </div>

      {/* Bottom Row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Recent Transactions */}
        <Card className="lg:col-span-2">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Recent Transactions</h3>
            <a
              href="/payments"
              className="text-sm text-primary-600 hover:text-primary-700 font-medium flex items-center gap-1"
            >
              View all
              <ArrowUpRight className="w-4 h-4" />
            </a>
          </div>
          <div className="overflow-x-auto">
            {transactionsLoading ? (
              <div className="flex items-center justify-center py-8">
                <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
              </div>
            ) : recentTransactions && recentTransactions.length > 0 ? (
              <table className="w-full">
                <thead>
                  <tr className="text-left text-sm text-gray-500 border-b border-gray-100">
                    <th className="pb-3 font-medium hidden sm:table-cell">ID</th>
                    <th className="pb-3 font-medium">Amount</th>
                    <th className="pb-3 font-medium">Status</th>
                    <th className="pb-3 font-medium hidden md:table-cell">Gateway</th>
                    <th className="pb-3 font-medium hidden lg:table-cell">Time</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {recentTransactions.map((tx) => (
                    <tr key={tx.id} className="hover:bg-gray-50 transition-colors">
                      <td className="py-3 text-sm font-mono text-gray-900 hidden sm:table-cell">{tx.id.slice(0, 12)}...</td>
                      <td className="py-3 text-sm text-gray-900">
                        {formatCurrency(tx.amount, tx.currency)}
                      </td>
                      <td className="py-3">
                        <StatusBadge status={tx.status} />
                      </td>
                      <td className="py-3 text-sm text-gray-600 hidden md:table-cell">{tx.gateway_profile_id || '-'}</td>
                      <td className="py-3 text-sm text-gray-500 hidden lg:table-cell">
                        {new Date(tx.created_at).toLocaleString()}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : (
              <div className="text-center py-8 text-gray-500">
                No transactions yet
              </div>
            )}
          </div>
        </Card>

        {/* Gateway Performance */}
        <Card>
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Gateway Performance</h3>
          <div className="space-y-4">
            {gatewayLoading ? (
              <div className="flex items-center justify-center py-8">
                <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
              </div>
            ) : gatewayPerformance && gatewayPerformance.length > 0 ? (
              gatewayPerformance.map((gw) => (
                <div key={gw.gateway_id} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium text-gray-700">{gw.name}</span>
                    <span className="text-sm text-gray-500">
                      {formatPercentage(gw.success_rate)}
                    </span>
                  </div>
                  <div className="w-full bg-gray-100 rounded-full h-2">
                    <div
                      className="bg-primary-500 h-2 rounded-full transition-all duration-500"
                      style={{ width: `${Math.min(gw.success_rate, 100)}%` }}
                    />
                  </div>
                  <div className="flex justify-between text-xs text-gray-400">
                    <span>{gw.avg_latency_ms}ms avg latency</span>
                    <span>{formatNumber(gw.transaction_count)} txns</span>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-center py-8 text-gray-500">
                No gateway data available
              </div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
