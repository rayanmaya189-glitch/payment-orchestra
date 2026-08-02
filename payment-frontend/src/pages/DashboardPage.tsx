import { useQuery } from '@tanstack/react-query';
import {
  TrendingUp,
  TrendingDown,
  CreditCard,
  Activity,
  Plug,
  DollarSign,
  ArrowUpRight,
  ArrowDownRight,
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
import { api } from '@/services/api';
import { formatCurrency, formatNumber, formatPercentage } from '@/utils/format';
import { Card } from '@/components/ui/Card';
import { MetricCard } from '@/components/ui/MetricCard';
import { StatusBadge } from '@/components/ui/StatusBadge';

// Mock data for demo
const mockDailyMetrics = [
  { date: 'Mon', count: 1250, amount: 125000, success_rate: 98.2 },
  { date: 'Tue', count: 1380, amount: 138000, success_rate: 97.8 },
  { date: 'Wed', count: 1420, amount: 142000, success_rate: 98.5 },
  { date: 'Thu', count: 1180, amount: 118000, success_rate: 96.9 },
  { date: 'Fri', count: 1560, amount: 156000, success_rate: 98.1 },
  { date: 'Sat', count: 890, amount: 89000, success_rate: 97.5 },
  { date: 'Sun', count: 720, amount: 72000, success_rate: 98.8 },
];

const mockRecentTransactions = [
  { id: 'pi_abc123', amount: 9900, currency: 'USD', status: 'captured', gateway: 'Stripe', created_at: '2 min ago' },
  { id: 'pi_def456', amount: 14900, currency: 'AED', status: 'authorized', gateway: 'Network Intl', created_at: '5 min ago' },
  { id: 'pi_ghi789', amount: 2499, currency: 'INR', status: 'failed', gateway: 'Razorpay', created_at: '8 min ago' },
  { id: 'pi_jkl012', amount: 49900, currency: 'USD', status: 'captured', gateway: 'Checkout.com', created_at: '12 min ago' },
  { id: 'pi_mno345', amount: 7500, currency: 'EUR', status: 'pending', gateway: 'Adyen', created_at: '15 min ago' },
];

const mockGatewayPerformance = [
  { name: 'Stripe', success_rate: 98.5, latency: 120, volume: 45000 },
  { name: 'Checkout.com', success_rate: 97.8, latency: 145, volume: 32000 },
  { name: 'Network Intl', success_rate: 96.2, latency: 180, volume: 28000 },
  { name: 'Razorpay', success_rate: 95.8, latency: 210, volume: 18000 },
  { name: 'Adyen', success_rate: 98.1, latency: 135, volume: 15000 },
];

export function DashboardPage() {
  // In production, these would fetch from the API
  // const { data: metrics } = useQuery(['dashboard-metrics'], api.getDashboardMetrics);
  // const { data: dailyMetrics } = useQuery(['daily-metrics'], () => api.getDailyMetrics('7d'));

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>
        <p className="text-gray-500">Welcome back! Here's what's happening with your payments.</p>
      </div>

      {/* Metric Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <MetricCard
          title="Total Transactions"
          value={formatNumber(8400)}
          change={12.5}
          icon={CreditCard}
          color="primary"
        />
        <MetricCard
          title="Success Rate"
          value={formatPercentage(97.8)}
          change={0.3}
          icon={Activity}
          color="success"
        />
        <MetricCard
          title="Total Volume"
          value={formatCurrency(840000, 'USD')}
          change={8.2}
          icon={DollarSign}
          color="primary"
        />
        <MetricCard
          title="Active Gateways"
          value="5"
          change={0}
          icon={Plug}
          color="info"
        />
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
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={mockDailyMetrics}>
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
          </div>
        </Card>

        {/* Success Rate Chart */}
        <Card>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-gray-900">Success Rate</h3>
            <span className="text-sm text-success-600 font-medium flex items-center gap-1">
              <TrendingUp className="w-4 h-4" />
              +0.3%
            </span>
          </div>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={mockDailyMetrics}>
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
                  formatter={(value: number) => [`${value}%`, 'Success Rate']}
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
            <table className="w-full">
              <thead>
                <tr className="text-left text-sm text-gray-500 border-b border-gray-100">
                  <th className="pb-3 font-medium">ID</th>
                  <th className="pb-3 font-medium">Amount</th>
                  <th className="pb-3 font-medium">Status</th>
                  <th className="pb-3 font-medium">Gateway</th>
                  <th className="pb-3 font-medium">Time</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {mockRecentTransactions.map((tx) => (
                  <tr key={tx.id} className="hover:bg-gray-50 transition-colors">
                    <td className="py-3 text-sm font-mono text-gray-900">{tx.id}</td>
                    <td className="py-3 text-sm text-gray-900">
                      {formatCurrency(tx.amount, tx.currency)}
                    </td>
                    <td className="py-3">
                      <StatusBadge status={tx.status} />
                    </td>
                    <td className="py-3 text-sm text-gray-600">{tx.gateway}</td>
                    <td className="py-3 text-sm text-gray-500">{tx.created_at}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>

        {/* Gateway Performance */}
        <Card>
          <h3 className="text-lg font-semibold text-gray-900 mb-4">Gateway Performance</h3>
          <div className="space-y-4">
            {mockGatewayPerformance.map((gw) => (
              <div key={gw.name} className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-700">{gw.name}</span>
                  <span className="text-sm text-gray-500">{gw.success_rate}%</span>
                </div>
                <div className="w-full bg-gray-100 rounded-full h-2">
                  <div
                    className="bg-primary-500 h-2 rounded-full transition-all duration-500"
                    style={{ width: `${gw.success_rate}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
}
