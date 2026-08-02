import { useQuery } from '@tanstack/react-query';
import {
  CheckCircle,
  AlertTriangle,
  XCircle,
  Clock,
  Activity,
  RefreshCw,
  ExternalLink,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api } from '@/services/api';
import { clsx } from 'clsx';

type ConnectorStatus = 'operational' | 'degraded' | 'outage' | 'maintenance';

interface ConnectorHealth {
  connector_id: string;
  name: string;
  status: ConnectorStatus;
  uptime_percentage: number;
  avg_latency_ms: number;
  last_incident: string | null;
  response_time_ms: number;
}

const statusConfig: Record<ConnectorStatus, { icon: typeof CheckCircle; color: string; bg: string; label: string }> = {
  operational: { icon: CheckCircle, color: 'text-success-600', bg: 'bg-success-50', label: 'Operational' },
  degraded: { icon: AlertTriangle, color: 'text-warning-600', bg: 'bg-warning-50', label: 'Degraded' },
  outage: { icon: XCircle, color: 'text-danger-600', bg: 'bg-danger-50', label: 'Outage' },
  maintenance: { icon: Clock, color: 'text-info-600', bg: 'bg-info-50', label: 'Maintenance' },
};

function StatusIndicator({ status }: { status: ConnectorStatus }) {
  const config = statusConfig[status];
  const Icon = config.icon;
  return (
    <div className={clsx('flex items-center gap-2', config.color)}>
      <Icon className="w-5 h-5" />
      <span className="font-medium">{config.label}</span>
    </div>
  );
}

function UptimeBar({ uptime }: { uptime: number }) {
  const segments = Array.from({ length: 30 }, (_, i) => {
    // Simulate uptime history
    const base = uptime / 100;
    return Math.random() < base ? 'up' : Math.random() < 0.5 ? 'degraded' : 'down';
  });

  return (
    <div className="flex gap-0.5" title={`Uptime: ${uptime}%`}>
      {segments.map((status, i) => (
        <div
          key={i}
          className={clsx(
            'w-2 h-6 rounded-sm',
            status === 'up' && 'bg-success-500',
            status === 'degraded' && 'bg-warning-500',
            status === 'down' && 'bg-danger-500'
          )}
        />
      ))}
    </div>
  );
}

export function StatusPage() {
  const { data: connectors, isLoading } = useQuery({
    queryKey: ['connector-health'],
    queryFn: () => api.getConnectorHealth(),
    refetchInterval: 30000, // Refresh every 30 seconds
    retry: 2,
  });

  const overallStatus: ConnectorStatus = connectors?.some((c: ConnectorHealth) => c.status === 'outage')
    ? 'outage'
    : connectors?.some((c: ConnectorHealth) => c.status === 'degraded')
    ? 'degraded'
    : 'operational';

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Platform Status</h1>
        <p className="text-gray-500">Real-time status of all payment connectors and platform services</p>
      </div>

      {/* Overall Status Banner */}
      <Card className={clsx(
        'border-l-4',
        overallStatus === 'operational' && 'border-l-success-500 bg-success-50',
        overallStatus === 'degraded' && 'border-l-warning-500 bg-warning-50',
        overallStatus === 'outage' && 'border-l-danger-500 bg-danger-50'
      )}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Activity className={clsx(
              'w-6 h-6',
              overallStatus === 'operational' && 'text-success-600',
              overallStatus === 'degraded' && 'text-warning-600',
              overallStatus === 'outage' && 'text-danger-600'
            )} />
            <div>
              <h2 className="text-lg font-semibold text-gray-900">
                {overallStatus === 'operational' && 'All Systems Operational'}
                {overallStatus === 'degraded' && 'Partial System Degradation'}
                {overallStatus === 'outage' && 'Major System Outage'}
              </h2>
              <p className="text-sm text-gray-500">
                Last updated: {new Date().toLocaleString()}
              </p>
            </div>
          </div>
          <button className="btn-secondary">
            <RefreshCw className="w-4 h-4 mr-2" />
            Refresh
          </button>
        </div>
      </Card>

      {/* Connector Status Grid */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-gray-900">Payment Connectors</h3>
        
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
          </div>
        ) : connectors && connectors.length > 0 ? (
          connectors.map((connector: ConnectorHealth) => {
            const config = statusConfig[connector.status];
            return (
              <Card key={connector.connector_id} className="hover:shadow-md transition-shadow">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className={clsx('p-2 rounded-lg', config.bg)}>
                      <config.icon className={clsx('w-5 h-5', config.color)} />
                    </div>
                    <div>
                      <h4 className="font-medium text-gray-900">{connector.name}</h4>
                      <div className="flex items-center gap-4 mt-1 text-sm text-gray-500">
                        <span>Uptime: {connector.uptime_percentage}%</span>
                        <span>Latency: {connector.avg_latency_ms}ms</span>
                        {connector.last_incident && (
                          <span className="text-warning-600">
                            Last incident: {new Date(connector.last_incident).toLocaleDateString()}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-6">
                    <UptimeBar uptime={connector.uptime_percentage} />
                    <StatusIndicator status={connector.status} />
                  </div>
                </div>
              </Card>
            );
          })
        ) : (
          <Card>
            <div className="text-center py-12 text-gray-500">
              <Activity className="w-12 h-12 mx-auto mb-4 text-gray-300" />
              <p className="font-medium">No connector data available</p>
              <p className="text-sm mt-1">Connect a payment provider to see its status</p>
            </div>
          </Card>
        )}
      </div>

      {/* Platform Services */}
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-gray-900">Platform Services</h3>
        <Card>
          <div className="space-y-3">
            {[
              { name: 'API Gateway', status: 'operational' as ConnectorStatus },
              { name: 'Webhook Delivery', status: 'operational' as ConnectorStatus },
              { name: 'Database', status: 'operational' as ConnectorStatus },
              { name: 'Cache (Redis)', status: 'operational' as ConnectorStatus },
              { name: 'Message Queue', status: 'operational' as ConnectorStatus },
            ].map((service) => (
              <div key={service.name} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                <span className="font-medium text-gray-900">{service.name}</span>
                <StatusIndicator status={service.status} />
              </div>
            ))}
          </div>
        </Card>
      </div>

      {/* Subscribe to Updates */}
      <Card>
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-medium text-gray-900">Subscribe to Updates</h3>
            <p className="text-sm text-gray-500">Get notified when connector status changes</p>
          </div>
          <button className="btn-secondary">
            <ExternalLink className="w-4 h-4 mr-2" />
            Subscribe
          </button>
        </div>
      </Card>
    </div>
  );
}
