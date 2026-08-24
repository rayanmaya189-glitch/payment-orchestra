import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  BarChart3,
  TrendingUp,
  Clock,
  Globe,
  CreditCard,
  ArrowUpRight,
  RefreshCw,
  AlertCircle,
} from 'lucide-react';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
} from 'recharts';
import { Card } from '@/components/ui/Card';
import { MetricCard } from '@/components/ui/MetricCard';
import { formatCurrency, formatPercentage, formatNumber } from '@/utils/format';
import { api, ApiError } from '@/services/api';

// Color palette for charts

export function AnalyticsPage() {
  const [timeRange, setTimeRange] = useState('7d');

  // Calculate date range based on selection
  const getDateRange = () => {
    const end = new Date().toISOString().split('T')[0];
    const days = timeRange === '24h' ? 1 : timeRange === '7d' ? 7 : timeRange === '30d' ? 30 : 90;
    const start = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString().split('T')[0];
    return { start, end };
  };

  const { start, end } = getDateRange();

  // Fetch analytics summary
  const {
    data: analytics,
    isLoading: analyticsLoading,
    error: analyticsError,
    refetch: refetchAnalytics,
  } = useQuery({
    queryKey: ['analytics-summary', start, end],
    queryFn: () => api.getAnalyticsSummary({
      start_date: start,
      end_date: end,
      granularity: timeRange === '24h' ? 'hour' : timeRange === '7d' ? 'day' : 'week',
    }),
    retry: 2,
    staleTime: 60000,
  });

  // Fetch gateway performance
  const {
    data: gatewayPerformance,
    isLoading: gatewayLoading,
  } = useQuery({
    queryKey: ['gateway-performance'],
    queryFn: () => api.getGatewayPerformance(),
    retry: 2,
    staleTime: 60000,
  });

  // Prepare volume by gateway data for chart
  const volumeByGateway = gatewayPerformance?.map(gw => ({
    name: gw.name,
    transactions: gw.transaction_count,
    revenue: gw.revenue_share,
    success_rate: gw.success_rate,
  })) || [];

  // Prepare payment methods data (would come from API in production)
  const paymentMethods = [
    { name: 'Visa', value: 45, color: '#1a1f71' },
    { name: 'Mastercard', value: 35, color: '#eb001b' },
    { name: 'Amex', value: 10, color: '#006fcf' },
    { name: 'Other', value: 10, color: '#9ca3af' },
  ];

  // Prepare status breakdown data
  const statusBreakdown = analytics?.breakdown_by_status 
    ? Object.entries(analytics.breakdown_by_status).map(([status, count]) => ({
        name: status.charAt(0).toUpperCase() + status.slice(1).replace('_', ' '),
        value: count,
      }))
    : [];

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Analytics</h1>
          <p className="text-gray-500">Insights into your payment performance</p>
        </div>
        <div className="flex items-center gap-3">
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(e.target.value)}
            className="px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value="24h">Last 24 hours</option>
            <option value="7d">Last 7 days</option>
            <option value="30d">Last 30 days</option>
            <option value="90d">Last 90 days</option>
          </select>
          <button
            onClick={() => refetchAnalytics()}
            disabled={analyticsLoading}
            className="btn-secondary flex items-center gap-2"
          >
            <RefreshCw className={`w-4 h-4 ${analyticsLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
      </div>

      {/* Error State */}
      {analyticsError && (
        <Card className="border-danger-200 bg-danger-50">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-danger-600" />
            <div>
              <p className="text-sm font-medium text-danger-800">Error loading analytics</p>
              <p className="text-sm text-danger-600">
                {analyticsError instanceof ApiError ? analyticsError.message : 'Failed to load analytics'}
              </p>
            </div>
          </div>
        </Card>
      )}

      {/* Metric Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        {analyticsLoading ? (
          // Loading skeletons
          Array.from({ length: 4 }).map((_, i) => (
            <Card key={i} className="animate-pulse">
              <div className="h-8 bg-gray-200 rounded w-1/3 mb-2"></div>
              <div className="h-6 bg-gray-200 rounded w-1/2"></div>
            </Card>
          ))
        ) : analytics ? (
          <>
            <MetricCard
              title="Total Volume"
              value={formatCurrency(analytics.total_volume, 'USD')}
              change={12.5}
              icon={BarChart3}
              color="primary"
            />
            <MetricCard
              title="Success Rate"
              value={formatPercentage(analytics.success_rate)}
              change={0.3}
              icon={TrendingUp}
              color="success"
            />
            <MetricCard
              title="Avg Processing Time"
              value={`${analytics.avg_latency_ms}ms`}
              change={-5.2}
              icon={Clock}
              color="info"
            />
            <MetricCard
              title="Total Transactions"
              value={formatNumber(analytics.total_transactions)}
              change={8.2}
              icon={Globe}
              color="primary"
            />
          </>
        ) : null}
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Volume by Gateway */}
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Volume by Gateway</h3>
          </div>
          <div className="h-64">
            {gatewayLoading ? (
              <div className="flex items-center justify-center h-full">
                <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
              </div>
            ) : volumeByGateway.length > 0 ? (
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={volumeByGateway}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                  <XAxis dataKey="name" stroke="#9ca3af" fontSize={12} />
                  <YAxis stroke="#9ca3af" fontSize={12} />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: '#fff',
                      border: '1px solid #e5e7eb',
                      borderRadius: '8px',
                    }}
                    formatter={(value: number) => [formatNumber(value), 'Transactions']}
                  />
                  <Bar dataKey="transactions" fill="#0ea5e9" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="flex items-center justify-center h-full text-gray-500">
                No gateway data available
              </div>
            )}
          </div>
        </Card>

        {/* Payment Methods Distribution */}
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Payment Methods</h3>
          </div>
          <div className="h-64 flex items-center justify-center">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={paymentMethods}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={100}
                  paddingAngle={5}
                  dataKey="value"
                >
                  {paymentMethods.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip
                  formatter={(value: number) => [`${value}%`, 'Share']}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div className="flex justify-center gap-6 mt-4">
            {paymentMethods.map((method) => (
              <div key={method.name} className="flex items-center gap-2">
                <div
                  className="w-3 h-3 rounded-full"
                  style={{ backgroundColor: method.color }}
                />
                <span className="text-sm text-gray-600">
                  {method.name} ({method.value}%)
                </span>
              </div>
            ))}
          </div>
        </Card>
      </div>

      {/* Status Breakdown */}
      {statusBreakdown.length > 0 && (
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Transaction Status Breakdown</h3>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {statusBreakdown.map((item) => (
              <div key={item.name} className="p-4 bg-gray-50 rounded-lg">
                <p className="text-2xl font-bold text-gray-900">{formatNumber(item.value)}</p>
                <p className="text-sm text-gray-500">{item.name}</p>
              </div>
            ))}
          </div>
        </Card>
      )}

      {/* Top Gateways Table */}
      <Card>
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-gray-900">Gateway Performance</h3>
          <a href="/connectors" className="text-sm text-primary-600 hover:text-primary-700 font-medium flex items-center gap-1">
            View all
            <ArrowUpRight className="w-4 h-4" />
          </a>
        </div>
        <div className="overflow-x-auto">
          {gatewayLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
            </div>
          ) : volumeByGateway.length > 0 ? (
            <table className="w-full">
              <thead>
                <tr className="text-left text-sm text-gray-500 border-b border-gray-100">
                  <th className="pb-3 font-medium">Gateway</th>
                  <th className="pb-3 font-medium">Transactions</th>
                  <th className="pb-3 font-medium">Revenue</th>
                  <th className="pb-3 font-medium">Success Rate</th>
                  <th className="pb-3 font-medium">Performance</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {volumeByGateway.map((gateway) => (
                  <tr key={gateway.name} className="hover:bg-gray-50">
                    <td className="py-4">
                      <div className="flex items-center gap-3">
                        <div className="w-10 h-10 bg-gray-100 rounded-lg flex items-center justify-center">
                          <CreditCard className="w-5 h-5 text-gray-600" />
                        </div>
                        <span className="font-medium text-gray-900">{gateway.name}</span>
                      </div>
                    </td>
                    <td className="py-4 text-sm text-gray-600">
                      {formatNumber(gateway.transactions)}
                    </td>
                    <td className="py-4 text-sm text-gray-900">
                      {formatCurrency(gateway.revenue, 'USD')}
                    </td>
                    <td className="py-4 text-sm text-gray-600">
                      {gateway.success_rate.toFixed(1)}%
                    </td>
                    <td className="py-4">
                      <div className="w-24 bg-gray-100 rounded-full h-2">
                        <div
                          className="bg-success-500 h-2 rounded-full"
                          style={{ width: `${Math.min(gateway.success_rate, 100)}%` }}
                        />
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-center py-12 text-gray-500">
              No gateway data available
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
