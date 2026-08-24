import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { 
  RefreshCw, 
  AlertCircle, 
  CheckCircle, 
  XCircle, 
  Clock,
  Filter,
  Download,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { formatCurrency, formatNumber } from '@/utils/format';
import toast from 'react-hot-toast';

const statusFilters = ['All', 'Pending', 'Matched', 'Unmatched', 'Resolved'];

export function ReconciliationPage() {
  const [statusFilter, setStatusFilter] = useState('All');
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  
  const queryClient = useQueryClient();

  // Fetch reconciliation exceptions
  const {
    data: exceptionsData,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['reconciliation-exceptions', statusFilter, page],
    queryFn: () => api.listReconciliationExceptions({
      status: statusFilter === 'All' ? undefined : statusFilter.toLowerCase(),
      limit: pageSize,
      offset: (page - 1) * pageSize,
    }),
    retry: 2,
  });

  // Resolve exception mutation
  const resolveMutation = useMutation({
    mutationFn: ({ id, resolution, notes }: { id: string; resolution: string; notes?: string }) =>
      api.resolveReconciliationException(id, { resolution, notes }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['reconciliation-exceptions'] });
      toast.success('Exception resolved');
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to resolve exception');
    },
  });

  const exceptions = exceptionsData?.items || [];
  const totalExceptions = exceptionsData?.total || 0;
  const totalPages = Math.ceil(totalExceptions / pageSize);

  const handleResolve = (id: string, resolution: string) => {
    const notes = window.prompt('Add notes (optional):');
    resolveMutation.mutate({ id, resolution, notes: notes || undefined });
  };

  const statusIcon = (status: string) => {
    switch (status) {
      case 'matched':
        return <CheckCircle className="w-5 h-5 text-success-500" />;
      case 'unmatched':
        return <XCircle className="w-5 h-5 text-danger-500" />;
      case 'pending':
        return <Clock className="w-5 h-5 text-warning-500" />;
      case 'resolved':
        return <CheckCircle className="w-5 h-5 text-primary-500" />;
      default:
        return <Clock className="w-5 h-5 text-gray-400" />;
    }
  };

  const statusColor = (status: string) => {
    switch (status) {
      case 'matched':
        return 'bg-success-100 text-success-700';
      case 'unmatched':
        return 'bg-danger-100 text-danger-700';
      case 'pending':
        return 'bg-warning-100 text-warning-700';
      case 'resolved':
        return 'bg-primary-100 text-primary-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Reconciliation</h1>
          <p className="text-gray-500">Settlement matching and exception management</p>
        </div>
        <div className="flex items-center gap-3">
          <button className="btn-secondary flex items-center gap-2">
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
            <div className="flex-1">
              <p className="text-sm font-medium text-danger-800">Error loading reconciliation data</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load data'}
              </p>
            </div>
          </div>
        </Card>
      )}

      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        <Card>
          <div className="text-center">
            <p className="text-3xl font-bold text-gray-900">{formatNumber(totalExceptions)}</p>
            <p className="text-sm text-gray-500">Total Exceptions</p>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <p className="text-3xl font-bold text-warning-600">
              {exceptions.filter(e => e.status === 'pending').length}
            </p>
            <p className="text-sm text-gray-500">Pending Review</p>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <p className="text-3xl font-bold text-danger-600">
              {exceptions.filter(e => e.status === 'unmatched').length}
            </p>
            <p className="text-sm text-gray-500">Unmatched</p>
          </div>
        </Card>
        <Card>
          <div className="text-center">
            <p className="text-3xl font-bold text-success-600">
              {exceptions.filter(e => e.status === 'resolved').length}
            </p>
            <p className="text-sm text-gray-500">Resolved</p>
          </div>
        </Card>
      </div>

      {/* Filters */}
      <Card padding={false}>
        <div className="p-4 border-b border-gray-100">
          <div className="flex items-center gap-2">
            <Filter className="w-5 h-5 text-gray-400" />
            <div className="flex gap-1">
              {statusFilters.map((status) => (
                <button
                  key={status}
                  onClick={() => {
                    setStatusFilter(status);
                    setPage(1);
                  }}
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

        {/* Table */}
        <div className="overflow-x-auto">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
            </div>
          ) : exceptions.length > 0 ? (
            <table className="w-full">
              <thead>
                <tr className="text-left text-sm text-gray-500 bg-gray-50">
                  <th className="px-4 py-3 font-medium">Status</th>
                  <th className="px-4 py-3 font-medium">Type</th>
                  <th className="px-4 py-3 font-medium">Amount</th>
                  <th className="px-4 py-3 font-medium">Description</th>
                  <th className="px-4 py-3 font-medium">Date</th>
                  <th className="px-4 py-3 font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {exceptions.map((exception) => (
                  <tr key={exception.id} className="hover:bg-gray-50">
                    <td className="px-4 py-4">
                      <div className="flex items-center gap-2">
                        {statusIcon(exception.status)}
                        <span className={`px-2 py-0.5 text-xs font-medium rounded ${statusColor(exception.status)}`}>
                          {exception.status}
                        </span>
                      </div>
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-600">
                      {exception.type}
                    </td>
                    <td className="px-4 py-4 text-sm font-medium text-gray-900">
                      {formatCurrency(exception.amount, exception.currency)}
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-600 max-w-xs truncate">
                      {exception.description}
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-500">
                      {new Date(exception.created_at).toLocaleDateString()}
                    </td>
                    <td className="px-4 py-4">
                      {exception.status !== 'resolved' && (
                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => handleResolve(exception.id, 'matched')}
                            disabled={resolveMutation.isPending}
                            className="text-sm text-success-600 hover:text-success-700 font-medium"
                          >
                            Match
                          </button>
                          <button
                            onClick={() => handleResolve(exception.id, 'dismissed')}
                            disabled={resolveMutation.isPending}
                            className="text-sm text-danger-600 hover:text-danger-700 font-medium"
                          >
                            Dismiss
                          </button>
                        </div>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-center py-12">
              <CheckCircle className="w-12 h-12 text-success-300 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-gray-900">All reconciled!</h3>
              <p className="text-gray-500 mt-1">No exceptions to review</p>
            </div>
          )}
        </div>

        {/* Pagination */}
        {totalExceptions > 0 && (
          <div className="px-4 py-3 border-t border-gray-100 flex items-center justify-between">
            <p className="text-sm text-gray-500">
              Showing {Math.min((page - 1) * pageSize + 1, totalExceptions)} to{' '}
              {Math.min(page * pageSize, totalExceptions)} of {totalExceptions} exceptions
            </p>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page === 1}
                className="btn-secondary text-sm"
              >
                Previous
              </button>
              <span className="text-sm text-gray-600">
                Page {page} of {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page === totalPages}
                className="btn-secondary text-sm"
              >
                Next
              </button>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
