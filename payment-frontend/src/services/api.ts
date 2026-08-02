import axios, { AxiosInstance, AxiosError } from 'axios';
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

class ApiClient {
  private client: AxiosInstance;

  constructor() {
    const baseURL = import.meta.env.VITE_API_URL || '/api';
    
    this.client = axios.create({
      baseURL,
      timeout: 30000,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    // Add auth token from localStorage
    this.client.interceptors.request.use((config) => {
      const token = localStorage.getItem('auth_token');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });

    // Handle errors
    this.client.interceptors.response.use(
      (response) => response,
      (error: AxiosError) => {
        if (error.response?.status === 401) {
          localStorage.removeItem('auth_token');
          window.location.href = '/login';
        }
        return Promise.reject(error);
      }
    );
  }

  // Dashboard
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

  // Payment Intents
  async listPaymentIntents(params?: {
    status?: string;
    limit?: number;
    offset?: number;
    start_date?: string;
    end_date?: string;
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

  // Gateway Profiles
  async listGatewayProfiles(): Promise<GatewayProfile[]> {
    const { data } = await this.client.get('/v1/gateway-profiles');
    return data;
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

  // Routing Policies
  async listRoutingPolicies(): Promise<RoutingPolicy[]> {
    const { data } = await this.client.get('/v1/routing-policies');
    return data;
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

  // API Keys
  async listApiKeys(): Promise<ApiKey[]> {
    const { data } = await this.client.get('/v1/api-keys');
    return data;
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

  // Connectors
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
    return data;
  }
}

export const api = new ApiClient();
