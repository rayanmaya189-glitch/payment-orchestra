import axios, { AxiosInstance, AxiosError } from 'axios';
import { toast } from 'react-hot-toast';
import type {
  PaymentIntent,
  GatewayProfile,
  RoutingPolicy,
  ApiKey,
  DashboardMetrics,
  TransactionSummary,
  DailyMetric,
  GatewayPerformance,
} from '@/types';

// Custom error class for API errors
export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
    public status?: number,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

// Retry configuration
const RETRY_CONFIG = {
  maxRetries: 3,
  retryDelay: 1000,
  retryableStatuses: [408, 429, 500, 502, 503, 504],
};

class ApiClient {
  private client: AxiosInstance;
  private retryCount = new Map<string, number>();

  constructor() {
    const baseURL = import.meta.env.VITE_API_URL || '/api';
    
    this.client = axios.create({
      baseURL,
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
      },
    });

    // Add auth token from localStorage
    this.client.interceptors.request.use(
      (config) => {
        const token = localStorage.getItem('auth_token');
        if (token) {
          config.headers.Authorization = `Bearer ${token}`;
        }
        
        // Add request ID for tracing
        config.headers['X-Request-ID'] = crypto.randomUUID();
        
        // Add operator ID header if available
        const orgId = localStorage.getItem('organization_id');
        if (orgId) {
          config.headers['X-Operator-ID'] = orgId;
        }
        
        return config;
      },
      (error) => Promise.reject(error)
    );

    // Handle errors with retry logic
    this.client.interceptors.response.use(
      (response) => {
        // Reset retry count on success
        const requestId = response.config.headers['X-Request-ID'];
        if (requestId) {
          this.retryCount.delete(requestId);
        }
        return response;
      },
      async (error: AxiosError) => {
        const config = error.config;
        const requestId = config?.headers?.['X-Request-ID'] as string;
        
        // Handle 401 - Unauthorized
        if (error.response?.status === 401) {
          localStorage.removeItem('auth_token');
          localStorage.removeItem('organization_id');
          window.location.href = '/login';
          throw new ApiError('AUTH_EXPIRED', 'Session expired. Please log in again.', 401);
        }

        // Handle 403 - Forbidden
        if (error.response?.status === 403) {
          toast.error('You do not have permission to perform this action');
          throw new ApiError('FORBIDDEN', 'Insufficient permissions.', 403);
        }

        // Handle 429 - Rate Limited
        if (error.response?.status === 429) {
          const retryAfter = error.response.headers['retry-after'] || 60;
          toast.error(`Rate limited. Please try again in ${retryAfter} seconds.`);
          throw new ApiError('RATE_LIMITED', `Rate limited. Retry after ${retryAfter}s.`, 429);
        }

        // Retry logic for transient errors
        if (requestId && this.shouldRetry(error)) {
          const attempts = this.retryCount.get(requestId) || 0;
          if (attempts < RETRY_CONFIG.maxRetries) {
            this.retryCount.set(requestId, attempts + 1);
            await this.delay(RETRY_CONFIG.retryDelay * Math.pow(2, attempts));
            return this.client.request(config!);
          }
        }

        // Transform error to ApiError
        const apiError = this.transformError(error);
        throw apiError;
      }
    );
  }

  private shouldRetry(error: AxiosError): boolean {
    if (!error.response) return true; // Network error
    return RETRY_CONFIG.retryableStatuses.includes(error.response.status);
  }

  private delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  private transformError(error: AxiosError): ApiError {
    const data = error.response?.data as Record<string, unknown> | undefined;
    const code = (data?.code as string) || 'UNKNOWN_ERROR';
    const message = (data?.message as string) || error.message || 'An unexpected error occurred';
    const status = error.response?.status;
    const details = data?.details as Record<string, unknown> | undefined;

    return new ApiError(code, message, status, details);
  }

  // ─── Dashboard ────────────────────────────────────────────────────────────

  async getDashboardMetrics(): Promise<DashboardMetrics> {
    const { data } = await this.client.get('/v1/dashboard/metrics');
    return data;
  }

  async getTransactionSummary(period: string): Promise<TransactionSummary> {
    const { data } = await this.client.get(`/v1/dashboard/summary?period=${period}`);
    return data;
  }

  async getDailyMetrics(startDate: string, endDate: string): Promise<DailyMetric[]> {
    const { data } = await this.client.get(`/v1/dashboard/daily?start=${startDate}&end=${endDate}`);
    return data;
  }

  async getGatewayPerformance(): Promise<GatewayPerformance[]> {
    const { data } = await this.client.get('/v1/dashboard/gateways');
    return data;
  }

  async getRecentTransactions(limit: number = 10): Promise<PaymentIntent[]> {
    const { data } = await this.client.get(`/v1/payment-intents?limit=${limit}&sort=created_at:desc`);
    return data.items || [];
  }

  // ─── Payment Intents ──────────────────────────────────────────────────────

  async listPaymentIntents(params?: {
    status?: string;
    limit?: number;
    offset?: number;
    start_date?: string;
    end_date?: string;
    search?: string;
  }): Promise<{ items: PaymentIntent[]; total: number }> {
    const { data } = await this.client.get('/v1/payment-intents', { params });
    return data;
  }

  async getPaymentIntent(id: string): Promise<PaymentIntent> {
    const { data } = await this.client.get(`/v1/payment-intents/${id}`);
    return data;
  }

  async createPaymentIntent(params: {
    amount: number;
    currency: string;
    order_id?: string;
    metadata?: Record<string, unknown>;
  }): Promise<PaymentIntent> {
    const { data } = await this.client.post('/v1/payment-intents', params);
    return data;
  }

  async capturePaymentIntent(id: string, amount?: number): Promise<PaymentIntent> {
    const { data } = await this.client.post(`/v1/payment-intents/${id}/capture`, { amount });
    return data;
  }

  async voidPaymentIntent(id: string): Promise<PaymentIntent> {
    const { data } = await this.client.post(`/v1/payment-intents/${id}/void`);
    return data;
  }

  async refundPaymentIntent(id: string, amount?: number): Promise<PaymentIntent> {
    const { data } = await this.client.post(`/v1/payment-intents/${id}/refund`, { amount });
    return data;
  }

  // ─── Gateway Profiles ─────────────────────────────────────────────────────

  async listGatewayProfiles(): Promise<GatewayProfile[]> {
    const { data } = await this.client.get('/v1/gateway-profiles');
    return data.items || [];
  }

  async getGatewayProfile(id: string): Promise<GatewayProfile> {
    const { data } = await this.client.get(`/v1/gateway-profiles/${id}`);
    return data;
  }

  async createGatewayProfile(params: {
    connector_id: string;
    display_name: string;
    environment: 'sandbox' | 'production';
    credentials: Record<string, string>;
  }): Promise<GatewayProfile> {
    const { data } = await this.client.post('/v1/gateway-profiles', params);
    return data;
  }

  async updateGatewayProfile(id: string, params: Partial<GatewayProfile>): Promise<GatewayProfile> {
    const { data } = await this.client.patch(`/v1/gateway-profiles/${id}`, params);
    return data;
  }

  async deleteGatewayProfile(id: string): Promise<void> {
    await this.client.delete(`/v1/gateway-profiles/${id}`);
  }

  async testGatewayConnection(id: string): Promise<{ success: boolean; message: string }> {
    const { data } = await this.client.post(`/v1/gateway-profiles/${id}/test`);
    return data;
  }

  // ─── Routing Policies ─────────────────────────────────────────────────────

  async listRoutingPolicies(): Promise<RoutingPolicy[]> {
    const { data } = await this.client.get('/v1/routing-policies');
    return data.items || [];
  }

  async getRoutingPolicy(id: string): Promise<RoutingPolicy> {
    const { data } = await this.client.get(`/v1/routing-policies/${id}`);
    return data;
  }

  async createRoutingPolicy(params: {
    name: string;
    rules: RoutingPolicy['rules'];
    rotation_strategy: RoutingPolicy['rotation_strategy'];
    failover_config: RoutingPolicy['failover_config'];
  }): Promise<RoutingPolicy> {
    const { data } = await this.client.post('/v1/routing-policies', params);
    return data;
  }

  async activateRoutingPolicy(id: string): Promise<RoutingPolicy> {
    const { data } = await this.client.post(`/v1/routing-policies/${id}/activate`);
    return data;
  }

  async deactivateRoutingPolicy(id: string): Promise<RoutingPolicy> {
    const { data } = await this.client.post(`/v1/routing-policies/${id}/deactivate`);
    return data;
  }

  // ─── API Keys ─────────────────────────────────────────────────────────────

  async listApiKeys(): Promise<ApiKey[]> {
    const { data } = await this.client.get('/v1/api-keys');
    return data.items || [];
  }

  async createApiKey(params: {
    name: string;
    scopes: string[];
    environment: 'sandbox' | 'production';
    expires_at?: string;
  }): Promise<ApiKey & { key: string }> {
    const { data } = await this.client.post('/v1/api-keys', params);
    return data;
  }

  async revokeApiKey(id: string): Promise<void> {
    await this.client.delete(`/v1/api-keys/${id}`);
  }

  async rotateApiKey(id: string): Promise<ApiKey & { key: string }> {
    const { data } = await this.client.post(`/v1/api-keys/${id}/rotate`);
    return data;
  }

  // ─── Connectors ───────────────────────────────────────────────────────────

  async listConnectors(): Promise<Array<{
    id: string;
    name: string;
    description: string;
    category: string;
    supported_currencies: string[];
    supported_countries: string[];
    features: string[];
    credentials_schema: Array<{
      name: string;
      label: string;
      type: string;
      required: boolean;
      help_text?: string;
    }>;
  }>> {
    const { data } = await this.client.get('/v1/connectors');
    return data.items || [];
  }

  async getConnectorSchema(connectorId: string): Promise<{
    fields: Array<{
      name: string;
      label: string;
      type: string;
      required: boolean;
      help_text?: string;
      validation_regex?: string;
    }>;
  }> {
    const { data } = await this.client.get(`/v1/connectors/${connectorId}/schema`);
    return data;
  }

  // ─── Auth ─────────────────────────────────────────────────────────────────

  async login(email: string, password: string): Promise<{
    token: string;
    user: {
      id: string;
      email: string;
      name: string;
      role: string;
    };
    organization: {
      id: string;
      name: string;
      tier: string;
      status: string;
    };
  }> {
    const { data } = await this.client.post('/v1/auth/login', { email, password });
    return data;
  }

  async register(params: {
    email: string;
    password: string;
    name: string;
    company_name: string;
  }): Promise<{
    token: string;
    user: {
      id: string;
      email: string;
      name: string;
      role: string;
    };
    organization: {
      id: string;
      name: string;
      tier: string;
      status: string;
    };
  }> {
    const { data } = await this.client.post('/v1/auth/register', params);
    return data;
  }

  async getMe(): Promise<{
    user: {
      id: string;
      email: string;
      name: string;
      role: string;
    };
    organization: {
      id: string;
      name: string;
      tier: string;
      status: string;
    };
  }> {
    const { data } = await this.client.get('/v1/auth/me');
    return data;
  }

  // ─── Reconciliation ───────────────────────────────────────────────────────

  async listReconciliationExceptions(params?: {
    status?: string;
    limit?: number;
    offset?: number;
  }): Promise<{
    items: Array<{
      id: string;
      type: string;
      status: string;
      amount: number;
      currency: string;
      description: string;
      created_at: string;
    }>;
    total: number;
  }> {
    const { data } = await this.client.get('/v1/reconciliation/exceptions', { params });
    return data;
  }

  async resolveReconciliationException(
    id: string,
    params: { resolution: string; notes?: string }
  ): Promise<void> {
    await this.client.post(`/v1/reconciliation/exceptions/${id}/resolve`, params);
  }

  // ─── Webhooks ─────────────────────────────────────────────────────────────

  async listWebhookEndpoints(): Promise<Array<{
    id: string;
    url: string;
    events: string[];
    status: string;
    created_at: string;
    last_triggered_at?: string;
  }>> {
    const { data } = await this.client.get('/v1/webhooks');
    return data.items || [];
  }

  async createWebhookEndpoint(params: {
    url: string;
    events: string[];
  }): Promise<{
    id: string;
    url: string;
    events: string[];
    status: string;
    secret: string;
    created_at: string;
  }> {
    const { data } = await this.client.post('/v1/webhooks', params);
    return data;
  }

  async deleteWebhookEndpoint(id: string): Promise<void> {
    await this.client.delete(`/v1/webhooks/${id}`);
  }

  async testWebhookEndpoint(id: string): Promise<{ success: boolean; response_code: number }> {
    const { data } = await this.client.post(`/v1/webhooks/${id}/test`);
    return data;
  }

  // ─── Audit Logs ───────────────────────────────────────────────────────────

  async listAuditLogs(params?: {
    start_date?: string;
    end_date?: string;
    action?: string;
    resource?: string;
    limit?: number;
    offset?: number;
  }): Promise<{
    items: Array<{
      id: string;
      action: string;
      resource: string;
      resource_id?: string;
      actor: {
        id: string;
        type: string;
        email?: string;
      };
      changes?: {
        before?: Record<string, unknown>;
        after?: Record<string, unknown>;
      };
      ip_address?: string;
      created_at: string;
    }>;
    total: number;
  }> {
    const { data } = await this.client.get('/v1/audit-logs', { params });
    return data;
  }

  // ─── Analytics ────────────────────────────────────────────────────────────

  async getAnalyticsSummary(params: {
    start_date: string;
    end_date: string;
    granularity: 'hour' | 'day' | 'week' | 'month';
  }): Promise<{
    total_transactions: number;
    total_volume: number;
    success_rate: number;
    avg_latency_ms: number;
    breakdown_by_status: Record<string, number>;
    breakdown_by_gateway: Record<string, number>;
  }> {
    const { data } = await this.client.get('/v1/analytics/summary', { params });
    return data;
  }

  async getRevenueMetrics(params: {
    start_date: string;
    end_date: string;
  }): Promise<{
    total_revenue: number;
    total_fees: number;
    net_revenue: number;
    revenue_by_gateway: Array<{
      gateway_id: string;
      name: string;
      revenue: number;
      fees: number;
    }>;
  }> {
    const { data } = await this.client.get('/v1/analytics/revenue', { params });
    return data;
  }

  // ─── Settings ─────────────────────────────────────────────────────────────

  async getOrganizationSettings(): Promise<{
    name: string;
    tier: string;
    status: string;
    billing_email: string;
    timezone: string;
    default_currency: string;
  }> {
    const { data } = await this.client.get('/v1/settings/organization');
    return data;
  }

  async updateOrganizationSettings(params: {
    name?: string;
    billing_email?: string;
    timezone?: string;
    default_currency?: string;
  }): Promise<void> {
    await this.client.patch('/v1/settings/organization', params);
  }

  // ─── SaaS Billing ─────────────────────────────────────────────────────────

  async getSubscription(): Promise<{
    plan: {
      id: string;
      name: string;
      price_monthly: number;
      features: string[];
    };
    status: string;
    current_period_end: string;
    usage: {
      transactions: number;
      included: number;
      overage: number;
    };
  } | null> {
    try {
      const { data } = await this.client.get('/v1/billing/subscription');
      return data;
    } catch (error) {
      if ((error as ApiError).status === 404) {
        return null;
      }
      throw error;
    }
  }

  async listPlans(): Promise<Array<{
    id: string;
    name: string;
    slug: string;
    price_monthly: number;
    price_per_txn: number;
    included_txns: number;
    features: string[];
  }>> {
    const { data } = await this.client.get('/v1/billing/plans');
    return data.items || [];
  }

  async updateSubscription(params: {
    plan_id: string;
  }): Promise<void> {
    await this.client.post('/v1/billing/subscription', params);
  }

  async cancelSubscription(params?: {
    reason?: string;
    cancel_at_period_end?: boolean;
  }): Promise<void> {
    await this.client.delete('/v1/billing/subscription', { data: params });
  }

  // ─── MFA ─────────────────────────────────────────────────────────────────

  async setupMfa(method: 'totp' | 'sms' | 'email'): Promise<{
    secret: string;
    qr_code_url: string;
    backup_codes: string[];
  }> {
    const { data } = await this.client.post('/v1/auth/mfa/setup', { method });
    return data;
  }

  async verifyMfa(code: string): Promise<{ verified: boolean }> {
    const { data } = await this.client.post('/v1/auth/mfa/verify', { code });
    return data;
  }

  async enableMfa(): Promise<{ enabled: boolean }> {
    const { data } = await this.client.post('/v1/auth/mfa/enable');
    return data;
  }

  // ─── Connector Health ─────────────────────────────────────────────────────

  async getConnectorHealth(): Promise<Array<{
    connector_id: string;
    name: string;
    status: 'operational' | 'degraded' | 'outage' | 'maintenance';
    uptime_percentage: number;
    avg_latency_ms: number;
    last_incident: string | null;
    response_time_ms: number;
  }>> {
    const { data } = await this.client.get('/v1/connectors/health');
    return data.items || [];
  }

  // ─── Gateway Credentials ──────────────────────────────────────────────────

  async submitGatewayCredentials(
    connectorId: string,
    credentials: Record<string, string>,
    environment: string = 'sandbox'
  ): Promise<GatewayProfile> {
    const { data } = await this.client.post('/v1/gateways', {
      connector_id: connectorId,
      credentials,
      environment,
      display_name: connectorId,
    });
    return data;
  }

  // ─── Health ───────────────────────────────────────────────────────────────

  async healthCheck(): Promise<{
    status: string;
    version: string;
    uptime: number;
  }> {
    const { data } = await this.client.get('/health');
    return data;
  }
}

export const api = new ApiClient();
