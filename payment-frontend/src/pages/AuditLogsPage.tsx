import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { 
  RefreshCw, 
  AlertCircle, 
  Filter,
  Download,
  Clock,
  User,
  Shield,
  Activity,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import { formatRelativeTime } from '@/utils/format';

const actionFilters = ['All', 'Create', 'Update', 'Delete', 'Login', 'Export'];
const resourceFilters = ['All', 'Payment', 'Gateway', 'API Key', 'Webhook', 'User'];

export function AuditLogsPage() {
  const [actionFilter, setActionFilter] = useState('All');
  const [resourceFilter, setResourceFilter] = useState('All');
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');

  // Fetch audit logs
  const {
    data: logsData,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['audit-logs', actionFilter, resourceFilter, page, startDate, endDate],
    queryFn: () => api.listAuditLogs({
      action: actionFilter === 'All' ? undefined : actionFilter.toLowerCase(),
      resource: resourceFilter === 'All' ? undefined : resourceFilter.toLowerCase(),
      limit: pageSize,
      offset: (page - 1) * pageSize,
      start_date: startDate || undefined,
      end_date: endDate || undefined,
    }),
    retry: 2,
  });

  const logs = logsData?.items || [];
  const totalLogs = logsData?.total || 0;
  const totalPages = Math.ceil(totalLogs / pageSize);

  const actionIcon = (action: string) => {
    if (action.includes('create')) return <Activity className="w-4 h-4 text-success-500" />;
    if (action.includes('update')) return <Activity className="w-4 h-4 text-primary-500" />;
    if (action.includes('delete')) return <Activity className="w-4 h-4 text-danger-500" />;
    if (action.includes('login')) return <Shield className="w-4 h-4 text-warning-500" />;
    return <Activity className="w-4 h-4 text-gray-400" />;
  };

  const actionColor = (action: string) => {
    if (action.includes('create')) return 'bg-success-100 text-success-700';
    if (action.includes('update')) return 'bg-primary-100 text-primary-700';
    if (action.includes('delete')) return 'bg-danger-100 text-danger-700';
    if (action.includes('login')) return 'bg-warning-100 text-warning-700';
    return 'bg-gray-100 text-gray-700';
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Audit Logs</h1>
          <p className="text-gray-500">Track all system activities and changes</p>
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
              <p className="text-sm font-medium text-danger-800">Error loading audit logs</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load logs'}
              </p>
            </div>
          </div>
        </Card>
      )}

      {/* Filters */}
      <Card padding={false}>
        <div className="p-4 border-b border-gray-100">
          <div className="flex flex-col lg:flex-row gap-4">
            {/* Action Filter */}
            <div className="flex items-center gap-2">
              <Filter className="w-5 h-5 text-gray-400" />
              <span className="text-sm text-gray-600">Action:</span>
              <div className="flex gap-1">
                {actionFilters.map((action) => (
                  <button
                    key={action}
                    onClick={() => {
                      setActionFilter(action);
                      setPage(1);
                    }}
                    className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                      actionFilter === action
                        ? 'bg-primary-100 text-primary-700'
                        : 'text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    {action}
                  </button>
                ))}
              </div>
            </div>

            {/* Resource Filter */}
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-600">Resource:</span>
              <div className="flex gap-1">
                {resourceFilters.map((resource) => (
                  <button
                    key={resource}
                    onClick={() => {
                      setResourceFilter(resource);
                      setPage(1);
                    }}
                    className={`px-3 py-1.5 text-sm font-medium rounded-lg transition-colors ${
                      resourceFilter === resource
                        ? 'bg-primary-100 text-primary-700'
                        : 'text-gray-600 hover:bg-gray-100'
                    }`}
                  >
                    {resource}
                  </button>
                ))}
              </div>
            </div>

            {/* Date Range */}
            <div className="flex items-center gap-2">
              <input
                type="date"
                value={startDate}
                onChange={(e) => {
                  setStartDate(e.target.value);
                  setPage(1);
                }}
                className="px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
              <span className="text-gray-400">to</span>
              <input
                type="date"
                value={endDate}
                onChange={(e) => {
                  setEndDate(e.target.value);
                  setPage(1);
                }}
                className="px-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
          </div>
        </div>

        {/* Table */}
        <div className="overflow-x-auto">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
            </div>
          ) : logs.length > 0 ? (
            <table className="w-full">
              <thead>
                <tr className="text-left text-sm text-gray-500 bg-gray-50">
                  <th className="px-4 py-3 font-medium">Timestamp</th>
                  <th className="px-4 py-3 font-medium">Action</th>
                  <th className="px-4 py-3 font-medium">Resource</th>
                  <th className="px-4 py-3 font-medium">Actor</th>
                  <th className="px-4 py-3 font-medium">Details</th>
                  <th className="px-4 py-3 font-medium">IP Address</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {logs.map((log) => (
                  <tr key={log.id} className="hover:bg-gray-50">
                    <td className="px-4 py-4">
                      <div className="flex items-center gap-2 text-sm text-gray-600">
                        <Clock className="w-4 h-4 text-gray-400" />
                        {formatRelativeTime(log.created_at)}
                      </div>
                    </td>
                    <td className="px-4 py-4">
                      <div className="flex items-center gap-2">
                        {actionIcon(log.action)}
                        <span className={`px-2 py-0.5 text-xs font-medium rounded ${actionColor(log.action)}`}>
                          {log.action}
                        </span>
                      </div>
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-600">
                      {log.resource}
                      {log.resource_id && (
                        <span className="ml-1 text-gray-400">({log.resource_id.slice(0, 8)}...)</span>
                      )}
                    </td>
                    <td className="px-4 py-4">
                      <div className="flex items-center gap-2 text-sm">
                        <User className="w-4 h-4 text-gray-400" />
                        <span className="text-gray-600">{log.actor.email || log.actor.id.slice(0, 8)}</span>
                        <span className="text-xs text-gray-400">({log.actor.type})</span>
                      </div>
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-600 max-w-xs">
                      {log.changes && (
                        <button className="text-primary-600 hover:text-primary-700 font-medium">
                          View changes
                        </button>
                      )}
                    </td>
                    <td className="px-4 py-4 text-sm text-gray-500 font-mono">
                      {log.ip_address || '-'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="text-center py-12">
              <Shield className="w-12 h-12 text-gray-300 mx-auto mb-4" />
              <h3 className="text-lg font-medium text-gray-900">No audit logs</h3>
              <p className="text-gray-500 mt-1">Activity will appear here as you use the platform</p>
            </div>
          )}
        </div>

        {/* Pagination */}
        {totalLogs > 0 && (
          <div className="px-4 py-3 border-t border-gray-100 flex items-center justify-between">
            <p className="text-sm text-gray-500">
              Showing {Math.min((page - 1) * pageSize + 1, totalLogs)} to{' '}
              {Math.min(page * pageSize, totalLogs)} of {totalLogs} logs
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
