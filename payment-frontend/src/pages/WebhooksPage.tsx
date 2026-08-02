import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { 
  Plus, 
  Trash2, 
  ExternalLink, 
  RefreshCw, 
  AlertCircle, 
  CheckCircle,
  Loader2,
  Copy,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { api, ApiError } from '@/services/api';
import toast from 'react-hot-toast';

const webhookEvents = [
  'payment_intent.created',
  'payment_intent.authorized',
  'payment_intent.captured',
  'payment_intent.failed',
  'payment_intent.refunded',
  'gateway.health_changed',
  'routing.policy.activated',
  'routing.policy.deactivated',
];

export function WebhooksPage() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [newWebhookUrl, setNewWebhookUrl] = useState('');
  const [newWebhookEvents, setNewWebhookEvents] = useState<string[]>([]);
  const [createdSecret, setCreatedSecret] = useState<string | null>(null);
  
  const queryClient = useQueryClient();

  // Fetch webhooks
  const {
    data: webhooks,
    isLoading,
    error,
    refetch,
  } = useQuery({
    queryKey: ['webhooks'],
    queryFn: () => api.listWebhookEndpoints(),
    retry: 2,
  });

  // Create webhook mutation
  const createMutation = useMutation({
    mutationFn: api.createWebhookEndpoint,
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['webhooks'] });
      setCreatedSecret(data.secret);
      toast.success('Webhook created successfully');
      // Reset form
      setNewWebhookUrl('');
      setNewWebhookEvents([]);
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to create webhook');
    },
  });

  // Delete webhook mutation
  const deleteMutation = useMutation({
    mutationFn: api.deleteWebhookEndpoint,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['webhooks'] });
      toast.success('Webhook deleted');
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to delete webhook');
    },
  });

  // Test webhook mutation
  const testMutation = useMutation({
    mutationFn: api.testWebhookEndpoint,
    onSuccess: (data) => {
      if (data.success) {
        toast.success(`Test webhook sent successfully (HTTP ${data.response_code})`);
      } else {
        toast.error(`Test webhook failed (HTTP ${data.response_code})`);
      }
    },
    onError: (error) => {
      toast.error(error instanceof ApiError ? error.message : 'Failed to test webhook');
    },
  });

  const handleCreateWebhook = () => {
    if (!newWebhookUrl.trim()) {
      toast.error('Please enter a webhook URL');
      return;
    }
    if (newWebhookEvents.length === 0) {
      toast.error('Please select at least one event');
      return;
    }
    createMutation.mutate({
      url: newWebhookUrl.trim(),
      events: newWebhookEvents,
    });
  };

  const handleDeleteWebhook = (id: string) => {
    if (window.confirm('Are you sure you want to delete this webhook endpoint?')) {
      deleteMutation.mutate(id);
    }
  };

  const handleTestWebhook = (id: string) => {
    testMutation.mutate(id);
  };

  const toggleEvent = (event: string) => {
    setNewWebhookEvents((prev) => {
      if (prev.includes(event)) {
        return prev.filter((e) => e !== event);
      } else {
        return [...prev, event];
      }
    });
  };

  const copySecret = async (secret: string) => {
    await navigator.clipboard.writeText(secret);
    toast.success('Webhook secret copied to clipboard');
  };

  return (
    <div className="space-y-6">
      {/* Page Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Webhooks</h1>
          <p className="text-gray-500">Manage webhook endpoints for event notifications</p>
        </div>
        <button
          onClick={() => {
            setCreatedSecret(null);
            setShowCreateModal(true);
          }}
          className="btn-primary flex items-center gap-2"
        >
          <Plus className="w-4 h-4" />
          Add Webhook
        </button>
      </div>

      {/* Error State */}
      {error && (
        <Card className="border-danger-200 bg-danger-50">
          <div className="flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-danger-600" />
            <div className="flex-1">
              <p className="text-sm font-medium text-danger-800">Error loading webhooks</p>
              <p className="text-sm text-danger-600">
                {error instanceof ApiError ? error.message : 'Failed to load webhooks'}
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

      {/* Webhooks List */}
      <Card>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <RefreshCw className="w-6 h-6 text-gray-400 animate-spin" />
          </div>
        ) : webhooks && webhooks.length > 0 ? (
          <div className="space-y-4">
            {webhooks.map((webhook) => (
              <div
                key={webhook.id}
                className="p-4 bg-gray-50 rounded-lg"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h3 className="font-medium text-gray-900 font-mono text-sm">
                        {webhook.url}
                      </h3>
                      <span className={`px-2 py-0.5 text-xs font-medium rounded ${
                        webhook.status === 'active' 
                          ? 'bg-success-100 text-success-700' 
                          : 'bg-gray-100 text-gray-700'
                      }`}>
                        {webhook.status}
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-1 mt-2">
                      {webhook.events.map((event) => (
                        <span
                          key={event}
                          className="px-2 py-0.5 text-xs bg-primary-100 text-primary-700 rounded"
                        >
                          {event}
                        </span>
                      ))}
                    </div>
                    {webhook.last_triggered_at && (
                      <p className="text-xs text-gray-500 mt-2">
                        Last triggered: {new Date(webhook.last_triggered_at).toLocaleString()}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => handleTestWebhook(webhook.id)}
                      disabled={testMutation.isPending}
                      className="p-2 text-gray-400 hover:text-primary-600 hover:bg-primary-50 rounded-lg transition-colors"
                      title="Test webhook"
                    >
                      {testMutation.isPending ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <ExternalLink className="w-4 h-4" />
                      )}
                    </button>
                    <button
                      onClick={() => handleDeleteWebhook(webhook.id)}
                      disabled={deleteMutation.isPending}
                      className="p-2 text-gray-400 hover:text-danger-600 hover:bg-danger-50 rounded-lg transition-colors"
                      title="Delete webhook"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="text-center py-12">
            <ExternalLink className="w-12 h-12 text-gray-300 mx-auto mb-4" />
            <h3 className="text-lg font-medium text-gray-900">No webhooks yet</h3>
            <p className="text-gray-500 mt-1">Create your first webhook to receive event notifications</p>
            <button
              className="btn-primary mt-4"
              onClick={() => setShowCreateModal(true)}
            >
              <Plus className="w-4 h-4 mr-2" />
              Add Webhook
            </button>
          </div>
        )}
      </Card>

      {/* Available Events */}
      <Card>
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Available Events</h3>
        <p className="text-sm text-gray-500 mb-4">
          Subscribe to these events to receive notifications when something happens.
        </p>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {webhookEvents.map((event) => (
            <div
              key={event}
              className="flex items-center gap-3 p-3 bg-gray-50 rounded-lg"
            >
              <div className="w-2 h-2 bg-success-500 rounded-full"></div>
              <code className="font-mono text-sm text-gray-900">{event}</code>
            </div>
          ))}
        </div>
      </Card>

      {/* Create Webhook Modal */}
      {showCreateModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <Card className="w-full max-w-lg">
            <div className="p-6">
              <h2 className="text-xl font-semibold text-gray-900 mb-4">
                {createdSecret ? 'Webhook Created' : 'Create Webhook'}
              </h2>
              
              {createdSecret ? (
                // Show created webhook secret
                <div className="space-y-4">
                  <div className="p-4 bg-success-50 border border-success-200 rounded-lg">
                    <p className="text-sm text-success-700 mb-2">
                      Your webhook has been created. Copy the signing secret now - you won't be able to see it again.
                    </p>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 p-2 bg-white border border-success-200 rounded font-mono text-sm break-all">
                        {createdSecret}
                      </code>
                      <button
                        onClick={() => copySecret(createdSecret)}
                        className="p-2 text-success-600 hover:bg-success-100 rounded"
                      >
                        <Copy className="w-5 h-5" />
                      </button>
                    </div>
                  </div>
                  <button
                    className="btn-primary w-full"
                    onClick={() => {
                      setShowCreateModal(false);
                      setCreatedSecret(null);
                    }}
                  >
                    Done
                  </button>
                </div>
              ) : (
                // Create form
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Webhook URL
                    </label>
                    <input
                      type="url"
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                      placeholder="https://your-app.com/webhooks"
                      value={newWebhookUrl}
                      onChange={(e) => setNewWebhookUrl(e.target.value)}
                    />
                  </div>
                  
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-2">
                      Events
                    </label>
                    <div className="space-y-2 max-h-48 overflow-y-auto">
                      {webhookEvents.map((event) => (
                        <label
                          key={event}
                          className="flex items-center gap-3 p-2 rounded-lg hover:bg-gray-50 cursor-pointer"
                        >
                          <input
                            type="checkbox"
                            className="rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                            checked={newWebhookEvents.includes(event)}
                            onChange={() => toggleEvent(event)}
                          />
                          <code className="text-sm font-mono text-gray-700">{event}</code>
                        </label>
                      ))}
                    </div>
                  </div>
                  
                  <div className="flex gap-3 mt-6">
                    <button
                      className="btn-secondary flex-1"
                      onClick={() => setShowCreateModal(false)}
                    >
                      Cancel
                    </button>
                    <button
                      className="btn-primary flex-1 flex items-center justify-center gap-2"
                      onClick={handleCreateWebhook}
                      disabled={createMutation.isPending || !newWebhookUrl.trim()}
                    >
                      {createMutation.isPending ? (
                        <>
                          <Loader2 className="w-4 h-4 animate-spin" />
                          Creating...
                        </>
                      ) : (
                        <>
                          <CheckCircle className="w-4 h-4" />
                          Create Webhook
                        </>
                      )}
                    </button>
                  </div>
                </div>
              )}
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
