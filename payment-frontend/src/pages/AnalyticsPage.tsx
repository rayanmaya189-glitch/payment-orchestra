import { useState } from 'react';
import {
  BarChart3,
  TrendingUp,
  Clock,
  Globe,
  CreditCard,
  ArrowUpRight,
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
import { Card, CardHeader } from '@/components/ui/Card';
import { MetricCard } from '@/components/ui/MetricCard';
import { formatCurrency, formatPercentage } from '@/utils/format';

const mockVolumeByCurrency = [
  { currency: 'USD', amount: 450000 },
  { currency: 'AED', amount: 280000 },
  { currency: 'INR', amount: 180000 },
  { currency: 'EUR', amount: 120000 },
  { currency: 'GBP', amount: 80000 },
];

const mockPaymentMethods = [
  { name: 'Visa', value: 45, color: '#1a1f71' },
  { name: 'Mastercard', value: 35, color: '#eb001b' },
  { name: 'Amex', value: 10, color: '#006fcf' },
  { name: 'Other', value: 10, color: '#9ca3af' },
];

const mockTopGateways = [
  { name: 'Stripe', transactions: 3200, revenue: 320000, success_rate: 98.5 },
  { name: 'Checkout.com', transactions: 2100, revenue: 210000, success_rate: 97.8 },
  { name: 'Network Intl', transactions: 1800, revenue: 180000, success_rate: 96.2 },
  { name: 'Razorpay', transactions: 1200, revenue: 120000, success_rate: 95.8 },
];

export function AnalyticsPage() {
  const [timeRange, setTimeRange] = useState('7d');

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Analytics</h1>
          <p className="text-gray-500">Insights into your payment performance</p>
        </div>
        <select
          value={timeRange}
          onChange={(e) => setTimeRange(e.target.value)}
          className="input"
        >
          <option value="24h">Last 24 hours</option>
          <option value="7d">Last 7 days</option>
          <option value="30d">Last 30 days</option>
          <option value="90d">Last 90 days</option>
        </select>
      </div>

      {/* Metric Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <MetricCard
          title="Total Volume"
          value={formatCurrency(1110000, 'USD')}
          change={12.5}
          icon={BarChart3}
          color="primary"
        />
        <MetricCard
          title="Success Rate"
          value={formatPercentage(97.8)}
          change={0.3}
          icon={TrendingUp}
          color="success"
        />
        <MetricCard
          title="Avg Processing Time"
          value="142ms"
          change={-5.2}
          icon={Clock}
          color="info"
        />
        <MetricCard
          title="Active Currencies"
          value="5"
          change={0}
          icon={Globe}
          color="primary"
        />
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Volume by Currency */}
        <Card>
          <CardHeader title="Volume by Currency" />
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={mockVolumeByCurrency}>
                <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
                <XAxis dataKey="currency" stroke="#9ca3af" fontSize={12} />
                <YAxis stroke="#9ca3af" fontSize={12} />
                <Tooltip
                  contentStyle={{
                    backgroundColor: '#fff',
                    border: '1px solid #e5e7eb',
                    borderRadius: '8px',
                  }}
                  formatter={(value: number) => [formatCurrency(value, 'USD'), 'Volume']}
                />
                <Bar dataKey="amount" fill="#0ea5e9" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </Card>

        {/* Payment Methods Distribution */}
        <Card>
          <CardHeader title="Payment Methods" />
          <div className="h-64 flex items-center justify-center">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={mockPaymentMethods}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={100}
                  paddingAngle={5}
                  dataKey="value"
                >
                  {mockPaymentMethods.map((entry, index) => (
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
            {mockPaymentMethods.map((method) => (
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

      {/* Top Gateways */}
      <Card>
        <CardHeader
          title="Gateway Performance"
          action={
            <a href="/connectors" className="text-sm text-primary-600 hover:text-primary-700 font-medium flex items-center gap-1">
              View all
              <ArrowUpRight className="w-4 h-4" />
            </a>
          }
        />
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="text-left text-sm text-gray-500 border-b border-gray-100">
                <th className="pb-3 font-medium">Gateway</th>
                <th className="pb-3 font-medium">Transactions</th>
                <th className="pb-3 font-medium">Volume</th>
                <th className="pb-3 font-medium">Success Rate</th>
                <th className="pb-3 font-medium">Performance</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {mockTopGateways.map((gateway) => (
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
                    {gateway.transactions.toLocaleString()}
                  </td>
                  <td className="py-4 text-sm text-gray-900">
                    {formatCurrency(gateway.revenue, 'USD')}
                  </td>
                  <td className="py-4 text-sm text-gray-600">
                    {gateway.success_rate}%
                  </td>
                  <td className="py-4">
                    <div className="w-24 bg-gray-100 rounded-full h-2">
                      <div
                        className="bg-success-500 h-2 rounded-full"
                        style={{ width: `${gateway.success_rate}%` }}
                      />
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}
