import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Search, Filter, Download, RefreshCw, Eye, ChevronLeft, ChevronRight, AlertCircle } from 'lucide-react';
import { api, ApiError } from '@/services/api';
import { Card } from '@/components/ui/Card';
import { StatusBadge } from '@/components/ui/StatusBadge';
import { formatCurrency } from '@/utils/format';
import type { PaymentIntent } from '@/types';

const statusFilters = ['All', 'Created', 'Authorizing', 'Authorized', 'Captured', 'Failed', 'Refunded'];

export function PaymentsPage() {
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState('All');
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);

  // Build query params
  const queryParams = {
    limit: pageSize,
    offset: (page - 1) * pageSize,
    ...(statusFilter !== 'All' && { status: statusFilter.toLowerCase() }),
    ...(searchQuery && { search: searchQuery }),
  };

  // Fetch payments
  const {
    data: paymentsData,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['payments', queryParams],
    queryFn: () => api.listPaymentIntents(queryParams),
    placeholderData: (previousData) => previousData,
  });

  const payments = (paymentsData as any)?.items || [];
  const totalPayments = (paymentsData as any)?.total || 0;
  const totalPages = Math.ceil(totalPayments / pageSize);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
    setPage(1); // Reset to first page on search
  };

  const handleStatusFilter = (status: string) => {
    setStatusFilter(status);
    setPage(1); // Reset to first page on filter change
  };

  const handleExport = async () => {
    // Fetch all payments for export (up to 1000)
    const exportData = await api.listPaymentIntents({
      limit: 1000,
      offset: 0,
      ...(statusFilter !== 'All' && { status: statusFilter.toLowerCase() }),
      ...(searchQuery && { search: searchQuery }),
    }) as any;
    const items = exportData?.items || [];

    if (items.length === 0) {
      return;
    }

    // Build CSV
    const headers = ['ID', 'Amount', 'Currency', 'Status', 'Connector', 'Created At'];
    const rows = items.map((p: any) => [
      p.id,
      (p.amount_minor / 100).toFixed(2),
      p.currency,
      p.status,
      p.connector || '',
      p.created_at,
    ]);

    const csv = [headers.join(','), ...rows.map((r: string[]) => r.join(','))].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `payments-${new Date().toISOString().split('T')[0]}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Payments</h1>
          <p className="text-gray-500">View and manage all payment transactions</p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleExport}
            className="btn-secondary flex items-center gap-2"
          >
            <Download className="w-4 h-4" />
            Export
          </button>
          <button
            onClick={() => refetch()}
            disabled={isLoading}
            className="btn-secondary flex items-center gap-2"
          >
            <RefreshCw className={`w-4 h-4 ${isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
      </div>

      {/* Error State */}
      {error && (
        <Card className="border-danger-200 bg-danger-50">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-danger-600" />
            <div>
              <p className="text-sm font-medium text-danger-800">Error loading payments</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load payments'}
              </p>
            </div>
          </div>
        </Card>
      )}

      {/* Filters */}
      <Card padding={false}>
        <div className="p-4 border-b border-gray-100">
          <div className="flex flex-col sm:flex-row gap-4">
            {/* Search */}
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" />
              <input
                type="text"
                placeholder="Search by payment ID, order reference, or amount..."
                value={searchQuery}
                onChange={(e) => handleSearch(e.target.value)}
                className="w-full pl-10 pr-4 py-2 bg-gray-50 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>

            {/* Status Filter */}
            <div className="flex items-center gap-2">
              <Filter className="w-5 h-5 text-gray-400" />
              <div className="flex gap-1 flex-wrap">
                {statusFilters.map((status) => (
                  <button
                    key={status}
                    onClick={() => handleStatusFilter(status)}
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
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
            </div>
          ) : payments.length > 0 ? (
            <table className="w-full">
              <thead>
                <tr className="text-left text-sm text-gray-500 bg-gray-50">
                  <th className="px-4 py-3 font-medium">Payment ID</th>
                  <th className="px-4 py-3 font-medium">Order</th>
                  <th className="px-4 py-3 font-medium">Amount</th>
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Gateway</th>
                  <th className="px-4 py-3 font-medium">Risk</th>
                  <th className="px-4 py-3 font-medium">Time</th>
                  <th className="px-4 py-3 font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {payments.map((payment: PaymentIntent) => (
                  <tr
                    key={payment.id}
                    className="hover:bg-gray-50 transition-colors"
                  >
                    <td className="px-4 py-4 text-sm font-mono text-primary-600 hover:text-primary-700 cursor-pointer">
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
                      {payment.gateway_profile_id || '-'}
                    </td>
                    <td className="px-4 py-4 text-sm">
                      {payment.risk_score !== undefined ? (
                        <span
                          className={`font-medium ${
                            payment.risk_score < 30
                              ? 'text-success-600'
                              : payment.risk_score < 70
                              ? 'text-warning-600'
                              : 'text-danger-600'
                          }`}
                        >
                          {payment.risk_score.toFixed(1)}%
                        </span>
                      ) : (
                        '-'
                      )}
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-500">
                      {new Date(payment.created_at).toLocaleString()}
                    </td>
                    <td className="px-4 py-4">
                      <button className="text-sm text-primary-600 hover:text-primary-700 font-medium flex items-center gap-1">
                        <Eye className="w-4 h-4" />
                        View
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-center py-12 text-gray-500">
              {searchQuery || statusFilter !== 'All' ? (
                <>
                  <p className="text-lg font-medium">No payments found</p>
                  <p className="text-sm mt-1">Try adjusting your search or filters</p>
                </>
              ) : (
                <>
                  <p className="text-lg font-medium">No payments yet</p>
                  <p className="text-sm mt-1">Payments will appear here once you start processing transactions</p>
                </>
              )}
            </div>
          )}
        </div>

        {/* Pagination */}
        {totalPayments > 0 && (
          <div className="px-4 py-3 border-t border-gray-100 flex items-center justify-between">
            <p className="text-sm text-gray-500">
              Showing {Math.min((page - 1) * pageSize + 1, totalPayments)} to{' '}
              {Math.min(page * pageSize, totalPayments)} of {totalPayments} payments
            </p>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page === 1}
                className="btn-secondary text-sm flex items-center gap-1"
              >
                <ChevronLeft className="w-4 h-4" />
                Previous
              </button>
              <span className="text-sm text-gray-600">
                Page {page} of {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page === totalPages}
                className="btn-secondary text-sm flex items-center gap-1"
              >
                Next
                <ChevronRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
