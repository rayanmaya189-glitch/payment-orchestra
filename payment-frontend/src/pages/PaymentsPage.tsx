import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Search, Filter, Download, RefreshCw } from 'lucide-react';
import { api } from '@/services/api';
import { Card } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { formatCurrency, formatRelativeTime } from '@/utils/format';

const mockPayments = [
  { id: 'pi_abc123def', amount: 9900, currency: 'USD', status: 'captured', gateway: 'Stripe', order_id: 'ORD-001', created_at: new Date(Date.now() - 120000) },
  { id: 'pi_ghi456jkl', amount: 14900, currency: 'AED', status: 'authorized', gateway: 'Network Intl', order_id: 'ORD-002', created_at: new Date(Date.now() - 300000) },
  { id: 'pi_mno789pqr', amount: 2499, currency: 'INR', status: 'failed', gateway: 'Razorpay', order_id: 'ORD-003', created_at: new Date(Date.now() - 480000) },
  { id: 'pi_stu012vwx', amount: 49900, currency: 'USD', status: 'captured', gateway: 'Checkout.com', order_id: 'ORD-004', created_at: new Date(Date.now() - 720000) },
  { id: 'pi_yza345bcd', amount: 7500, currency: 'EUR', status: 'pending', gateway: 'Adyen', order_id: 'ORD-005', created_at: new Date(Date.now() - 900000) },
  { id: 'pi_efg678hij', amount: 25000, currency: 'USD', status: 'captured', gateway: 'Stripe', order_id: 'ORD-006', created_at: new Date(Date.now() - 1200000) },
  { id: 'pi_klm901nop', amount: 3450, currency: 'GBP', status: 'refunded', gateway: 'Checkout.com', order_id: 'ORD-007', created_at: new Date(Date.now() - 1800000) },
  { id: 'pi_qrs234tuv', amount: 8900, currency: 'USD', status: 'captured', gateway: 'Stripe', order_id: 'ORD-008', created_at: new Date(Date.now() - 2400000) },
];

const statusFilters = ['All', 'Captured', 'Authorized', 'Pending', 'Failed', 'Refunded'];

export function PaymentsPage() {
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState('All');

  const filteredPayments = mockPayments.filter((payment) => {
    const matchesSearch =
      payment.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      payment.order_id?.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesStatus =
      statusFilter === 'All' || payment.status === statusFilter.toLowerCase();
    return matchesSearch && matchesStatus;
  });

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Payments</h1>
          <p className="text-gray-500">View and manage all payment transactions</p>
        </div>
        <div className="flex items-center gap-3">
          <button className="btn-secondary">
            <Download className="w-4 h-4 mr-2" />
            Export
          </button>
          <button className="btn-primary">
            <RefreshCw className="w-4 h-4 mr-2" />
            Refresh
          </button>
        </div>
      </div>

      {/* Filters */}
      <Card padding={false}>
        <div className="p-4 border-b border-gray-100">
          <div className="flex flex-col sm:flex-row gap-4">
            {/* Search */}
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
              <input
                type="text"
                placeholder="Search by ID or order reference..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-10 pr-4 py-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>

            {/* Status Filter */}
            <div className="flex items-center gap-2">
              <Filter className="w-5 h-5 text-gray-400" />
              <div className="flex gap-1">
                {statusFilters.map((status) => (
                  <button
                    key={status}
                    onClick={() => setStatusFilter(status)}
                    className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                      statusFilter === status
                        ? 'bg-primary-100 text-primary-700'
                        : 'text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    {status}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>

        {/* Table */}
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="text-left text-sm text-gray-500 bg-gray-50">
                <th className="px-4 py-3 font-medium">Payment ID</th>
                <th className="px-4 py-3 font-medium">Order</th>
                <th className="px-4 py-3 font-medium">Amount</th>
                <th className="px-4 py-3 font-medium">Status</th>
                <th className="px-4 py-3 font-medium">Gateway</th>
                <th className="px-4 py-3 font-medium">Time</th>
                <th className="px-4 py-3 font-medium">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {filteredPayments.map((payment) => (
                <tr
                  key={payment.id}
                  className="hover:bg-gray-50 transition-colors cursor-pointer"
                >
                  <td className="px-4 py-4 text-sm font-mono text-primary-600 hover:text-primary-700">
                    {payment.id}
                  </td>
                  <td className="px-4 py-4 text-sm text-gray-600">
                    {payment.order_id || '-'}
                  </td>
                  <td className="px-4 py-4 text-sm font-medium text-gray-900">
                    {formatCurrency(payment.amount, payment.currency)}
                  </td>
                  <td className="px-4 py-4">
                    <StatusBadge status={payment.status} />
                  </td>
                  <td className="px-4 py-4 text-sm text-gray-600">
                    {payment.gateway}
                  </td>
                  <td className="px-4 py-4 text-sm text-gray-500">
                    {formatRelativeTime(payment.created_at)}
                  </td>
                  <td className="px-4 py-4">
                    <button className="text-sm text-primary-600 hover:text-primary-700 font-medium">
                      View
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        <div className="px-4 py-3 border-t border-gray-100 flex items-center justify-between">
          <p className="text-sm text-gray-500">
            Showing {filteredPayments.length} of {mockPayments.length} payments
          </p>
          <div className="flex items-center gap-2">
            <button className="btn-secondary text-sm" disabled>
              Previous
            </button>
            <button className="btn-secondary text-sm">Next</button>
          </div>
        </div>
      </Card>
    </div>
  );
}
